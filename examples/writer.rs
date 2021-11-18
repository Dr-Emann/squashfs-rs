use chrono::DateTime;
use sqfs::write;
use tempfile::tempfile;

#[tokio::main]
async fn main() {
    let f = tempfile().expect("Unable to open a temp file");
    let mut archive = write::Archive::from_writer(Box::new(
        positioned_io::RandomAccessFile::try_new(f).unwrap(),
    ));
    println!("{:#?}", archive);
    let mut root = archive.create_dir();

    let mut child_dir = archive.create_dir();
    child_dir.set_mode(sqfs::Mode::O755);
    child_dir.set_uid(1000).set_gid(1000);
    child_dir.set_modified_time(DateTime::from(std::time::UNIX_EPOCH));

    /*
    let mut file = archive.create_file();
    file.set_contents(Box::new(b"hi there" as &[u8]));
    file.set_mode(sqfs::Mode::from_bits_truncate(0o555));
    file.set_uid(1000).set_gid(2000);
    file.set_modified_time(DateTime::from(std::time::UNIX_EPOCH));
    let file_ref = file.finish(&mut archive);

    child_dir.add_item("my_file", file_ref);
     */
    let child_dir_ref = child_dir.finish(&mut archive);

    // root.add_item("my_file_link", file_ref);
    root.add_item("subdir", child_dir_ref);

    let root_ref = root.finish(&mut archive);
    archive.set_root(root_ref);
    println!("{:#?}", archive);
    archive.flush().expect("Unable to flush");
}
