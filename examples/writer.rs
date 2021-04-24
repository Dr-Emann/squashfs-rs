use chrono::{DateTime, Utc};
use sqfs::write;
use std::io::Write;
use tempfile::tempfile;

fn main() {
    /*
    let f = tempfile().expect("Unable to open a temp file");
    let mut archive = write::Archive::from_writer(Box::new(
        positioned_io::RandomAccessFile::try_new(f).unwrap(),
    ));
    println!("{:#?}", archive);
    let mut root = archive.begin_root();

    let child_dir = root.begin_dir("subdir");
    let mut child_dir = child_dir.done_subdirs();
    child_dir.set_mode(sqfs::Mode::from_bits(0o755).unwrap());
    child_dir.set_uid(1000).set_gid(1000);
    child_dir.set_modified_time(DateTime::from(std::time::UNIX_EPOCH));

    let mut file = archive.create_file("hi there".as_bytes());
    file.set_mode(sqfs::Mode::from_bits(0o555).unwrap());
    file.set_uid(1000).set_gid(2000);
    file.set_modified_time(DateTime::from(std::time::UNIX_EPOCH));
    let file_ref = file.finish();

    child_dir.add("my_file", file_ref);
    child_dir.finish();

    let mut root = root.done_subdirs();
    root.add("my_file_link", file_ref);

    let item = root.build(&mut archive);
    archive.set_root(item);
    println!("{:#?}", archive);
    archive.flush().expect("Unable to flush");
     */
}
