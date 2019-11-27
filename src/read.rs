use crate::compression;
use crate::errors::*;
use crate::shared_position_file::Positioned;
use packed_serialize;
use positioned_io::{RandomAccessFile, ReadAt};
use snafu::ensure;
use std::io;
use std::path::Path;
use std::sync::Arc;

use slog::{debug, error, info, o, trace, Drain, Logger};

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
    pub fn open<P: AsRef<Path>>(p: P) -> Result<Self, Error> {
        Archive::open_with_logger(p, default_logger())
    }

    pub fn open_with_logger<P: AsRef<Path>>(p: P, logger: Logger) -> Result<Self, Error> {
        Self::_open_with_logger(p.as_ref(), logger)
    }

    fn _open_with_logger(path: &Path, logger: Logger) -> Result<Self, Error> {
        let path_str = path.display().to_string();
        let logger = logger.new(o!("file" => path_str));
        let file = RandomAccessFile::open(path)?;
        Self::with_logger(file, logger)
    }
}

impl<R: ReadAt> Archive<R> {
    pub fn new(reader: R) -> Result<Self, Error> {
        Self::with_logger(reader, default_logger())
    }

    pub fn with_logger(mut reader: R, logger: Logger) -> Result<Self, Error> {
        let mut positioned = Positioned::new(&mut reader);
        let superblock: repr::superblock::Superblock = packed_serialize::read(&mut positioned)?;

        debug!(logger, "Read superblock";
            "magic" => superblock.magic,
            "inode_count" => superblock.inode_count,
            "modification_time" => superblock.modification_time,
            "block_size" => superblock.block_size,
            "fragment_entry_count" => superblock.fragment_entry_count,
            "compression_id" => ?superblock.compression_id,
            "block_log" => superblock.block_log,
            "flags" => superblock.flags,
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

        ensure!(
            superblock.magic == repr::superblock::MAGIC,
            BadMagic {
                magic: superblock.magic
            }
        );
        ensure!(
            superblock.version_major == repr::superblock::VERSION_MAJOR
                && superblock.version_minor == repr::superblock::VERSION_MINOR,
            BadVersion {
                major: superblock.version_major,
                minor: superblock.version_minor,
            }
        );
        ensure!(
            superblock.block_size == 1 << superblock.block_log,
            InvalidMetadata {
                err: format!(
                    "Corrupt block size, {:#x} != {:#x}",
                    superblock.block_size,
                    1 << superblock.block_log
                )
            }
        );

        let compression_kind = compression::Kind::from_id(superblock.compression_id);
        ensure!(
            compression_kind != compression::Kind::Unknown,
            UnknownCompression {
                compression_id: superblock.compression_id,
            }
        );
        ensure!(
            compression_kind.supported(),
            DisabledCompression {
                compression_kind: compression_kind,
            }
        );

        let compressor = compression_kind.compressor();
        // TODO: Load compression options
        let flags = match repr::superblock::Flags::from_bits(superblock.flags) {
            Some(flags) => flags,
            None => {
                return UnsupportedOption {
                    err: format!("Unknown superblock flags in {:x}", superblock.flags),
                }
                .fail()
                .map_err(Into::into);
            }
        };

        assert!(!flags.contains(repr::superblock::Flags::COMPRESSOR_OPTIONS));
        info!(logger, "Loaded compressor {:?}", compressor.config();
            "compression_kind" => %compression_kind);

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
