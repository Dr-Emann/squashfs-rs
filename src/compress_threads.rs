use super::pool;
use crate::compression::Compressor;
use crossbeam_channel as channel;
use futures::channel::oneshot;
use futures::FutureExt;
use std::future::Future;
use std::{io, mem, thread};

pub struct ParallelCompressor {
    // Destructors are run in top-down order, so this closes the sender before joining
    sender: channel::Sender<Request>,
    threads: ThreadJoiner,
}

#[derive(Debug, Copy, Clone)]
enum RequestType {
    Compress,
    Decompress { max_size: usize },
}

struct Request {
    data: Vec<u8>,
    request_type: RequestType,
    reply: oneshot::Sender<io::Result<Response>>,
}

pub struct Response {
    pub data: pool::Block<'static>,
    pub compressed: bool,
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
    pub fn new(threads: usize, compressor: Compressor) -> Self {
        assert!(threads > 0);

        let (tx, rx) = channel::bounded(0);
        let mut thread_handles = Vec::with_capacity(threads);
        for _ in 0..threads - 1 {
            thread_handles.push(std::thread::spawn(thread_fn(
                rx.clone(),
                compressor.clone(),
            )));
        }
        thread_handles.push(std::thread::spawn(thread_fn(rx, compressor)));

        Self {
            threads: ThreadJoiner(thread_handles),
            sender: tx,
        }
    }

    pub fn compress(&self, data: Vec<u8>) -> impl Future<Output = io::Result<Response>> {
        let (tx, rx) = oneshot::channel();
        let request = Request {
            data,
            request_type: RequestType::Compress,
            reply: tx,
        };

        self.sender.send(request).unwrap();

        rx.map(Result::unwrap)
    }

    pub fn decompress(
        &self,
        data: Vec<u8>,
        max_size: usize,
    ) -> impl Future<Output = io::Result<Response>> {
        let (tx, rx) = oneshot::channel();
        let request = Request {
            data,
            request_type: RequestType::Decompress { max_size },
            reply: tx,
        };

        self.sender.send(request).unwrap();

        rx.map(Result::unwrap)
    }
}

fn thread_fn(rx: channel::Receiver<Request>, mut compressor: Compressor) -> impl FnOnce() -> () {
    move || {
        for mut request in rx {
            let mut src = pool::attach_block(mem::take(&mut request.data));
            let mut response = Response {
                data: pool::block(),
                compressed: false,
            };
            let response: io::Result<Response> = match request.request_type {
                RequestType::Compress => {
                    // TODO: Profile if this should use unsafe set_len
                    // Set to 1 smaller, so compressing to an equal sized result will just be left uncompressed
                    response.data.resize(src.len() - 1, 0);
                    match compressor.compress(&src, &mut response.data) {
                        Ok(n) => {
                            response.data.truncate(n);
                            response.compressed = true;
                            Ok(response)
                        }
                        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                            // result should get request data, and we'll return the invalid response data to the pool
                            mem::swap(&mut src, &mut response.data);
                            response.compressed = false;
                            Ok(response)
                        }
                        Err(e) => Err(e),
                    }
                }
                RequestType::Decompress { max_size } => {
                    response.data.resize(max_size, 0);
                    compressor.decompress(&src, &mut response.data).map(|n| {
                        response.data.truncate(n);
                        response
                    })
                }
            };
            let _ = request.reply.send(response);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compression::{self, Compressor};

    #[test]
    fn multiple_requests() {
        futures::executor::block_on(async {
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
            let response2 = compressor.compress(uncompressible.clone());

            let (response1, response2) =
                futures::join!(response1.map(Result::unwrap), response2.map(Result::unwrap));

            assert!(response1.compressed);
            assert!(response1.data.len() < duplicate_data.len());
            assert!(!response2.compressed);
            assert_eq!(&*response2.data, &uncompressible);
        });
    }
}
