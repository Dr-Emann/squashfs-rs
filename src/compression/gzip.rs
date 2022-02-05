use crate::compression::{CodecImpl, ConfigValue};
use flate2::{FlushCompress, FlushDecompress};
use std::io;

pub type Config = repr::compression::options::Gzip;

#[derive(Debug)]
pub struct Gzip;

#[derive(Debug)]
pub struct GzipCompressor(flate2::Compress);

#[derive(Debug)]
pub struct GzipDecompressor(flate2::Decompress);

impl super::Compressor for GzipCompressor {
    fn compress(&mut self, src: &[u8], dst: &mut [u8]) -> io::Result<usize> {
        let compressor = &mut self.0;
        loop {
            let in_offset = min_mem(compressor.total_in(), src.len());
            let input = &src[in_offset..];

            let out_offset = min_mem(compressor.total_out(), dst.len());
            let output = &mut dst[out_offset..];

            let status = compressor.compress(input, output, FlushCompress::Finish)?;
            match status {
                flate2::Status::Ok => continue,
                flate2::Status::BufError => return Err(io::ErrorKind::UnexpectedEof.into()),
                flate2::Status::StreamEnd => break,
            }
        }
        Ok(compressor.total_out() as usize)
    }
}

impl super::Decompressor for GzipDecompressor {
    fn decompress(&mut self, src: &[u8], dst: &mut [u8]) -> io::Result<usize> {
        let decompressor = &mut self.0;
        loop {
            let in_offset = min_mem(decompressor.total_in(), src.len());
            let input = &src[in_offset..];

            let out_offset = min_mem(decompressor.total_out(), dst.len());
            let output = &mut dst[out_offset..];

            let status = decompressor.decompress(input, output, FlushDecompress::Finish)?;
            match status {
                flate2::Status::Ok => continue,
                flate2::Status::BufError => return Err(io::ErrorKind::UnexpectedEof.into()),
                flate2::Status::StreamEnd => break,
            }
        }
        Ok(decompressor.total_out() as usize)
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
            "window_size" => {
                let value = value.parse().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidInput, "Invalid window_size")
                })?;
                self.window_size = value;
            }
            // TODO: strategies
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
        let Self {
            compression_level,
            window_size,
            strategies,
        } = *self;
        vec![
            (
                "compression_level",
                ConfigValue::Int(compression_level.into()),
            ),
            ("window_size", ConfigValue::Int(window_size.into())),
            (
                "strategies",
                ConfigValue::String(format!("{:?}", strategies)),
            ),
        ]
    }
}

impl CodecImpl for Gzip {
    type Compressor = GzipCompressor;
    type Decompressor = GzipDecompressor;
    type Config = Config;

    fn read_config(data: &[u8]) -> io::Result<Self::Config> {
        let config: Config = repr::read(data)?;
        let compression_level = config.compression_level;
        if !(1..=9).contains(&compression_level) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid compression level ({})", compression_level),
            ));
        }
        let window_size = config.window_size;
        if !(9..=15).contains(&window_size) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid window size ({})", window_size),
            ));
        }
        Ok(config)
    }

    fn compressor(config: Self::Config) -> Self::Compressor {
        GzipCompressor(flate2::Compress::new(
            flate2::Compression::new(config.compression_level),
            true,
        ))
    }

    fn decompressor(config: Self::Config) -> Self::Decompressor {
        GzipDecompressor(flate2::Decompress::new(true))
    }
}

fn min_mem(file_size: u64, mem_size: usize) -> usize {
    if file_size < mem_size as u64 {
        file_size as usize
    } else {
        mem_size
    }
}
