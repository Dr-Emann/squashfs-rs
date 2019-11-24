use crate::compression;
use crate::shared_position_file::Positioned;
use packed_serialize;
use positioned_io::{RandomAccessFile, ReadAt};
use std::io;
use std::path::Path;
use std::sync::Arc;

pub struct Archive<R> {
    inner: Arc<ArchiveInner<R>>,
}

#[derive(Debug)]
struct ArchiveInner<R> {
    reader: R,
    compressor: compression::Compressor,
}

impl Archive<RandomAccessFile> {
    pub fn open<P: AsRef<Path>>(p: P) -> io::Result<Self> {
        let file = RandomAccessFile::open(p)?;
        Archive::new(file)
    }
}

impl<R: ReadAt> Archive<R> {
    pub fn new(mut reader: R) -> io::Result<Self> {
        let mut positioned = Positioned::new(&mut reader);
        let superblock: repr::superblock::Superblock = packed_serialize::read(&mut positioned)?;
        let compression_kind = compression::Kind::from_id(superblock.compression_id);
        if !compression_kind.supported() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Unsupported compression",
            ));
        }
        unimplemented!()
    }
}
