use super::pool;
use crate::compression::Compressor;
use crate::thread;
use futures::channel::oneshot;
use futures::FutureExt;
use std::future::Future;
use std::{fmt, io, mem};

pub struct ParallelCompressor {
    // Destructors are run in top-down order, so this closes the sender before joining
    sender: crossbeam_channel::Sender<Request>,
    threads: crate::thread::Joiner<()>,
}

enum RequestType {
    Compress {
        with_compressed: Box<dyn FnOnce(CompressResult) + Send + 'static>,
    },
    Decompress {
        with_decompressed: Box<dyn FnOnce(io::Result<pool::Block<'static>>) + Send + 'static>,
        max_size: usize,
    },
}

impl fmt::Debug for RequestType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestType::Compress { .. } => f.debug_struct("Compress").finish(),
            RequestType::Decompress { max_size, .. } => f
                .debug_struct("Decompress")
                .field("max_size", max_size)
                .finish(),
        }
    }
}

#[derive(Debug)]
pub struct CompressResult {
    pub data: pool::Block<'static>,
    pub compressed: bool,
}

struct Request {
    data: Vec<u8>,
    request_type: RequestType,
}

impl ParallelCompressor {
    pub fn new(compressor: Compressor) -> Self {
        Self::with_threads(compressor, num_cpus::get())
    }

    pub fn with_threads(compressor: Compressor, threads: usize) -> Self {
        assert!(threads > 0);

        let (tx, rx) = crossbeam_channel::bounded(0);
        let threads = thread::Joiner::new(threads, || thread_fn(rx.clone(), compressor.clone()));

        Self {
            threads,
            sender: tx,
        }
    }

    pub fn compress<F: FnOnce(CompressResult) + Send + 'static>(&self, data: Vec<u8>, f: F) {
        let request = Request {
            data,
            request_type: RequestType::Compress {
                with_compressed: Box::new(f),
            },
        };

        self.sender.send(request).unwrap();
    }

    pub fn decompress<F: FnOnce(io::Result<pool::Block<'static>>) + Send + 'static>(
        &self,
        data: Vec<u8>,
        max_size: usize,
        f: F,
    ) {
        let request = Request {
            data,
            request_type: RequestType::Decompress {
                with_decompressed: Box::new(f),
                max_size,
            },
        };

        self.sender.send(request).unwrap();
    }

    pub fn compress_fut(&self, data: Vec<u8>) -> impl Future<Output = CompressResult> {
        let (tx, rx) = oneshot::channel();
        self.compress(data, move |compress_result| {
            // Ignore closed receiver
            let _ = tx.send(compress_result);
        });
        rx.map(Result::unwrap)
    }

    pub fn decompress_fut(
        &self,
        max_size: usize,
        data: Vec<u8>,
    ) -> impl Future<Output = io::Result<pool::Block<'static>>> {
        let (tx, rx) = oneshot::channel();
        self.decompress(data, max_size, move |maybe_block| {
            // Ignore closed receiver
            let _ = tx.send(maybe_block);
        });
        rx.map(Result::unwrap)
    }
}

fn thread_fn(
    rx: crossbeam_channel::Receiver<Request>,
    mut compressor: Compressor,
) -> impl FnOnce() {
    move || {
        for mut request in rx {
            let mut src = pool::attach_block(mem::take(&mut request.data));
            let mut dst = pool::block();
            match request.request_type {
                RequestType::Compress { with_compressed } => {
                    let compressed: bool;
                    // TODO: Profile if this should use unsafe set_len
                    // Set to 1 smaller, so compressing to an equal sized result will just be left uncompressed
                    dst.resize(src.len() - 1, 0);
                    match compressor.compress(&src, &mut dst) {
                        Ok(n) => {
                            dst.truncate(n);
                            compressed = true;
                        }
                        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                            // dst should get request data, and we'll return the invalid response data to the pool (in src)
                            mem::swap(&mut src, &mut dst);
                            compressed = false;
                        }
                        Err(e) => {
                            panic!("compressor should not be able to have an error compressing")
                        }
                    }
                    with_compressed(CompressResult {
                        data: dst,
                        compressed,
                    });
                }
                RequestType::Decompress {
                    with_decompressed,
                    max_size,
                } => {
                    dst.resize(max_size, 0);
                    let maybe_result = compressor.decompress(&src, &mut dst).map(|n| {
                        dst.truncate(n);
                        dst
                    });
                    with_decompressed(maybe_result);
                }
            };
        }
    }
}

impl fmt::Debug for ParallelCompressor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParallelCompressor").finish_non_exhaustive()
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

            let compressor =
                ParallelCompressor::with_threads(Compressor::new(compression::Kind::ZLib), 2);
            let response1 = compressor.compress_fut(duplicate_data.clone());
            let response2 = compressor.compress_fut(uncompressible.clone());

            let (response1, response2) = futures::join!(response1, response2);

            assert!(response1.compressed);
            assert!(response1.data.len() < duplicate_data.len());
            assert!(!response2.compressed);
            assert_eq!(&*response2.data, &uncompressible);
        });
    }
}
