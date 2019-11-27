use snafu::{IntoError, Snafu};
use std::io;

#[derive(Debug, Snafu)]
pub struct Error(ErrorInner);

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum ErrorInner {
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
    #[snafu(display("Invalid metadata: {}", err))]
    InvalidMetadata { err: String },
    #[snafu(display("Unknown compression type: {}", compression_id.0))]
    UnknownCompression {
        compression_id: repr::compression::Id,
    },
    #[snafu(display("sqfs built without support for {}", compression_kind))]
    DisabledCompression {
        compression_kind: crate::compression::Kind,
    },
    #[snafu(display("Unsupported option: {}", err))]
    UnsupportedOption { err: String },
    #[snafu(display("IO error: {}", source))]
    Io {
        source: io::Error,
        backtrace: snafu::Backtrace,
    },
}

impl From<io::Error> for ErrorInner {
    fn from(e: io::Error) -> Self {
        Io.into_error(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error(e.into())
    }
}
