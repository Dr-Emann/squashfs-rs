use sqfs::read::Archive;

fn main() {
    let archive = Archive::open("tmp.squashfs").unwrap();
    println!("{:#?}", archive);
}
