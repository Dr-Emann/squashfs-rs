use crate::compression;
use crate::compression::{AnyCodec, Decompressor};
use crate::errors::*;
use crate::shared_position_file::Positioned;
use byteorder::ReadBytesExt;
use positioned_io::{RandomAccessFile, ReadAt};
use slog::Logger;
use std::cell::RefCell;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::{io, mem};
use thread_local::ThreadLocal;
use zerocopy::FromBytes;

#[derive(Debug)]
pub struct Archive<R> {
    inner: Arc<ArchiveInner<R>>,
}

#[derive(Debug)]
struct ArchiveInner<R> {
    reader: R,
    superblock: repr::superblock::Superblock,
    compressor_base: compression::AnyCodec,
    compressor: ThreadLocal<RefCell<compression::AnyCodec>>,
    logger: Logger,
}

impl Archive<RandomAccessFile> {
    pub fn open<P: AsRef<Path>>(p: P) -> Result<Self> {
        Archive::open_with_logger(p, crate::default_logger())
    }

    pub fn open_with_logger<P: AsRef<Path>>(p: P, logger: Logger) -> Result<Self> {
        Self::_open_with_logger(p.as_ref(), logger)
    }

    fn _open_with_logger(path: &Path, logger: Logger) -> Result<Self> {
        let path_str = path.display().to_string();
        let logger = logger.new(slog::o!("file" => path_str));
        let file = RandomAccessFile::open(path)?;
        Self::with_logger(file, logger)
    }
}

impl<R: ReadAt> Archive<R> {
    pub fn new(reader: R) -> Result<Self, Error> {
        Self::with_logger(reader, crate::default_logger())
    }

    pub fn with_logger(mut reader: R, logger: Logger) -> Result<Self> {
        let mut positioned = Positioned::new(&mut reader);

        let superblock: repr::superblock::Superblock = repr::read(&mut positioned)?;
        log_superblock(&logger, &superblock);

        let compressor_kind = validate_superblock(&superblock)?;
        let flags = superblock.flags;
        // Check for unknown bits
        if !(flags & repr::superblock::Flags::all()).is_empty() {
            return Err(SuperblockError::UnsupportedOption(format!(
                "Unknown superblock flags in {:x}",
                flags
            ))
            .into());
        }
        let compressor = if flags.contains(repr::superblock::Flags::COMPRESSOR_OPTIONS) {
            let mut options = [0; repr::metablock::SIZE];
            let size = read_metablock(&mut positioned, None, &mut options, false, &logger)?;
            AnyCodec::configured(compressor_kind, &options[..size])?
        } else {
            AnyCodec::new(compressor_kind)
        };
        slog::info!(logger, "Loaded compressor {:?}", compressor.config(); "compression_kind" => %compressor.kind());

        let id_indexes = metablock_indexes::<repr::uid_gid::Id, _>(
            &mut reader,
            superblock.id_table_start,
            superblock.xattr_id_table_start,
            superblock.id_count.into(),
        );

        eprintln!("yo: {:#?}", id_indexes.collect::<io::Result<Vec<_>>>());

        Ok(Self {
            inner: Arc::new(ArchiveInner {
                reader,
                superblock,
                compressor_base: compressor,
                compressor: ThreadLocal::new(),
                logger,
            }),
        })
    }
}

struct MetablockIndexes<R> {
    reader: R,
    block_count: usize,
}

fn metablock_indexes<T: FromBytes, At: ReadAt>(
    reader: At,
    start: u64,
    end: u64,
    item_count: usize,
) -> MetablockIndexes<impl io::Read> {
    assert!(end > start);
    let total_size = mem::size_of::<T>() * item_count;
    let block_count = (total_size + (repr::metablock::SIZE - 1)) / repr::metablock::SIZE;
    let reader = Positioned::with_position(reader, start).take(end - start);
    MetablockIndexes {
        reader,
        block_count,
    }
}

impl<R> MetablockIndexes<R> {}

impl<R: io::Read> Iterator for MetablockIndexes<R> {
    type Item = io::Result<u64>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.block_count == 0 {
            return None;
        }

        self.block_count -= 1;
        let result = [0u8; std::mem::size_of::<u64>()];
        Some(self.reader.read_u64::<byteorder::LE>())
    }
}

fn validate_superblock(
    superblock: &repr::superblock::Superblock,
) -> Result<compression::Kind, SuperblockError> {
    if superblock.magic != repr::superblock::MAGIC {
        return Err(SuperblockError::BadMagic {
            magic: superblock.magic,
        });
    }
    if superblock.version_major != repr::superblock::VERSION_MAJOR
        || superblock.version_minor != repr::superblock::VERSION_MINOR
    {
        return Err(SuperblockError::BadVersion {
            major: superblock.version_major,
            minor: superblock.version_minor,
        });
    }
    if superblock.block_size != 1 << superblock.block_log {
        return Err(SuperblockError::CorruptBlockSizes {
            block_log: superblock.block_log,
            block_size: superblock.block_size,
        });
    }

    let compression_kind = compression::Kind::from_id(superblock.compression_id);
    if compression_kind == compression::Kind::Unknown {
        return Err(SuperblockError::UnknownCompression {
            id: superblock.compression_id,
        });
    }
    if !compression_kind.supported() {
        return Err(SuperblockError::DisabledCompression {
            kind: compression_kind,
        });
    }
    Ok(compression_kind)
}

fn read_metablock<R: io::Read>(
    mut reader: R,
    compressor: Option<&mut compression::AnyCodec>,
    dst: &mut [u8],
    exact: bool,
    logger: &Logger,
) -> Result<usize, Error> {
    let header: repr::metablock::Header = repr::read(&mut reader)?;
    let compressed = header.compressed();
    let size = header.size() as usize;
    if size > repr::metablock::SIZE {
        return Err(MetablockError::HugeMetablock(size).into());
    }

    if compressed {
        let compressor = compressor.ok_or(MetablockError::CompressedCompressorOptions)?;
        // TODO: Is it worth it to use uninitialized?
        let mut intermediate = [0; repr::metablock::SIZE];
        // Safe to slice because of above ensure!
        reader.read_exact(&mut intermediate[..size])?;
        let size = compressor.decompress(&intermediate[..size], dst)?;
        if exact && size != dst.len() {
            return Err(MetablockError::UnexpectedMetablockSize {
                actual: size,
                expected: dst.len(),
            }
            .into());
        }
        Ok(size)
    } else {
        if exact && size != dst.len() {
            return Err(MetablockError::UnexpectedMetablockSize {
                actual: size,
                expected: dst.len(),
            }
            .into());
        }
        reader.read_exact(&mut dst[..size])?;
        Ok(size)
    }
}

fn log_superblock(logger: &Logger, superblock: &repr::superblock::Superblock) {
    slog::debug!(logger, "Read superblock";
        "magic" => superblock.magic,
        "inode_count" => superblock.inode_count,
        "modification_time" => superblock.modification_time.0,
        "block_size" => superblock.block_size,
        "fragment_entry_count" => superblock.fragment_entry_count,
        // Extra braces to avoid a reference to a packed field
        "compression_id" => ?{superblock.compression_id},
        "block_log" => superblock.block_log,
        // Extra braces to avoid a reference to a packed field
        "flags" => ?{superblock.flags},
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
    )
}
