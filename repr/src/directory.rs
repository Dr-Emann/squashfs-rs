use crate::inode;
use packed_serialize::PackedStruct;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Header {
    /// Number of entries following the header
    pub count: u32,
    /// The index of the block in the Inode Table where the inodes is stored
    pub start: u32,
    /// An arbitrary inode number.
    ///
    /// The entries that follow store their inode number as a difference to this.
    /// Typically the inode numbers are allocated in a continuous sequence for all children
    /// of a directory and the header simply stores the first one.
    /// Hard links of course break the sequence and require a new header if they are further
    /// away than +/- 32k of this number. Inode number allocation and picking of the reference
    /// could of course be optimized to prevent this
    pub inode_number: inode::Idx,
}

/// A directory entry
///
/// A directory entry is followed by a string of size `name_size + 1`
///
/// The basic and extended inode types both have a size field that stores the uncompressed size of
/// all the directory entries (including all headers) belonging to the inode.
/// This field is used to deduce if more data is following while iterating over directory entries,
/// even without knowing how many headers and partial lists there will be.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Entry {
    /// An offset into the uncompressed inode metadata block
    pub offset: u16,
    /// The difference of this inode's number to the reference stored in the header
    pub inode_offset: i16,
    /// The inode kind
    ///
    /// **For extended inodes, the corresponding basic type is stored here instead**
    pub kind: inode::Kind,
    /// One less than the size of the entry name
    pub name_size: u16,
}

/// A directory index
///
/// To speed up lookups on directories with lots of entries, the extended directory inode can
/// store an index table, holding the locations of all directory headers and the name of the
/// first entry after the header.
///
/// To allow for fast lookups, a new directory header should be emitted every time the entry list
/// crosses a metadata block boundary.
///
/// A directory index is followed by string name of `name_size + 1` bytes
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Index {
    /// A byte offset from the first directory header to the current header, as if the uncompressed
    /// directory metadata blocks were laid out in memory consecutively.
    pub index: u32,
    /// Start offset of a directory table metadata block
    pub start: u32,
    /// One less than the size of the entry name
    pub name_size: u32,
}
