mod fs;

use std::future::Future;
use std::io;
use std::ops::DerefMut;
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait AsyncReadAt {
    fn poll_read_at(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
        pos: u64,
    ) -> Poll<io::Result<usize>>;
}

pub trait AsyncReadAtExt: AsyncReadAt {
    fn read_at<'a>(&'a mut self, buf: &'a mut [u8], pos: u64) -> ReadAt<'a, Self>
    where
        Self: Unpin,
    {
        ReadAt::new(self, buf, pos)
    }
}

impl<R: ?Sized + AsyncReadAt> AsyncReadAtExt for R {}

/// Future for the [`read`](super::AsyncReadExt::read) method.
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct ReadAt<'a, R: ?Sized> {
    reader: &'a mut R,
    buf: &'a mut [u8],
    pos: u64,
}

impl<R: ?Sized + Unpin> Unpin for ReadAt<'_, R> {}

macro_rules! deref_async_read_at {
    () => {
        fn poll_read_at(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
            pos: u64,
        ) -> Poll<io::Result<usize>> {
            Pin::new(&mut **self).poll_read_at(cx, buf, pos)
        }
    };
}

impl<T: ?Sized + AsyncReadAt + Unpin> AsyncReadAt for Box<T> {
    deref_async_read_at!();
}

impl<T: ?Sized + AsyncReadAt + Unpin> AsyncReadAt for &mut T {
    deref_async_read_at!();
}

impl<P> AsyncReadAt for Pin<P>
where
    P: DerefMut + Unpin,
    P::Target: AsyncReadAt,
{
    fn poll_read_at(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
        pos: u64,
    ) -> Poll<io::Result<usize>> {
        self.get_mut().as_mut().poll_read_at(cx, buf, pos)
    }
}

impl<'a, R: AsyncReadAt + ?Sized + Unpin> ReadAt<'a, R> {
    fn new(reader: &'a mut R, buf: &'a mut [u8], pos: u64) -> Self {
        Self { reader, buf, pos }
    }
}

impl<R: AsyncReadAt + ?Sized + Unpin> Future for ReadAt<'_, R> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        Pin::new(&mut this.reader).poll_read_at(cx, this.buf, this.pos)
    }
}
