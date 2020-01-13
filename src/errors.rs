use snafu::{IntoError, Snafu};
use std::io;
use std::path::PathBuf;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
pub struct Error(ErrorInner);

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum ErrorInner {
    #[snafu(display("Unable to open {}: {}", path.display(), source))]
    UnableToOpen { path: PathBuf, source: io::Error },

    #[snafu(display("Superblock error: {}", source))]
    BadSuperblock { source: SuperblockError },

    #[snafu(display("Superblock error: {}", source))]
    BadMetablock { source: MetablockError },
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum SuperblockError {
    #[snafu(display(
        "Magic mismatch: expected {:#x}, got {:#x}",
        repr::superblock::MAGIC,
        magic
    ))]
    BadMagic { magic: u32 },

    #[snafu(display(
        "Invalid archive version {}.{}: sqfs only supports version 4.0",
        major,
        minor
    ))]
    BadVersion { major: u16, minor: u16 },

    #[snafu(display("Unknown compression type: {}", compression_id.0))]
    UnknownCompression {
        compression_id: repr::compression::Id,
    },

    #[snafu(display("sqfs built without support for {}", compression_kind))]
    DisabledCompression {
        compression_kind: crate::compression::Kind,
    },

    #[snafu(display("Block size ({}) invalid", actual))]
    OutOfRangeBlockSize { actual: u32 },

    #[snafu(display("Block size mismatch ({}/{})", (1 << *block_log as u32), block_size))]
    CorruptBlockSizes { block_log: u16, block_size: u32 },

    #[snafu(display("Unsupported option: {}", err))]
    UnsupportedOption { err: String },

    #[snafu(display("IO error: {}", source))]
    SuperblockIo { source: io::Error },
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum MetablockError {
    #[snafu(display(
        "Metadata block size too large {} (max {})",
        actual,
        ::repr::metablock::SIZE
    ))]
    HugeMetablock { actual: usize },

    #[snafu(display("Metadata block size mismatch: expected {}, got {}", expected, actual))]
    UnexpectedMetablockSize { actual: usize, expected: usize },

    #[snafu(display("Compressor options cannot require compression"))]
    CompressedCompressorOptions,

    #[snafu(display("IO error: {}", source))]
    MetablockIo {
        source: io::Error,
        backtrace: snafu::Backtrace,
    },
}

impl From<SuperblockError> for ErrorInner {
    fn from(e: SuperblockError) -> Self {
        BadSuperblock.into_error(e)
    }
}

impl From<SuperblockError> for Error {
    fn from(e: SuperblockError) -> Self {
        Self(e.into())
    }
}

impl From<MetablockError> for ErrorInner {
    fn from(e: MetablockError) -> Self {
        BadMetablock.into_error(e)
    }
}

impl From<MetablockError> for Error {
    fn from(e: MetablockError) -> Self {
        Self(e.into())
    }
}

impl From<io::Error> for SuperblockError {
    fn from(e: io::Error) -> Self {
        SuperblockIo.into_error(e)
    }
}

impl From<io::Error> for MetablockError {
    fn from(e: io::Error) -> Self {
        MetablockIo.into_error(e)
    }
}
