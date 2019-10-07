use packed_serialize::PackedStruct;
use bitflags::bitflags;

use crate::{compression, inode};

/// The magic constant which marks a squashfs archive
pub const MAGIC: u32 = 0x7371_7368;

/// The supported major version of the squashfs archive metadata
pub const VERSION_MAJOR: u16 = 4;
/// The supported minor version of the squashfs archive metadata
pub const VERSION_MINOR: u16 = 0;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Superblock {
    /// Must match the value of [`MAGIC`](constant.MAGIC.html) (`0x73717368`) to be considered a
    /// squashfs archive
    pub magic: u32,
    /// The number of inodes stored in the inode table
    pub inode_count: u32,
    /// The number of seconds (not counting leap seconds) since 00:00, Jan 1 1970 UTC when the
    /// archive was created (or last appended to). This is *unsigned*, so it expires in the
    /// year 2106 (as opposed to 2038).
    pub modification_time: i32,
    /// The size of a data block in bytes. Must be a power of two between 4096 and 1048576 (1 MiB)
    pub block_size: u32,
    /// The number of entries in the fragment table
    pub fragment_entry_count: u32,
    /// The ID of the compression algorithm used
    pub compression_id: compression::Id,
    /// The log2 of block_size. If block_size and block_log do not agree, the archive is considered
    /// corrupt
    pub block_log: u16,
    /// See [`Flags`](struct.Flags.html)
    pub flags: Flags,
    /// The number of entries in the id lookup table
    pub id_count: u16,
    /// The major version of the squashfs file format. Should always equal
    /// [`VERSION_MAJOR`](constant.VERSION_MAJOR.html) (4)
    pub version_major: u16,
    /// The minor version of the squashfs file format. Should always equal
    /// [`VERSION_MINOR`](constant.VERSION_MINOR.html) (0)
    pub version_minor: u16,
    /// A reference to the inode of the root directory of the archive
    pub root_inode_ref: inode::Ref,
    /// The number of bytes used by the archive. Because squashfs archives are often padded to
    /// 4KiB, this can often be less than the file size
    pub bytes_used: u64,
    /// The byte offset at which the id table starts
    pub id_table_start: u64,
    /// The byte offset at which the xattr id table starts
    pub xattr_id_table_start: u64,
    /// The byte offset at which the inode table starts
    pub inode_table_start: u64,
    /// The byte offset at which the directory table starts
    pub directory_table_start: u64,
    /// The byte offset at which the fragment table starts
    pub fragment_table_start: u64,
    /// The byte offset at which the export table starts
    pub export_table_start: u64,
}

bitflags! {
    #[derive(PackedStruct)]
    pub struct Flags: u16 {
        /// Inodes are stored uncompressed. For backward compatibility reasons, UID/GIDs are also stored uncompressed.
        const UNCOMPRESSED_INODES     = 1;
        /// Data are stored uncompressed
        const UNCOMPRESSED_DATA       = 1 << 1;
        /// Unused in squashfs 4+. Should always be unset
        const CHECK                   = 1 << 2;
        /// Fragments are stored uncompressed
        const UNCOMPRESSED_FRAGMENTS  = 1 << 3;
        /// Fragments are not used. Files smaller than the block size are stored in a full block.
        const NO_FRAGMENTS            = 1 << 4;
        /// If the last block of a file is smaller than the block size, it will be instead stored as a fragment
        const ALWAYS_FRAGMENTS        = 1 << 5;
        /// Identical files are recognized, and stored only once
        const DUPLICATES              = 1 << 6;
        /// Filesystem has support for export via NFS (The export table is populated)
        const EXPORTABLE              = 1 << 7;
        /// Xattrs are stored uncompressed
        const UNCOMPRESSED_XATTRS     = 1 << 8;
        /// Xattrs are not stored
        const NO_XATTRS               = 1 << 9;
        /// The compression options section is present
        const COMPRESSOR_OPTIONS      = 1 << 10;
        /// UID/GIDs are stored uncompressed.
        ///
        /// Note that the UNCOMPRESSED_INODES flag also has this effect.
        /// If that flag is set, this flag has no effect.
        /// This flag is currently only available on master in git, no released version of
        /// squashfs-tools yet supports it.
        const UNCOMPRESSED_IDS        = 1 << 11;
    }
}
