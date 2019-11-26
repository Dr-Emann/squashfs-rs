use sqfs::read::Archive;

use sloggers::terminal::{Destination, TerminalLoggerBuilder};
use sloggers::types::Severity;
use sloggers::Build;

fn main() {
    let mut builder = TerminalLoggerBuilder::new();
    builder.level(Severity::Debug);
    builder.destination(Destination::Stderr);

    let archive = Archive::open_with_logger("tmp.squashfs", builder.build().unwrap()).unwrap();
    println!("{:#?}", archive);
}
