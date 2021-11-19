pub mod fs;

use std::io;

pub type BufResult<T> = io::Result<(T, Vec<u8>)>;

#[async_trait::async_trait]
pub trait AsyncReadAt {
    async fn read_at(&self, buf: Vec<u8>, pos: u64) -> BufResult<usize>;
    async fn read_exact_at(&self, buf: Vec<u8>, pos: u64) -> BufResult<()>;
}

#[async_trait::async_trait]
pub trait AsyncWriteAt {
    async fn write_at(&self, buf: Vec<u8>, pos: u64) -> BufResult<usize>;
    async fn write_all_at(&self, buf: Vec<u8>, pos: u64) -> BufResult<()>;
}
