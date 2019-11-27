use sqfs::read::Archive;

use sloggers::terminal::{Destination, TerminalLoggerBuilder};
use sloggers::types::{Format, Severity};
use sloggers::Build;

fn main() {
    std::process::exit(real_main());
}

fn real_main() -> i32 {
    let mut builder = TerminalLoggerBuilder::new();
    builder.level(Severity::Debug);
    builder.destination(Destination::Stderr);
    builder.format(Format::Full);
    let logger = builder.build().unwrap();

    let archive = match Archive::open_with_logger("tmp.squashfs", logger.clone()) {
        Ok(archive) => archive,
        Err(e) => {
            slog::crit!(logger, "{}", e);
            return 1;
        }
    };

    0
}
