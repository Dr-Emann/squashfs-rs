use parking_lot::Mutex;
use positioned_io::{RandomAccessFile, ReadAt, WriteAt};
use std::io;

pub trait SharedWriteAt: Send + Sync {
    fn write_at(&self, buf: &[u8], pos: u64) -> io::Result<usize>;
    fn write_all_at(&self, buf: &[u8], pos: u64) -> io::Result<()>;
    fn flush(&self) -> io::Result<()>;
}

impl SharedWriteAt for RandomAccessFile {
    fn write_at(&self, buf: &[u8], pos: u64) -> io::Result<usize> {
        positioned_io::WriteAt::write_at(&mut &*self, pos, buf)
    }

    fn write_all_at(&self, buf: &[u8], pos: u64) -> io::Result<()> {
        positioned_io::WriteAt::write_all_at(&mut &*self, pos, buf)
    }

    fn flush(&self) -> io::Result<()> {
        positioned_io::WriteAt::flush(&mut &*self)
    }
}

impl<W: SharedWriteAt + ?Sized> SharedWriteAt for &W {
    fn write_at(&self, buf: &[u8], pos: u64) -> io::Result<usize> {
        SharedWriteAt::write_at(*self, buf, pos)
    }

    fn write_all_at(&self, buf: &[u8], pos: u64) -> io::Result<()> {
        SharedWriteAt::write_all_at(*self, buf, pos)
    }

    fn flush(&self) -> io::Result<()> {
        SharedWriteAt::flush(*self)
    }
}

impl<W: WriteAt + Sync + Send + ?Sized> SharedWriteAt for Mutex<W> {
    fn write_at(&self, buf: &[u8], pos: u64) -> io::Result<usize> {
        self.lock().write_at(pos, buf)
    }

    fn write_all_at(&self, buf: &[u8], pos: u64) -> io::Result<()> {
        self.lock().write_all_at(pos, buf)
    }

    fn flush(&self) -> io::Result<()> {
        self.lock().flush()
    }
}

pub struct Positioned<F> {
    file: F,
    position: u64,
}

impl<W> Positioned<W> {
    pub fn new(file: W) -> Self {
        Self { file, position: 0 }
    }

    pub fn with_position(file: W, position: u64) -> Self {
        Self { file, position }
    }
}

impl<W: SharedWriteAt> io::Write for Positioned<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let position = self.position;
        let res = self.file.write_at(buf, position)?;
        self.position += res as u64;
        Ok(res)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        let position = self.position;
        self.file.write_all_at(buf, position)?;
        self.position += buf.len() as u64;
        Ok(())
    }
}

impl<R: ReadAt> io::Read for Positioned<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let position = self.position;
        let res = self.file.read_at(position, buf)?;
        self.position += res as u64;
        Ok(res)
    }
}
