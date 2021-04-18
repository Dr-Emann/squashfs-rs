use chrono::{DateTime, Utc};
use sqfs::write;
use tempfile::tempfile;

fn main() {
    let f = tempfile().expect("Unable to open a temp file");
    let mut archive = write::Archive::from_writer(Box::new(
        positioned_io::RandomAccessFile::try_new(f).unwrap(),
    ));
    println!("{:#?}", archive);

    let mut root = archive.root();
    root.set_mode(sqfs::Mode::from_bits(0o755).unwrap());
    root.set_uid(1000);
    root.set_gid(1000);
    root.set_modified_time(DateTime::from(std::time::UNIX_EPOCH));

    {
        let mut child1 = root.create_file("a.txt");
        let mut child2 = root.create_file("b.txt");
        write!(child1, "hi there").unwrap();
        write!(child2, "Hi child {}", 2).unwrap();
    }

    archive.flush().expect("Unable to flush");
}
