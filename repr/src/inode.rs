use packed_serialize::PackedStruct;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Ref(pub u64);

impl Ref {
    #[inline]
    pub fn block_idx(self) -> u32 {
        ((self.0 >> 16) & 0xFFFF_FFFF) as u32
    }

    #[inline]
    pub fn start_offset(self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }
}


#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Kind(pub u16);

impl Kind {
    pub const BASIC_DIR: Kind = Kind(1);
    pub const BASIC_FILE: Kind = Kind(2);
    pub const BASIC_SYMLINK: Kind = Kind(3);
    pub const BASIC_BLOCK_DEV: Kind = Kind(4);
    pub const BASIC_CHAR_DEV: Kind = Kind(5);
    pub const BASIC_FIFO: Kind = Kind(6);
    pub const BASIC_SOCKET: Kind = Kind(7);

    pub const EXT_DIR: Kind = Kind(8);
    pub const EXT_FILE: Kind = Kind(9);
    pub const EXT_SYMLINK: Kind = Kind(10);
    pub const EXT_BLOCK_DEV: Kind = Kind(11);
    pub const EXT_CHAR_DEV: Kind = Kind(12);
    pub const EXT_FIFO: Kind = Kind(13);
    pub const EXT_SOCKET: Kind = Kind(14);

    pub const MAX: Kind = Kind::EXT_SOCKET;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Header {
    /// The type of item described by the inode which follows this header
    pub inode_type: Kind,
    /// A bitmask representing the permissions for the item described by the inode.
    /// The values match with the permission values of mode_t (the mode bits, not the file type)
    pub permissions: super::Mode,
    /// The index of the user id in the UID/GID Table
    pub uid_idx: u16,
    /// The index of the group id in the UID/GID Table
    pub gid_idx: u16,
    /// The number of seconds (not counting leap seconds) since
    /// 00:00, Jan 1 1970 UTC when the item described by the inode was last modified
    pub modified_time: i32,
    /// The position of this inode in the full list of inodes.
    /// Value should be in the range `[1, inode_count]` (inclusive)
    /// This can be treated as a unique identifier for this inode, and can be
    /// used as a key to recreate hard links: when processing the archive,
    /// remember the visited values of inode_number. If an inode number has
    /// already been visited, this inode is hardlinked
    pub inode_number: u32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct BasicDir {
    /// The common header for inodes
    pub header: Header,
    /// The index of the block in the Directory Table where the directory entry information starts
    pub block_idx: u32,
    /// The number of hard links to this directory
    pub hard_link_count: u32,
    /// TODO: Check meaning
    pub file_size: u16,
    /// The (uncompressed) offset within the block in the Directory Table where the directory entry
    /// information starts
    pub block_offset: u16,
    /// The inode_number of the parent of this directory. If this is the root directory, this will be 1
    pub parent_inode_number: u32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct ExtendedDir {
    /// The common header for inodes
    pub header: Header,
    /// The number of hard links to this directory
    pub hard_link_count: u32,
    /// TODO: Check meaning
    pub file_size: u32,
    /// The index of the block in the Directory Table where the directory entry information starts
    pub block_idx: u32,
    /// The inode_number of the parent of this directory. If this is the root directory, this will be 1
    pub parent_inode_number: u32,
    /// The number of directory indexes. TODO: More Info. May be related to exports?
    pub index_count: u16,
    /// The (uncompressed) offset within the block in the Directory Table where the directory entry
    /// information starts
    pub block_offset: u16,
    /// A reference to inode information in the xattr table.
    /// TODO: More info after learning about the xattr table
    pub xattr_idx: u32,
}
