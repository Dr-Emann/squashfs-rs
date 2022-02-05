use crate::compression::{CodecImpl, ConfigValue};
use std::fmt::Formatter;
use std::{fmt, io};
use zstd::bulk as zbulk;

pub type Config = repr::compression::options::Zstd;

#[derive(Debug)]
pub struct Zstd;

pub struct ZstdCompressor(zbulk::Compressor<'static>);

pub struct ZstdDecompressor(zbulk::Decompressor<'static>);

impl super::Compressor for ZstdCompressor {
    fn compress(&mut self, src: &[u8], dst: &mut [u8]) -> io::Result<usize> {
        self.0.compress_to_buffer(src, dst)
    }
}

impl super::Decompressor for ZstdDecompressor {
    fn decompress(&mut self, src: &[u8], dst: &mut [u8]) -> io::Result<usize> {
        self.0.decompress_to_buffer(src, dst)
    }
}

impl super::Config for Config {
    fn set(&mut self, field: &str, value: &str) -> io::Result<()> {
        match field {
            "compression_level" => {
                let value = value.parse().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidInput, "Invalid compression_level")
                })?;
                self.compression_level = value;
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Unknown field {field}"),
                ));
            }
        }
        Ok(())
    }

    fn key_values(&self) -> Vec<(&'static str, ConfigValue<'_>)> {
        vec![(
            "compression_level",
            ConfigValue::Int(self.compression_level.into()),
        )]
    }
}

impl CodecImpl for Zstd {
    type Compressor = ZstdCompressor;
    type Decompressor = ZstdDecompressor;
    type Config = Config;

    fn read_config(data: &[u8]) -> io::Result<Self::Config> {
        let config: Config = repr::read(data)?;
        let compression_level = config.compression_level;
        if !(1..=22).contains(&compression_level) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid compression level ({})", compression_level),
            ));
        }
        Ok(config)
    }

    fn compressor(config: Self::Config) -> Self::Compressor {
        ZstdCompressor(zbulk::Compressor::new(config.compression_level as _).unwrap())
    }

    fn decompressor(config: Self::Config) -> Self::Decompressor {
        ZstdDecompressor(zbulk::Decompressor::new().unwrap())
    }
}

impl fmt::Debug for ZstdCompressor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ZstdCompressor").finish()
    }
}

impl fmt::Debug for ZstdDecompressor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ZstdDecompressor").finish()
    }
}
