use crate::compression::Codec;
use std::{fmt, io};
use zstd::bulk::{Compressor, Decompressor};

pub type Config = repr::compression::options::Zstd;

pub struct Zstd {
    config: Config,
    decompressor: Decompressor<'static>,
    compressor: Compressor<'static>,
}

impl fmt::Debug for Zstd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Zstd")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl Default for Zstd {
    fn default() -> Self {
        Self::with_config(Config::default())
    }
}

impl Clone for Zstd {
    fn clone(&self) -> Self {
        Self::with_config(self.config)
    }
}

impl Codec for Zstd {
    type Config = Config;

    fn with_config(config: Config) -> Self
    where
        Self: Sized,
    {
        Self {
            config,
            decompressor: Decompressor::new().unwrap(),
            compressor: Compressor::new(config.compression_level as _).unwrap(),
        }
    }

    fn configured(options: &[u8]) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let config: Config = repr::read(options)?;
        let compression_level = config.compression_level;
        if !(1..=22).contains(&compression_level) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid compression level ({})", compression_level),
            ));
        }
        Ok(Self::with_config(config))
    }

    fn compress(&mut self, src: &[u8], dst: &mut [u8]) -> std::io::Result<usize> {
        self.compressor.compress_to_buffer(src, dst)
    }

    fn decompress(&mut self, src: &[u8], dst: &mut [u8]) -> std::io::Result<usize> {
        self.decompressor.decompress_to_buffer(src, dst)
    }

    fn config(&self) -> &Config {
        &self.config
    }
}
