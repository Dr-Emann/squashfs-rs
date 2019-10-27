//! Directory Table
//!
//! Directory listings, including file names, and references to inodes
//!
//! For each directory inode, the directory table stores a list of all entries stored inside, with
//! references back to the inodes that describe those entries.
//
//The entry list is self is sorted ASCIIbetically by entry name. To save space, a delta encoding is
// used to store the inode number, i.e. the list is preceded by a header with a reference inode
// number and all entries store the difference to that. Furthermore, the header also includes the
// location of a metadata block that the inodes of all of the following entries are in.
// The entries just store an offset into the uncompressed metadata block.

use crate::inode;
use packed_serialize::PackedStruct;

/// A header which precedes a list of directory entries
///
///Every time, the inode block changes or the difference of the inode number cannot be encoded in
/// 16 bits anymore, a new header is emitted.
///
///A header must not be followed by more than 256 entries. If there are more entries,
/// a new header is emitted.
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
///
///The file names are stored without trailing null bytes. Since a zero length name makes no sense,
/// the name length is stored off-by-one, i.e. the value 0 cannot be encoded
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
