use chrono::{DateTime, Utc};
use sqfs::write;
use tempfile::tempfile;

fn main() {
    let f = tempfile().expect("Unable to open a temp file");
    let mut archive = write::Archive::from_writer(Box::new(
        positioned_io::RandomAccessFile::try_new(f).unwrap(),
    ));
    println!("{:#?}", archive);
    let mut root = archive.create_dir();
    root.set_mode(sqfs::Mode::from_bits(0o755).unwrap());
    root.set_uid(1000).set_gid(1000);
    root.set_modified_time(DateTime::from(std::time::UNIX_EPOCH));
    let item = root.build();
    archive.set_root(item);
    println!("{:#?}", archive);
    archive.flush().expect("Unable to flush");
}
