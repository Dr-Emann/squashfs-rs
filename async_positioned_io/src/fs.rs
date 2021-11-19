use crate::{AsyncReadAt, AsyncWriteAt, BufResult};
use std::fs::File as StdFile;
use std::io;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct File {
    std: Arc<StdFile>,
}

impl File {
    pub fn new(f: StdFile) -> Self {
        Self { std: Arc::new(f) }
    }
}

#[async_trait::async_trait]
impl AsyncReadAt for File {
    async fn read_at(&self, mut buf: Vec<u8>, pos: u64) -> BufResult<usize> {
        let std = Arc::clone(&self.std);
        tokio::task::spawn_blocking(move || {
            let res = file_read_at(&std, &mut buf, pos);
            res.map(|i| (i, buf))
        })
        .await?
    }

    async fn read_exact_at(&self, mut buf: Vec<u8>, pos: u64) -> BufResult<()> {
        let std = Arc::clone(&self.std);
        tokio::task::spawn_blocking(move || {
            let res = file_read_exact_at(&std, &mut buf, pos);
            res.map(|i| (i, buf))
        })
        .await?
    }
}

#[async_trait::async_trait]
impl AsyncWriteAt for File {
    async fn write_at(&self, buf: Vec<u8>, pos: u64) -> BufResult<usize> {
        let std = Arc::clone(&self.std);
        tokio::task::spawn_blocking(move || {
            let res = file_write_at(&std, &buf, pos);
            res.map(|i| (i, buf))
        })
        .await?
    }

    async fn write_all_at(&self, buf: Vec<u8>, pos: u64) -> BufResult<()> {
        let std = Arc::clone(&self.std);
        tokio::task::spawn_blocking(move || {
            let res = file_write_all_at(&std, &buf, pos);
            res.map(|i| (i, buf))
        })
        .await?
    }
}

fn file_read_at(f: &StdFile, buf: &mut [u8], pos: u64) -> io::Result<usize> {
    #[cfg(unix)]
    return std::os::unix::fs::FileExt::read_at(f, buf, pos);
    #[cfg(windows)]
    return std::os::windows::fs::FileExt::seek_read(f, buf, pos);
}

fn file_read_exact_at(f: &StdFile, mut buf: &mut [u8], mut pos: u64) -> io::Result<()> {
    while !buf.is_empty() {
        match file_read_at(f, buf, pos) {
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(n) => {
                buf = &mut buf[n..];
                pos += n as u64;
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

fn file_write_at(f: &StdFile, buf: &[u8], pos: u64) -> io::Result<usize> {
    #[cfg(unix)]
    return std::os::unix::fs::FileExt::write_at(f, buf, pos);
    #[cfg(windows)]
    return std::os::windows::fs::FileExt::seek_write(f, buf, pos);
}

fn file_write_all_at(f: &StdFile, mut buf: &[u8], mut pos: u64) -> io::Result<()> {
    while !buf.is_empty() {
        match file_write_at(f, buf, pos) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(n) => {
                buf = &buf[n..];
                pos += n as u64;
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AsyncReadAt;
    use std::io::Write;

    #[tokio::test]
    async fn file_read_at() {
        let std = tempfile::tempfile().unwrap();
        writeln!(&std, "1234567890").unwrap();
        let file = File::new(std);
        let buf = vec![0; 5];
        let (_, buf) = file.read_exact_at(buf, 1).await.unwrap();
        assert_eq!(&buf[..], "23456".as_bytes());
    }
}
