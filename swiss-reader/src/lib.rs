#[cfg(not(unix))]
mod default;
#[cfg(unix)]
mod unix;

use std::io;
use std::io::IoSliceMut;

pub trait SparseRead: io::Read {
    /// Seek past a possible hole at the current position
    ///
    /// Attempts to seek past the current hole, if the current position is at a
    /// hole. Returns the number of bytes skipped, if any.
    ///
    /// It is a valid (and the default) implementation to always return `Ok(0)`:
    /// this means "no holes"
    fn skip_hole(&mut self) -> io::Result<u64> {
        Ok(0)
    }
}

// Use default implementation
impl SparseRead for &[u8] {}
impl SparseRead for io::Empty {}
impl<T> SparseRead for io::Cursor<T> where T: AsRef<[u8]> {}

impl<R> SparseRead for &mut R
where
    R: SparseRead,
{
    fn skip_hole(&mut self) -> io::Result<u64> {
        (**self).skip_hole()
    }
}
impl<R> SparseRead for Box<R>
where
    R: SparseRead,
{
    fn skip_hole(&mut self) -> io::Result<u64> {
        (**self).skip_hole()
    }
}

pub struct NoHoles<R> {
    inner: R,
}

impl<R> NoHoles<R> {
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R> io::Read for NoHoles<R>
where
    R: io::Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.inner.read_vectored(bufs)
    }
}

// Default impl is correct for NoHoles
impl<R> SparseRead for NoHoles<R> where R: io::Read {}
