use crate::compress_threads::ParallelCompressor;
use crate::pool;
use crate::write::{fragments, ReadHoles};
use crossbeam_channel::Receiver;
use futures::channel::oneshot;
use std::io;
use std::io::{Error, Read};
use std::sync::Arc;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FragmentConfig {
    NoFragments,
    FragmentEnds,
    FragmentAlways,
}

pub struct Datablocks<W> {
    writer: W,
    current_offset: u64,
    block_size: u32,
    fragment_config: FragmentConfig,
}

impl<W: io::Write> Datablocks<W> {
    pub fn new(
        writer: W,
        fragment_config: FragmentConfig,
        compressor: Option<Arc<ParallelCompressor>>,
    ) -> Self {
        Self {
            writer,
            current_offset: 0,
            fragment_config,
        }
    }

    pub fn position(&self) -> repr::datablock::Ref {
        repr::datablock::Ref(self.current_offset)
    }
}

struct Request {
    file: Box<dyn ReadHoles>,
    tx: oneshot::Sender<io::Result<Response>>,
}

struct Response {
    uncompressed_size: u64,
    sparse_bytes: u64,
    sizes: Vec<repr::datablock::Size>,
    fragment: Option<repr::fragment::Idx>,
}

fn handle_file(
    block_size: usize,
    compressor: Option<&ParallelCompressor>,
    mut file: Box<dyn ReadHoles>,
) -> io::Result<Response> {
    let mut sizes = Vec::new();
    let mut do_skip = true;
    loop {
        let mut block = pool::block();
        if do_skip {
            let mut hole_size = match file.skip_hole() {
                Ok(size) => size,
                Err(e) if e.kind() == io::ErrorKind::Unsupported => {
                    do_skip = false;
                    0
                }
                Err(e) => return Err(e),
            };
            let empty_blocks = (hole_size / block_size as u64) as usize;
            let remaining = (hole_size % block_size as u64) as usize;
            sizes.resize(sizes.len() + empty_blocks, repr::datablock::Size::ZERO);
            block.resize(remaining, 0);
        }

        let to_fill = block_size - block.len();
        let bytes_read = file.by_ref().take(to_fill as u64).read_to_end(&mut block)?;
        if let Some(compressor) = compressor {
            compressor.compress(block.detach());
        }
    }
    unimplemented!()
}

fn writer_thread<W: io::Write>(
    mut writer: W,
    block_size: usize,
    compressor: Option<Arc<ParallelCompressor>>,
    fragment_config: FragmentConfig,
    rx: Receiver<Request>,
) -> fragments::Table {
    let fragments = fragments::Table::new(compressor.clone());
    let current_fragments = [Vec::new(); 2];
    for mut request in rx {
        handle_file(request.file);
    }

    fragments
}
