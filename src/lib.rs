#![allow(unused_variables, dead_code)]

use slog::Drain;

mod compress_threads;
mod compression;
pub mod config;
mod pool;
pub mod read;
pub mod shared_position_file;
pub mod write;

pub(crate) mod errors;

pub use repr::Mode;

fn default_logger() -> slog::Logger {
    slog::Logger::root(slog_stdlog::StdLog.fuse(), slog::o!())
}
