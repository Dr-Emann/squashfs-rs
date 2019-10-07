#![allow(unused_variables, dead_code)]

pub mod config;
pub mod shared_position_file;
pub mod write;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CompressionType {
    Gzip = 1,
    Lzma,
    Lzo,
    Xz,
    Lz4,
    Zstd,
}

impl Default for CompressionType {
    fn default() -> Self {
        CompressionType::Gzip
    }
}
