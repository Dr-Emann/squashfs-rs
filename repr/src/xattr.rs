//! Xattr Table
//!
//! Extended attributes are arbitrary key value pairs attached to inodes. The key names use dots as
//! separators to create a hierarchy of namespaces.
//!
//! Squashfs uses multiple levels of indirection to store Xattr key value pairs associated with
//! inodes. To saves space, the topmost namespace prefix is removed and encoded as an integer ID
//! instead. This approach limits squashfs xattr support to the following, commonly used namespaces:
//!
//! ```text
//! 0 - user.
//! 1 - trusted.
//! 2 - security.
//! ```
//!
//! This means that on the one hand squashfs can store SELinux labels or capabilities since those
//! are stored in the `security.*` namespaces, but cannot store ACLs which are stored in
//! system.posix_acl_access because it has no way to encode the system. prefix yet.
//!
//! The key value pairs of all inodes are stored consecutively in metadata blocks. The values can
//! be either be stored inline, i.e. an Xattr Key Entry is directly followed by an
//! Xattr Value Entry, or out of line to deduplicate identical values.
//!
//! If a value is stored out of line, the value entry structure holds a 64 bit reference instead of
//! a string that specifies the location of the value string, similar to an inode reference, but
//! relative to the the first metadata block containing the key value pairs.
//!
//! Typically, the first occurrence of a value is stored in line and every consecutive use of the
//! same value uses an out of line value to refer back to the first one.

use zerocopy::{AsBytes, FromBytes, Unaligned};

/// An xattr key
///
/// Followed by a name string of size `name_size`
#[derive(Debug, Copy, Clone, PartialEq, Eq, AsBytes, FromBytes, Unaligned)]
#[repr(C, packed)]
pub struct Key {
    /// The ID of the key prefix
    ///
    /// If the value that follows is stored out of line, the flag `Kind::OUT_OF_LINE` is ORed to the type ID
    pub kind: Kind,
    /// The size of the key name **including** the omitted prefix but excluding the trailing null byte
    pub name_size: u16,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, AsBytes, FromBytes, Unaligned)]
#[repr(C, packed)]
pub struct Kind(pub u16);

impl Kind {
    pub const USER: Kind = Kind(0);
    pub const TRUSTED: Kind = Kind(1);
    pub const SECURITY: Kind = Kind(2);

    pub const OUT_OF_LINE: Kind = Kind(0x0100);

    pub fn out_of_line(self) -> bool {
        self.0 & Kind::OUT_OF_LINE.0 != 0
    }

    pub fn prefix(self) -> Kind {
        Kind(self.0 & !Kind::OUT_OF_LINE.0)
    }
}

/// An xattr value
///
/// If `Kind::OUT_OF_LINE` is set, the value is stored out of line:
///   * `value_size` will always be 8
///   * The following 8 bytes should be interpreted as a 64 bit reference that specifies the
///     location of the value string, similar to an inode reference, but relative to the the first
///     metadata block containing the key value pairs.
/// If the value is not stored out of line, the structure is followed by `value_size` bytes of data
#[derive(Debug, Copy, Clone, PartialEq, Eq, AsBytes, FromBytes, Unaligned)]
#[repr(C, packed)]
pub struct Value {
    /// The size of the value string
    ///
    /// If the value is stored out of line, this is always 8, i.e. the size of an unsigned 64 bit integer
    pub value_size: u32,
}

/// The header on the Xattr Lookup Table
///
/// In order to locate xattrs on disk, an approach similar to ID and fragment tables is used.
/// The following data structure is stored directly on in the archive (i.e. uncompressed and without additional headers).
///
/// The xattr_id_table_start in the superblock stores the absolute position of this table.
///
/// The table is followed by u64 locations of metadata blocks.
/// There will be `ceil(xattr_entry_count * sizeof(LookupEntry) / metablock_size)` items
#[derive(Debug, Copy, Clone, PartialEq, Eq, AsBytes, FromBytes, Unaligned)]
#[repr(C, packed)]
pub struct LookupTable {
    /// The absolute position of the first metadata block holding the key/value pairs.
    pub xattr_table_start: u64,
    /// The number of entries in the Xattr Lookup Table
    pub xattr_entry_count: u32,
    /// Unused
    pub _unused: u32,
}

/// A Lookup Table Entry
///
/// To actually address a block of key value pairs associated with an inode, a lookup table is used
/// that specifies the start and size of a block of key value pairs.
///
/// All an inode needs to store is a 32 bit index into this table.
/// If two inodes have the identical xattrs (e.g. they have the same SELinux labels and no other
/// attributes), the key/value block is only written once, there is only one lookup table entry and
/// both inodes have the same index.
#[derive(Debug, Copy, Clone, PartialEq, Eq, AsBytes, FromBytes, Unaligned)]
#[repr(C, packed)]
pub struct LookupEntry {
    /// A reference to the start of the key value block
    pub xattr_ref: Ref,
    /// The number of key value pairs
    pub count: u32,
    /// The exact, uncompressed size in bytes of the entire block of key value pairs
    ///
    /// This counts only what has been written to disk and including the key/value entry structures
    pub size: u32,
}

pub use crate::metablock::Ref;

/// References the entry with the `i`th index in the Xattr Id Table
#[derive(Debug, Copy, Clone, PartialEq, Eq, AsBytes, FromBytes, Unaligned)]
#[repr(C, packed)]
pub struct Idx(pub u32);
