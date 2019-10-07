use parking_lot::Mutex;
use positioned_io::RandomAccessFile;
use positioned_io::WriteAt;
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

impl<W: SharedWriteAt> SharedWriteAt for &W {
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

pub struct PositionedWriter<W> {
    writer: W,
    position: u64,
}

impl<W: SharedWriteAt> PositionedWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            position: 0,
        }
    }

    pub fn with_position(writer: W, position: u64) -> Self {
        Self { writer, position }
    }
}

impl<W: SharedWriteAt> io::Write for PositionedWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let position = self.position;
        let res = self.writer.write_at(buf, position)?;
        self.position += res as u64;
        Ok(res)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        let position = self.position;
        self.writer.write_all_at(buf, position)?;
        self.position += buf.len() as u64;
        Ok(())
    }
}

impl<W: WriteAt + Sync + Send> SharedWriteAt for Mutex<W> {
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
