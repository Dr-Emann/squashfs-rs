//! Fragment Table
//!
//! Fragments are combined into fragment blocks of at most block_size bytes long. This table
//! describes the location and size of these fragment blocks, not the fragments within them.
//!
//! This table is stored in two levels: The fragment block entries are stored in metadata blocks,
//! and the file offsets to these metadata blocks are stored at the offset specified by the
//! `fragment_table_start` field of the superblock.
//!
//! Each metadata block can store 512 fragment block entries (16 bytes per fragment block entry),
//! so there will be `ceil(fragment_entry_count / 512.0)` metadata blocks (and the same number of
//! `u64` offsets stored at `fragment_table_start`)
//!
//! To read the list of fragment block entries, read `ceil(fragment_entry_count / 512.0)` `u64`
//! offsets starting at `fragment_table_start`, then read the metadata blocks at the offsets read,
//! interpreting the data of the metadata blocks as a packed array of fragment block entries.

use zerocopy::{AsBytes, FromBytes, Unaligned};

/// Fragment block entry
#[derive(Debug, Copy, Clone, PartialEq, Eq, AsBytes, FromBytes, Unaligned)]
#[repr(C, packed)]
pub struct Entry {
    /// The offset within the archive where the fragment block starts
    pub start: u64,
    /// This stores two pieces of information
    ///
    /// If the block is uncompressed, the `0x1000000` (`1<<24`) bit wil be set. The remaining bits
    /// describe the size of the fragment block on disk. Because the max value of block_size is
    /// 1 MiB (`1<<20`), and the size of a fragment block should be less than `block_size`, the
    /// uncompressed bit will never be set by the size.
    pub size: Size,
    /// This field is unused
    pub _unused: u32,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, AsBytes, FromBytes, Unaligned)]
#[repr(C, packed)]
pub struct Idx(pub u32);

pub use crate::datablock::Size;
