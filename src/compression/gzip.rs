use flate2::{FlushCompress, FlushDecompress};
use std::cell::{RefCell, RefMut};
use std::io;

pub type Config = repr::compression::options::Gzip;

#[derive(Debug)]
struct State {
    decompressor: flate2::Decompress,
    compressor: flate2::Compress,
}

#[derive(Debug, Default)]
pub struct Gzip {
    config: Config,
    state: thread_local::CachedThreadLocal<RefCell<State>>,
}

impl Gzip {
    fn state(&self) -> &RefCell<State> {
        self.state.get_or(|| RefCell::new(State::new(self.config)))
    }
    fn decompressor(&self) -> RefMut<flate2::Decompress> {
        let state = self.state().borrow_mut();
        let mut decompressor = RefMut::map(state, |s| &mut s.decompressor);
        decompressor.reset(true);
        decompressor
    }

    fn compressor(&self) -> RefMut<flate2::Compress> {
        let state = self.state().borrow_mut();
        let mut compressor = RefMut::map(state, |s| &mut s.compressor);
        compressor.reset();
        compressor
    }
}

impl super::Compress for Gzip {
    fn load(options: &[u8]) -> io::Result<Self>
    where
        Self: Sized,
    {
        let config: Config = packed_serialize::try_read(options)?.unwrap_or_default();
        if config.compression_level < 1 || config.compression_level > 9 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid compression level ({})", config.compression_level),
            ));
        }
        if config.window_size < 9 || config.window_size > 15 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid window size ({})", config.window_size),
            ));
        }
        Ok(Self {
            config,
            state: thread_local::CachedThreadLocal::new(),
            // state: flate2::Decompress::new(true),
        })
    }

    fn compress(&self, src: &[u8], dst: &mut [u8]) -> io::Result<usize> {
        let mut compressor = self.compressor();
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

    fn decompress(&self, src: &[u8], dst: &mut [u8]) -> io::Result<usize> {
        let mut decompressor = self.decompressor();
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

impl State {
    fn new(config: Config) -> Self {
        let compression = flate2::Compression::new(config.compression_level);
        Self {
            decompressor: flate2::Decompress::new(true),
            compressor: flate2::Compress::new(compression, true),
            // compressor: flate2::Compress::new_with_window_bits(compression, true, window_bits),
        }
    }
}

fn min_mem(file_size: u64, mem_size: usize) -> usize {
    if file_size < mem_size as u64 {
        file_size as usize
    } else {
        mem_size
    }
}
