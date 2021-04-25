use chrono::DateTime;
use sqfs::write;
use tempfile::tempfile;

fn main() {
    let f = tempfile().expect("Unable to open a temp file");
    let mut archive = write::Archive::from_writer(Box::new(
        positioned_io::RandomAccessFile::try_new(f).unwrap(),
    ));
    println!("{:#?}", archive);
    let root = archive.begin_root();

    let child_dir = root.begin_dir("subdir");
    let mut child_dir = child_dir.done_subdirs();
    child_dir.set_mode(sqfs::Mode::O755);
    child_dir.set_uid(1000).set_gid(1000);
    child_dir.set_modified_time(DateTime::from(std::time::UNIX_EPOCH));

    let mut file = archive.create_file();
    file.set_contents(Box::new(b"hi there" as &[u8]));
    file.set_mode(sqfs::Mode::from_bits_truncate(0o555));
    file.set_uid(1000).set_gid(2000);
    file.set_modified_time(DateTime::from(std::time::UNIX_EPOCH));
    let file_ref = file.finish(&mut archive);

    child_dir.add_item("my_file", file_ref);
    child_dir.finish(&mut archive);

    let mut root = root.done_subdirs();
    root.add_item("my_file_link", file_ref);

    let item = root.finish(&mut archive);
    archive.set_root(item);
    println!("{:#?}", archive);
    archive.flush().expect("Unable to flush");
}
