//! Inode Table
//!
//! Metadata (ownership, permissions, etc) for items in the archive

use crate::{uid_gid, xattr};
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
pub struct Idx(pub u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Kind(pub u16);

impl Kind {
    /// Following the header is a [`BasicDir`](struct.BasicDir.html) structure
    pub const BASIC_DIR: Kind = Kind(1);
    /// Following the header is a [`BasicFile`](struct.BasicFile.html) structure
    pub const BASIC_FILE: Kind = Kind(2);
    /// Following the header is a [`Symlink`](struct.Symlink.html) structure
    pub const BASIC_SYMLINK: Kind = Kind(3);
    /// Following the header is a [`BasicDevice`](struct.BasicDevice.html) structure
    pub const BASIC_BLOCK_DEV: Kind = Kind(4);
    /// Following the header is a [`BasicDevice`](struct.BasicDevice.html) structure
    pub const BASIC_CHAR_DEV: Kind = Kind(5);
    /// Following the header is a [`BasicIpc`](struct.BasicIpc.html) structure
    pub const BASIC_FIFO: Kind = Kind(6);
    /// Following the header is a [`BasicIpc`](struct.BasicIpc.html) structure
    pub const BASIC_SOCKET: Kind = Kind(7);

    /// Following the header is a [`ExtendedDir`](struct.ExtendedDir.html) structure
    pub const EXT_DIR: Kind = Kind(8);
    /// Following the header is a [`ExtendedFile`](struct.ExtendedFile.html) structure
    pub const EXT_FILE: Kind = Kind(9);
    /// Following the header is a [`Symlink`](struct.Symlink.html) structure
    pub const EXT_SYMLINK: Kind = Kind(10);
    /// Following the header is a [`ExtendedDevice`](struct.ExtendedDevice.html) structure
    pub const EXT_BLOCK_DEV: Kind = Kind(11);
    /// Following the header is a [`ExtendedDevice`](struct.ExtendedDevice.html) structure
    pub const EXT_CHAR_DEV: Kind = Kind(12);
    /// Following the header is a [`ExtendedIpc`](struct.ExtendedIpc.html) structure
    pub const EXT_FIFO: Kind = Kind(13);
    /// Following the header is a [`ExtendedIpc`](struct.ExtendedIpc.html) structure
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
    pub uid_idx: uid_gid::Idx,
    /// The index of the group id in the UID/GID Table
    pub gid_idx: uid_gid::Idx,
    /// The unsigned number of seconds (not counting leap seconds) since 00:00, Jan 1 1970 UTC
    /// when the item described by the inode was last modified
    pub modified_time: u32,
    /// The position of this inode in the full list of inodes.
    /// Value should be in the range `[1, inode_count]` (inclusive)
    /// This can be treated as a unique identifier for this inode, and can be
    /// used as a key to recreate hard links: when processing the archive,
    /// remember the visited values of inode_number. If an inode number has
    /// already been visited, this inode is hardlinked
    pub inode_number: Idx,
}

/// A basic directory inode structure
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct BasicDir {
    /// The index of the block in the Directory Table where the directory entry information starts
    pub block_idx: u32,
    /// The number of hard links to this directory
    pub hard_link_count: u32,
    /// Total (uncompressed) size in bytes of the entries in the Directory Table, including headers
    pub file_size: u16,
    /// The (uncompressed) offset within the block in the Directory Table where the directory entry
    /// information starts
    pub block_offset: u16,
    /// The inode_number of the parent of this directory. If this is the root directory, this will be 1
    pub parent_inode_number: Idx,
}

/// A full extended directory inode structure
///
/// This inode is followed by `index_count + 1` directory index entries for faster
/// lookup in the directory table
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct ExtendedDir {
    /// The number of hard links to this directory
    pub hard_link_count: u32,
    /// Total (uncompressed) size in bytes of the entries in the Directory Table, including headers
    pub file_size: u32,
    /// The index of the block in the Directory Table where the directory entry information starts
    pub block_idx: u32,
    /// The inode_number of the parent of this directory. If this is the root directory, this will be 1
    pub parent_inode_number: Idx,
    /// The number of directory index entries following the inode structure
    pub index_count: u16,
    /// The (uncompressed) offset within the block in the Directory Table where the directory entry
    /// information starts
    pub block_offset: u16,
    /// An index into the xattr lookup table. Set to 0xFFFFFFFF if the inode has no extended attributes
    pub xattr_idx: xattr::Idx,
}

/// A basic file inode structure
///
/// This inode is followed by a list of `u32` block sizes.
/// If this file ends in a fragment, the size of this list is the number of full data blocks
/// needed to store file_size bytes. If this file does not have a fragment, the size of the list is
/// the number of blocks needed to store file_size bytes, rounded up. Each item in the list
/// describes the (possibly compressed) size of a block.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct BasicFile {
    /// The offset from the start of the archive where the data blocks are stored
    pub blocks_start: u32,
    /// The index of a fragment entry in the fragment table which describes the data block the
    /// fragment of this file is stored in.
    ///
    /// If this file does not end with a fragment, this should be 0xFFFFFFFF
    pub fragment_block_index: u32,
    /// The (uncompressed) offset within the fragment data block where the fragment for this file.
    ///
    /// Information about the fragment can be found at fragment_block_index.
    /// The size of the fragment can be found as `file_size % superblock.block_size`.
    /// If this file does not end with a fragment, the value of this field is undefined (probably zero)
    pub block_offset: u32,
    /// The (uncompressed) size of this file
    pub file_size: u32,
}

/// A full extended file inode structure
///
/// This inode is followed by a list of `u32` block sizes.
/// If this file ends in a fragment, the size of this list is the number of full data blocks
/// needed to store file_size bytes. If this file does not have a fragment, the size of the list is
/// the number of blocks needed to store file_size bytes, rounded up. Each item in the list
/// describes the (possibly compressed) size of a block.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct ExtendedFile {
    /// The offset from the start of the archive where the data blocks are stored
    pub blocks_start: u64,
    /// The (uncompressed) size of this file
    pub file_size: u64,
    /// The number of bytes saved by omitting blocks of zero bytes.
    /// Used in the kernel for sparse file accounting
    pub sparse: u64,
    /// The number of hard links to this node
    pub hard_link_count: u32,
    /// The index of a fragment entry in the fragment table which describes the data block the
    /// fragment of this file is stored in.
    ///
    /// If this file does not end with a fragment, this should be 0xFFFFFFFF
    pub fragment_block_index: u32,
    /// The (uncompressed) offset within the fragment data block where the fragment for this file.
    ///
    /// Information about the fragment can be found at fragment_block_index.
    /// If this file does not end with a fragment,
    /// the value of this field is undefined (probably zero)
    pub block_offset: u32,
    /// An index into the xattr lookup table.
    ///
    /// Set to `0xFFFFFFFF` if the inode has no extended attributes
    pub xattr_idx: xattr::Idx,
}

/// A symlink inode structure
///
/// This inode is followed by a path string `target_bytes` long.
/// The path string may not contain any null characters.
/// If the header had a kind `EXT_SYMLINK`, the path string is followed by an xattr_idx u32, which
/// is an index into the xattr lookup table. Set to 0xFFFFFFFF if the inode has no extended attributes
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Symlink {
    /// The number of hard links to this symlink
    pub hard_link_count: u32,
    /// The size in bytes of the target path string following this inode
    /// which describes the target of this symlink
    pub target_size: u32,
}

/// A basic device inode structure
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct BasicDevice {
    /// The number of hard links to this device
    pub hard_link_count: u32,
    /// The device represented
    pub device: DeviceNumber,
}

/// A full extended device inode structure
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct ExtendedDevice {
    /// The number of hard links to this device
    pub hard_link_count: u32,
    /// The device represented
    pub device: DeviceNumber,
    /// An index into the xattr lookup table. Set to 0xFFFFFFFF if the inode has no extended attributes
    pub xattr_idx: xattr::Idx,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct DeviceNumber(pub u32);

impl DeviceNumber {
    pub fn new(major: u32, minor: u32) -> Self {
        assert!(major <= 0x0_0FFF);
        assert!(minor <= 0xF_FFFF);
        DeviceNumber(major << 8 | minor & 0xFF | (minor & !0xFF) << 12)
    }

    pub fn major(self) -> u32 {
        (self.0 & 0xfff00) >> 8
    }

    pub fn minor(self) -> u32 {
        (self.0 & 0xff) | ((self.0 >> 12) & 0xfff00)
    }
}

/// A basic IPC (fifo/socket) inode structure
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct BasicIpc {
    /// The number of hard links to this device
    pub hard_link_count: u32,
}

/// A full extended IPC (fifo/socket) inode structure
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct ExtendedIpc {
    /// The number of hard links to this device
    pub hard_link_count: u32,
    /// An index into the xattr lookup table. Set to 0xFFFFFFFF if the inode has no extended attributes
    pub xattr_idx: xattr::Idx,
}
