use sqfs::read::Archive;

use sloggers::terminal::{Destination, TerminalLoggerBuilder};
use sloggers::types::{Format, Severity};
use sloggers::Build;

use std::env;
use std::path::Path;

fn main() {
    std::process::exit(real_main());
}

fn real_main() -> i32 {
    let mut builder = TerminalLoggerBuilder::new();
    builder.level(Severity::Debug);
    builder.destination(Destination::Stderr);
    builder.format(Format::Full);
    let logger = builder.build().unwrap();

    let file_name = env::args_os().nth(1);
    let file_name = file_name
        .as_ref()
        .map_or_else(|| Path::new("tmp.squashfs"), Path::new);

    let _archive = match Archive::open_with_logger(file_name, logger.clone()) {
        Ok(archive) => archive,
        Err(e) => {
            slog::crit!(logger, "{}", e);
            return 1;
        }
    };

    0
}
