use flate2::{FlushCompress, FlushDecompress};
use std::io;

pub type Config = repr::compression::options::Gzip;

#[derive(Debug)]
pub struct Gzip {
    config: Config,
    decompressor: flate2::Decompress,
    compressor: flate2::Compress,
}

impl Default for Gzip {
    fn default() -> Self {
        Self::with_config(Config::default())
    }
}

impl Gzip {
    fn with_config(config: Config) -> Self {
        let level = flate2::Compression::new(config.compression_level);
        Self {
            config,
            decompressor: flate2::Decompress::new(true),
            compressor: flate2::Compress::new(level, true),
        }
    }

    fn decompressor(&mut self) -> &mut flate2::Decompress {
        let decompressor = &mut self.decompressor;
        decompressor.reset(true);
        decompressor
    }

    fn compressor(&mut self) -> &mut flate2::Compress {
        let compressor = &mut self.compressor;
        compressor.reset();
        compressor
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn configured(options: &[u8]) -> io::Result<Self> {
        let config: Config = repr::read(options)?;
        let compression_level = config.compression_level;
        if compression_level < 1 || compression_level > 9 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid compression level ({})", compression_level),
            ));
        }
        let window_size = config.window_size;
        if window_size < 9 || window_size > 15 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid window size ({})", window_size),
            ));
        }
        let level = flate2::Compression::new(compression_level);
        Ok(Self {
            config,
            compressor: flate2::Compress::new(level, true),
            decompressor: flate2::Decompress::new(true),
        })
    }

    pub fn compress(&mut self, src: &[u8], dst: &mut [u8]) -> io::Result<usize> {
        let compressor = self.compressor();
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

    pub fn decompress(&mut self, src: &[u8], dst: &mut [u8]) -> io::Result<usize> {
        let decompressor = self.decompressor();
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

    pub fn config(&self) -> Config {
        self.config
    }
}

fn min_mem(file_size: u64, mem_size: usize) -> usize {
    if file_size < mem_size as u64 {
        file_size as usize
    } else {
        mem_size
    }
}
