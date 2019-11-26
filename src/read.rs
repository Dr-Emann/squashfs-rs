use crate::compression;
use crate::shared_position_file::Positioned;
use packed_serialize;
use positioned_io::{RandomAccessFile, ReadAt};
use std::io;
use std::path::Path;
use std::sync::Arc;

use slog::*;

#[derive(Debug)]
pub struct Archive<R> {
    inner: Arc<ArchiveInner<R>>,
}

#[derive(Debug)]
struct ArchiveInner<R> {
    reader: R,
    superblock: repr::superblock::Superblock,
    compressor: compression::Compressor,
    logger: Logger,
}

fn default_logger() -> Logger {
    slog::Logger::root(slog_stdlog::StdLog.fuse(), o!())
}

impl Archive<RandomAccessFile> {
    pub fn open<P: AsRef<Path>>(p: P) -> io::Result<Self> {
        Archive::open_with_logger(p, default_logger())
    }

    pub fn open_with_logger<P: AsRef<Path>>(p: P, logger: Logger) -> io::Result<Self> {
        Self::_open_with_logger(p.as_ref(), logger)
    }

    fn _open_with_logger(path: &Path, logger: Logger) -> io::Result<Self> {
        let path_str = path.display().to_string();
        let logger = logger.new(o!("file" => path_str));
        let file = RandomAccessFile::open(path)?;
        Self::with_logger(file, logger)
    }
}

impl<R: ReadAt> Archive<R> {
    pub fn new(reader: R) -> io::Result<Self> {
        Self::with_logger(reader, default_logger())
    }

    pub fn with_logger(mut reader: R, logger: Logger) -> io::Result<Self> {
        let mut positioned = Positioned::new(&mut reader);
        let superblock: repr::superblock::Superblock = packed_serialize::read(&mut positioned)?;

        info!(logger, "Read superblock";
            "magic" => superblock.magic,
            "inode_count" => superblock.inode_count,
            "modification_time" => superblock.modification_time,
            "block_size" => superblock.block_size,
            "fragment_entry_count" => superblock.fragment_entry_count,
            "compression_id" => ?superblock.compression_id,
            "block_log" => superblock.block_log,
            "flags" => ?superblock.flags,
            "id_count" => superblock.id_count,
            "version_major" => superblock.version_major,
            "version_minor" => superblock.version_minor,
            "root_inode_ref" => ?superblock.root_inode_ref,
            "bytes_used" => superblock.bytes_used,
            "id_table_start" => superblock.id_table_start,
            "xattr_id_table_start" => superblock.xattr_id_table_start,
            "inode_table_start" => superblock.inode_table_start,
            "directory_table_start" => superblock.directory_table_start,
            "fragment_table_start" => superblock.fragment_table_start,
            "export_table_start" => superblock.export_table_start
        );

        let compression_kind = compression::Kind::from_id(superblock.compression_id);
        if !compression_kind.supported() {
            error!(logger, "Unsupported compression: {:?}", compression_kind);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Unsupported compression",
            ));
        }

        let compressor = compression_kind.compressor();
        // TODO: Load compression options
        assert!(!superblock
            .flags
            .contains(repr::superblock::Flags::COMPRESSOR_OPTIONS));
        info!(logger, "Loaded compressor {:?}", compressor);

        Ok(Self {
            inner: Arc::new(ArchiveInner {
                reader,
                superblock,
                compressor,
                logger,
            }),
        })
    }
}
