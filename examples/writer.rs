use chrono::{DateTime, Utc};
use sqfs::write;

fn main() {
    let mut archive =
        write::Archive::create("/tmp/sqfs.squashfs").expect("Can't create an archive");
    println!("{:#?}", archive);
    let mut empty_dir = archive.create_dir();
    empty_dir.set_mode(sqfs::Mode::from_bits(0o755).unwrap());
    empty_dir.set_uid(1000).set_gid(1000);
    empty_dir.set_modified_time(DateTime::from(std::time::UNIX_EPOCH));
    let empty_dir = empty_dir.build();
    archive.set_root(empty_dir);
    archive.flush().expect("Unable to flush");
}
