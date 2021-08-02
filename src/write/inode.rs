use super::metablock_writer::MetablockWriter;
use crate::compress_threads::ParallelCompressor;
use crate::Mode;
use std::convert::TryInto;
use std::io;
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct Table {
    writer: MetablockWriter,
    count: u32,
}

impl Table {
    pub fn new(compressor: Option<Arc<ParallelCompressor>>) -> Self {
        Self {
            writer: MetablockWriter::new(compressor),
            count: 0,
        }
    }

    pub async fn finish(self) -> Vec<u8> {
        self.writer.finish().await
    }

    pub async fn add(&mut self, entry: Entry) -> io::Result<repr::inode::Ref> {
        let result = self.writer.position();

        let extended = entry.needs_ext();

        let inode_number = repr::inode::Idx(self.count);
        self.count += 1;

        let header = repr::inode::Header {
            inode_type: entry.data.inode_kind(extended),
            permissions: entry.common.permissions & Mode::PERM_MASK,
            uid_idx: entry.common.uid_idx,
            gid_idx: entry.common.gid_idx,
            modified_time: entry.common.modified_time,
            inode_number,
        };

        self.writer.write(&header).await;

        let common = &entry.common;

        match &entry.data {
            Data::Directory(dir_data) => {
                if !extended {
                    self.write_basic_dir(common, dir_data).await;
                } else {
                    self.write_ext_dir(common, dir_data).await;
                }
            }
            Data::File(file_data) => {
                if !extended {
                    self.write_basic_file(common, file_data).await;
                } else {
                    self.write_ext_file(common, file_data).await;
                }
            }
            Data::Symlink(symlink_data) => self.write_symlink(common, symlink_data).await,
            Data::BlockDev(dev_data) | Data::CharDev(dev_data) => {
                if !extended {
                    self.write_basic_device(common, dev_data).await;
                } else {
                    self.write_ext_device(common, dev_data).await;
                }
            }
            Data::Fifo | Data::Socket => {
                if !extended {
                    self.write_basic_ipc(common).await;
                } else {
                    self.write_ext_ipc(common).await;
                }
            }
        }

        Ok(result)
    }

    async fn write_basic_dir(&mut self, common: &Common, data: &DirData) {
        let body = repr::inode::BasicDir {
            dir_block_start: data.dir_ref.block_start(),
            // Note that for historical reasons, the hard link count of a directory includes
            // the number of entries in the directory and is initialized to 2 for an empty
            // directory. I.e. a directory with N entries has at least N + 2 link count.
            hard_link_count: repr::inode::dir_hardlink_count(
                common.hardlink_count,
                data.child_count,
            ),
            // Safe because we should never write a basic dir if we need an extended one
            file_size: repr::inode::dir_stored_size(data.dir_size)
                .try_into()
                .expect("Should not try to make a basic dir with a large size"),
            block_offset: data.dir_ref.start_offset(),
            parent_inode_number: data.parent_inode_num,
        };

        self.writer.write(&body).await;
    }

    async fn write_ext_dir(&mut self, common: &Common, data: &DirData) {
        let body = repr::inode::ExtendedDir {
            hard_link_count: repr::inode::dir_hardlink_count(
                common.hardlink_count,
                data.child_count,
            ),
            file_size: repr::inode::dir_stored_size(data.dir_size),
            dir_block_start: data.dir_ref.block_start(),
            parent_inode_number: data.parent_inode_num,
            index_count: data
                .header_locations
                .as_ref()
                .map_or(0, |locations| locations.len().try_into().unwrap()),
            block_offset: data.dir_ref.start_offset(),
            xattr_idx: common.xattr_idx,
        };

        self.writer.write(&body).await;

        todo!("Need to write header locations")
    }

    async fn write_basic_file(&mut self, common: &Common, data: &FileData) {
        let body = repr::inode::BasicFile {
            blocks_start: data
                .blocks_start
                .0
                .try_into()
                .expect("Should not try to make a basic file with a large blocks_start"),
            fragment_block_index: data.fragment_block_idx,
            block_offset: data.fragment_offset,
            file_size: data
                .file_size
                .try_into()
                .expect("Should not make a basic file with a large file size"),
        };

        self.writer.write(&body).await;
        for block_size in &data.block_sizes {
            self.writer.write(block_size).await;
        }
    }

    async fn write_ext_file(&mut self, common: &Common, data: &FileData) {
        let body = repr::inode::ExtendedFile {
            blocks_start: data.blocks_start,
            fragment_block_index: data.fragment_block_idx,
            block_offset: data.fragment_offset,
            file_size: data.file_size,
            sparse: data.sparse_bytes,
            hard_link_count: common.hardlink_count,
            xattr_idx: common.xattr_idx,
        };

        self.writer.write(&body).await;
        for block_size in &data.block_sizes {
            self.writer.write(block_size).await;
        }
    }

    async fn write_symlink(&mut self, common: &Common, data: &SymlinkData) {
        let body = repr::inode::Symlink {
            hard_link_count: common.hardlink_count,
            target_size: data.target_path.len().try_into().unwrap(),
        };

        self.writer.write(&body).await;
        self.writer.write_raw(&data.target_path).await;

        if common.xattr_idx.is_some() {
            self.writer.write(&common.xattr_idx).await;
        }
    }

    async fn write_basic_device(&mut self, common: &Common, data: &DeviceData) {
        let body = repr::inode::BasicDevice {
            hard_link_count: common.hardlink_count,
            device: data.device,
        };

        self.writer.write(&body).await;
    }

    async fn write_ext_device(&mut self, common: &Common, data: &DeviceData) {
        let body = repr::inode::ExtendedDevice {
            hard_link_count: common.hardlink_count,
            device: data.device,
            xattr_idx: common.xattr_idx,
        };

        self.writer.write(&body).await;
    }

    async fn write_basic_ipc(&mut self, common: &Common) {
        let body = repr::inode::BasicIpc {
            hard_link_count: common.hardlink_count,
        };

        self.writer.write(&body).await;
    }

    async fn write_ext_ipc(&mut self, common: &Common) {
        let body = repr::inode::ExtendedIpc {
            hard_link_count: common.hardlink_count,
            xattr_idx: common.xattr_idx,
        };

        self.writer.write(&body).await;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub common: Common,
    pub data: Data,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Common {
    pub permissions: Mode,
    pub uid_idx: repr::uid_gid::Idx,
    pub gid_idx: repr::uid_gid::Idx,
    pub modified_time: repr::Time,
    pub hardlink_count: u32,
    pub xattr_idx: repr::xattr::Idx,
    /// Force extended type of inode
    pub force_ext: bool,
}

impl Entry {
    fn needs_ext(&self) -> bool {
        if self.common.force_ext || self.common.xattr_idx.is_some() {
            return true;
        }

        match &self.data {
            Data::Directory(data) => {
                let stored_dir_size = repr::inode::dir_stored_size(data.dir_size);
                data.header_locations.is_some() || stored_dir_size > u16::MAX.into()
            }
            Data::File(data) => {
                self.common.hardlink_count > 1
                    || data.blocks_start.0 > u32::MAX.into()
                    || data.file_size > u32::MAX.into()
                    || data.sparse_bytes > 0
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Data {
    Directory(DirData),
    File(FileData),
    Symlink(SymlinkData),
    BlockDev(DeviceData),
    CharDev(DeviceData),
    Fifo,
    Socket,
}

impl Data {
    fn inode_kind(&self, extended: bool) -> repr::inode::Kind {
        use repr::inode::Kind;

        match (self, extended) {
            (Data::Directory(_), false) => Kind::BASIC_DIR,
            (Data::Directory(_), true) => Kind::EXT_DIR,
            (Data::File(_), false) => Kind::BASIC_FILE,
            (Data::File(_), true) => Kind::EXT_FILE,
            (Data::Symlink(_), false) => Kind::BASIC_SYMLINK,
            (Data::Symlink(_), true) => Kind::EXT_SYMLINK,
            (Data::BlockDev(_), false) => Kind::BASIC_BLOCK_DEV,
            (Data::BlockDev(_), true) => Kind::EXT_BLOCK_DEV,
            (Data::CharDev(_), false) => Kind::BASIC_CHAR_DEV,
            (Data::CharDev(_), true) => Kind::EXT_CHAR_DEV,
            (Data::Fifo, false) => Kind::BASIC_FIFO,
            (Data::Fifo, true) => Kind::EXT_FIFO,
            (Data::Socket, false) => Kind::BASIC_SOCKET,
            (Data::Socket, true) => Kind::EXT_SOCKET,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirData {
    pub dir_ref: repr::directory::Ref,
    pub dir_size: u32,
    pub parent_inode_num: repr::inode::Idx,
    pub child_count: u32,
    // TODO: This is the wrong type, need index as if it were uncompressed
    pub header_locations: Option<Vec<repr::directory::Ref>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileData {
    pub blocks_start: repr::datablock::Ref,
    pub file_size: u64,
    pub sparse_bytes: u64,
    pub fragment_block_idx: repr::fragment::Idx,
    pub fragment_offset: u32,
    pub block_sizes: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymlinkData {
    pub target_path: Vec<u8>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DeviceData {
    pub device: repr::inode::DeviceNumber,
}

#[cfg(test)]
mod tests {
    use super::*;
    use repr::inode as raw;
    use std::mem;

    #[test]
    fn add_entries() {
        futures::executor::block_on(async {
            let mut table = Table::new(None);

            let common = Common {
                permissions: Default::default(),
                uid_idx: repr::uid_gid::Idx(0),
                gid_idx: repr::uid_gid::Idx(0),
                modified_time: repr::Time(0),
                hardlink_count: 1,
                xattr_idx: repr::xattr::Idx::default(),
                force_ext: false,
            };
            let entry = Entry {
                common,
                data: Data::Socket,
            };
            table.add(entry).await.unwrap();

            let entry = Entry {
                common,
                data: Data::Symlink(SymlinkData {
                    target_path: b"abcdef".to_vec(),
                }),
            };
            let r = table.add(entry).await.unwrap();
            assert_eq!(r.block_start(), 0);
            // Size of base header + ipc
            assert_eq!(
                r.start_offset() as usize,
                mem::size_of::<raw::Header>() + mem::size_of::<raw::BasicIpc>()
            );

            let entry = Entry {
                common,
                data: Data::File(FileData {
                    blocks_start: repr::datablock::Ref(0),
                    file_size: 10,
                    sparse_bytes: 0,
                    fragment_block_idx: Default::default(),
                    fragment_offset: 0,
                    block_sizes: vec![10],
                }),
            };
            let r = table.add(entry).await.unwrap();
            assert_eq!(r.block_start(), 0);
            // Size of base header + file + 1 block size
            assert_eq!(
                r.start_offset() as usize,
                mem::size_of::<raw::Header>()
                    + mem::size_of::<raw::BasicIpc>()
                    + mem::size_of::<raw::Header>()
                    + mem::size_of::<raw::Symlink>()
                    + 6 // target_path of "abcdef"
            );

            let data = table.finish().await;
            assert_eq!(
                data,
                concat!(
                    "\x56\0\x07\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x01\0\0\0\x03\0\0\0",
                    "\0\0\0\0\0\0\0\0\x01\0\0\0\x01\0\0\0\x06\0\0\0abcdef\x02\0\0",
                    "\0\0\0\0\0\0\0\0\0\x02\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x0A\0\0",
                    "\0\x0A\0\0\0",
                )
                .as_bytes()
            );
        });
    }
}
