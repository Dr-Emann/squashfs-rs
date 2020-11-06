use std::io;
use thiserror::Error as ThisError;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, ThisError)]
#[error(transparent)]
pub struct Error(#[from] ErrorInner);

#[derive(Debug, ThisError)]
pub(crate) enum ErrorInner {
    #[error("Superblock error: {0}")]
    BadSuperblock(#[from] SuperblockError),

    #[error("Metablock error: {0}")]
    Metablock(#[from] MetablockError),

    #[error(transparent)]
    Io(#[from] io::Error),
}

#[derive(Debug, ThisError)]
pub(crate) enum SuperblockError {
    #[error("Magic mismatch: expected {:#x}, got {:#x}", repr::superblock::MAGIC, .magic)]
    BadMagic { magic: u32 },

    #[error("Invalid archive version {major}.{minor}: sqfs only supports version 4.0")]
    BadVersion { major: u16, minor: u16 },

    #[error("Unknown compression type: {}", .id.0)]
    UnknownCompression { id: repr::compression::Id },

    #[error("sqfs built without support for {kind}")]
    DisabledCompression { kind: crate::compression::Kind },

    #[error("Block size ({actual}) invalid")]
    OutOfRangeBlockSize { actual: u32 },

    #[error("Block size mismatch ({}/{})", (1 << * (.block_log) as u32), .block_size)]
    CorruptBlockSizes { block_log: u16, block_size: u32 },

    #[error("Invalid start of section {section} ({offset})")]
    InvalidSectionStart { section: &'static str, offset: u64 },

    #[error("Unsupported option: {0}")]
    UnsupportedOption(String),
}

#[derive(Debug, ThisError)]
pub(crate) enum MetablockError {
    #[error("Metadata block size too large {0} (max {})", ::repr::metablock::SIZE)]
    HugeMetablock(usize),

    #[error("Metadata block size mismatch: expected {expected}, got {actual}")]
    UnexpectedMetablockSize { actual: usize, expected: usize },

    #[error("Compressor options cannot require compression")]
    CompressedCompressorOptions,
}

impl From<SuperblockError> for Error {
    fn from(e: SuperblockError) -> Self {
        Error(e.into())
    }
}

impl From<MetablockError> for Error {
    fn from(e: MetablockError) -> Self {
        Error(e.into())
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error(e.into())
    }
}
