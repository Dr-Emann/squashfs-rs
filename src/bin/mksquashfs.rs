use squashfs_rs::write::{Archive, Config};

fn main() {
    let config = Config::new();
    let archive = Archive::create("./tmp.squashfs", &config).unwrap();
    println!("{:?}", archive);
    archive.flush().unwrap();
}
