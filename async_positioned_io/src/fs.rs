use crate::AsyncReadAt;
use std::fs::File as StdFile;
use std::future::Future;
use std::mem;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::{cmp, io};
use tokio::task::JoinHandle;

pub struct File {
    std: Arc<StdFile>,
    state: State,
}

impl File {
    pub fn new(f: StdFile) -> Self {
        Self {
            std: Arc::new(f),
            state: State::Idle(Vec::new()),
        }
    }
}

enum State {
    Idle(Vec<u8>),
    Busy(JoinHandle<(Operation, Vec<u8>)>),
}

enum Operation {
    Read(io::Result<usize>),
    Write(io::Result<()>),
}

impl AsyncReadAt for File {
    fn poll_read_at(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        dst: &mut [u8],
        pos: u64,
    ) -> Poll<std::io::Result<usize>> {
        let me = self.get_mut();

        loop {
            match me.state {
                State::Idle(ref mut buf) => {
                    let mut buf = mem::take(buf);
                    buf.clear();
                    buf.reserve(dst.len());
                    unsafe { buf.set_len(dst.len()) };
                    let std = Arc::clone(&me.std);

                    me.state = State::Busy(tokio::task::spawn_blocking(move || {
                        let res = file_read_at(&std, &mut buf, pos);
                        if let Ok(size) = res {
                            buf.truncate(size);
                        }
                        (Operation::Read(res), buf)
                    }));
                }
                State::Busy(ref mut rx) => {
                    let (op, mut buf) = match Pin::new(rx).poll(cx) {
                        Poll::Ready(x) => x,
                        Poll::Pending => return Poll::Pending,
                    }?;

                    match op {
                        Operation::Read(Ok(size)) => {
                            let size = cmp::min(size, buf.len());
                            dst.copy_from_slice(&buf[..size]);
                            me.state = State::Idle(buf);
                            return Poll::Ready(Ok(size));
                        }
                        Operation::Read(Err(e)) => {
                            assert!(buf.is_empty());

                            me.state = State::Idle(buf);
                            return Poll::Ready(Err(e));
                        }
                        Operation::Write(Ok(_)) => {
                            assert!(buf.is_empty());
                            me.state = State::Idle(buf);
                            continue;
                        }
                        Operation::Write(Err(e)) => {
                            todo!();
                            // assert!(inner.last_write_err.is_none());
                            // inner.last_write_err = Some(e.kind());
                            // inner.state = Idle(Some(buf));
                        }
                    }
                }
            }
        }
    }
}

fn file_read_at(f: &StdFile, buf: &mut [u8], pos: u64) -> io::Result<usize> {
    #[cfg(unix)]
    return std::os::unix::fs::FileExt::read_at(f, buf, pos);
    #[cfg(windows)]
    return std::os::windows::fs::FileExt::seek_read(f, buf, pos);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AsyncReadAtExt;
    use std::io::Write;

    #[tokio::test]
    async fn file_read_at() {
        let std = tempfile::tempfile().unwrap();
        writeln!(&std, "1234567890");
        let file = File::new(std);
        let mut buf = [0; 5];
        let n = file.read_at(&mut buf, 1).await.unwrap();
    }
}
