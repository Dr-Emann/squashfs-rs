use crossbeam_channel as channel;
use std::io;
use std::mem;
use std::thread;

pub struct ParallelCompressor {
    buffers: (channel::Sender<Vec<u8>>, channel::Receiver<Vec<u8>>),
    // Destructors are run in top-down order, so this closes the sender before joining
    sender: channel::Sender<Request>,
    threads: ThreadJoiner,
}

enum RequestType {
    Compress,
    Decompress { max_size: usize },
}

struct Request {
    data: Vec<u8>,
    request_type: RequestType,
    reply: channel::Sender<io::Result<Response>>,
}

pub struct Response {
    pub data: Vec<u8>,
    pub compressed: bool,
    data_return: channel::Sender<Vec<u8>>,
}

impl Drop for Response {
    fn drop(&mut self) {
        let data = mem::take(&mut self.data);
        let _ = self.data_return.try_send(data);
    }
}

struct ThreadJoiner(Vec<thread::JoinHandle<()>>);

impl Drop for ThreadJoiner {
    fn drop(&mut self) {
        for t in self.0.drain(..) {
            t.join().unwrap();
        }
    }
}

impl ParallelCompressor {
    pub fn new(threads: usize, compressor: super::Compressor) -> Self {
        assert!(threads > 0);

        let (tx, rx) = channel::bounded(0);
        let buffers = channel::bounded(threads);
        let mut thread_handles = Vec::with_capacity(threads);
        for _ in 0..threads - 1 {
            thread_handles.push(std::thread::spawn(thread_fn(
                rx.clone(),
                compressor.clone(),
                buffers.clone(),
            )));
        }
        thread_handles.push(std::thread::spawn(thread_fn(
            rx,
            compressor,
            buffers.clone(),
        )));

        Self {
            buffers,
            threads: ThreadJoiner(thread_handles),
            sender: tx,
        }
    }

    pub fn response(&self, data: Vec<u8>, compressed: bool) -> Response {
        Response {
            data,
            compressed,
            data_return: self.buffers.0.clone(),
        }
    }

    pub fn compress(&self, data: Vec<u8>) -> channel::Receiver<io::Result<Response>> {
        let (tx, rx) = channel::bounded(1);
        let request = Request {
            data,
            request_type: RequestType::Compress,
            reply: tx,
        };

        self.sender.send(request).unwrap();

        rx
    }

    pub fn decompress(
        &self,
        data: Vec<u8>,
        max_size: usize,
    ) -> channel::Receiver<io::Result<Response>> {
        let (tx, rx) = channel::bounded(1);
        let request = Request {
            data,
            request_type: RequestType::Decompress { max_size },
            reply: tx,
        };

        self.sender.send(request).unwrap();

        rx
    }
}

fn thread_fn(
    rx: channel::Receiver<Request>,
    mut compressor: super::Compressor,
    (buffer_tx, buffer_rx): (channel::Sender<Vec<u8>>, channel::Receiver<Vec<u8>>),
) -> impl FnOnce() -> () {
    move || {
        for mut request in rx {
            let mut response = Response {
                data: buffer_rx.try_recv().unwrap_or_default(),
                compressed: false,
                data_return: buffer_tx.clone(),
            };
            match request.request_type {
                RequestType::Compress => {
                    // TODO: Profile if this should use unsafe set_len
                    // Set to 1 smaller, so compressing to an equal sized result will just be left uncompressed
                    response.data.resize(request.data.len() - 1, 0);
                    let response = match compressor.compress(&request.data, &mut response.data) {
                        Ok(n) => {
                            response.data.truncate(n);
                            response.compressed = true;
                            Ok(response)
                        }
                        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                            // result should get request data, and we'll return the invalid response data to the pool
                            mem::swap(&mut request.data, &mut response.data);
                            response.compressed = false;
                            Ok(response)
                        }
                        Err(e) => Err(e),
                    };
                    let _ = request.reply.try_send(response);
                }
                RequestType::Decompress { max_size } => {
                    response.data.resize(max_size, 0);
                    let response = compressor
                        .decompress(&request.data, &mut response.data)
                        .map(|n| {
                            response.data.truncate(n);
                            response
                        });
                    let _ = request.reply.try_send(response);
                }
            }
            let _ = buffer_tx.try_send(mem::take(&mut request.data));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compression::{self, Compressor};
    use std::time::Duration;

    #[test]
    fn multiple_requests() {
        let duplicate_data: Vec<u8> = "hi there you all"
            .as_bytes()
            .iter()
            .copied()
            .cycle()
            .take(4 * 1024)
            .collect();

        let uncompressible = vec![1];

        let compressor = ParallelCompressor::new(2, Compressor::new(compression::Kind::ZLib));
        let response1 = compressor.compress(duplicate_data.clone());
        assert!(response1.try_recv().is_err());

        let response2 = compressor.compress(uncompressible.clone());
        assert!(response1.try_recv().is_err());
        assert!(response2.try_recv().is_err());

        let response2 = response2
            .recv_timeout(Duration::from_millis(500))
            .unwrap()
            .unwrap();
        let response1 = response1
            .recv_timeout(Duration::from_millis(500))
            .unwrap()
            .unwrap();
        assert!(response1.compressed);
        assert!(response1.data.len() < duplicate_data.len());
        assert!(!response2.compressed);
        assert_eq!(response2.data, uncompressible);
    }
}
