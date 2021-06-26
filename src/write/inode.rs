use super::metablock_writer::MetablockWriter;
use crate::compress_threads::ParallelCompressor;
use crate::Mode;
use std::convert::TryInto;
use std::io;
use std::sync::Arc;

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

        unimplemented!()
    }

    async fn write_basic_dir(&mut self, common: &Common, data: &DirData) -> io::Result<()> {
        let body = repr::inode::BasicDir {
            dir_block_start: data.dir_ref.block_start(),
            // Note that for historical reasons, the hard link count of a directory includes
            // the number of entries in the directory and is initialized to 2 for an empty
            // directory. I.e. a directory with N entries has at least N + 2 link count.
            hard_link_count: repr::inode::dir_hardlink_count(
                common.hardlink_count,
                data.child_count,
            ),
            // Safe because we should never write a basic dir if we can avoid it
            file_size: data.dir_size.try_into().unwrap(),
            block_offset: data.dir_ref.start_offset(),
            parent_inode_number: data.parent_inode_num,
        };

        self.writer.write(&body).await
    }

    async fn write_ext_dir(&mut self, common: &Common, data: &DirData) -> io::Result<()> {
        let body = repr::inode::ExtendedDir {
            hard_link_count: repr::inode::dir_hardlink_count(
                common.hardlink_count,
                data.child_count,
            ),
            file_size: data.dir_size,
            dir_block_start: data.dir_ref.block_start(),
            parent_inode_number: data.parent_inode_num,
            index_count: data
                .header_locations
                .as_ref()
                .map_or(0, |locations| locations.len().try_into().unwrap()),
            block_offset: data.dir_ref.start_offset(),
            xattr_idx: common.xattr_idx,
        };

        self.writer.write(&body).await
    }
}

pub struct Entry {
    pub common: Common,
    pub data: Data,
}

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
                data.header_locations.is_some() || data.dir_size > u16::MAX.into()
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

pub struct DirData {
    pub dir_ref: repr::directory::Ref,
    pub dir_size: u32,
    pub parent_inode_num: repr::inode::Idx,
    pub child_count: u32,
    pub header_locations: Option<Vec<repr::directory::Ref>>,
}

pub struct FileData {
    pub blocks_start: repr::datablock::Ref,
    pub file_size: u64,
    pub sparse_bytes: u64,
    pub fragment_block_idx: repr::fragment::Idx,
    pub fragment_offset: u32,
    pub block_sizes: Vec<u32>,
}

pub struct SymlinkData {
    pub target_path: Vec<u8>,
}

pub struct DeviceData {
    pub device: repr::inode::DeviceNumber,
}
