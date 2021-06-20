//! Metadata blocks
//!
//! Metadata blocks are compressed in 8KiB blocks. A metadata block is prefixed by a u16 header.
//! The highest bit of the header is set if the block is stored uncompressed (this will happen if
//! the block grew when compressed, or e.g. the [`UNCOMPRESSED_INODES`] superblock flag is set).
//! The lower 15 bits specifies the size of the metadata block (not including the header) on disk.
//!
//! To read a metadata block, read a u16.
//! If the highest bit is set (size & 0x8000 == 0x8000) the following data is uncompressed.
//! Mask out the highest bit to get the size of the block data on disk
//! (this should always be <= 8KiB). Read that many bytes. If the data is compressed,
//! uncompress the data. In pseudocode:
//!
//! ```text
//! header = read_u16(offset=offset)
//! data_size = header & 0x7FFF
//! compressed = header & 0x8000
//! data = read(offset=offset+2, len=data_size)
//! if(compressed) {
//!     data = uncompress(data)
//! }
//! return data
//! ```
//!
//! Neither the size on disk, nor the compressed size should exceed 8KiB. The uncompressed size
//! should always be equal to 8KiB, with the exception of the last metadata block of a section,
//! which may have an uncompressed size less than 8KiB.
//!
//! [`UNCOMPRESSED_INODES`]: ../superblock/struct.Flags.html#associatedconstant.UNCOMPRESSED_INODES

use std::fmt;
use zerocopy::{AsBytes, FromBytes, Unaligned};

pub const SIZE: usize = 8 * 1024;

pub const COMPRESSED_FLAG: u16 = 0x8000;

pub type Metablock = [u8; SIZE];

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, AsBytes, FromBytes, Unaligned)]
#[repr(C, packed)]
pub struct Ref(pub u64);

impl Ref {
    #[inline]
    pub fn new(block_start: u32, offset: u16) -> Self {
        let block_start: u64 = block_start.into();
        let offset: u64 = offset.into();

        Self(block_start << 16 | offset)
    }

    #[inline]
    pub fn block_start(self) -> u32 {
        ((self.0 >> 16) & 0xFFFF_FFFF) as u32
    }

    #[inline]
    pub fn start_offset(self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }
}

impl Default for Ref {
    fn default() -> Self {
        Self(!0)
    }
}

impl fmt::Debug for Ref {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Ref")
            .field("block_start", &self.block_start())
            .field("start_offset", &self.start_offset())
            .finish()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, AsBytes, FromBytes, Unaligned)]
#[repr(C, packed)]
pub struct Header(pub u16);

impl Header {
    pub fn new(size: u16, compressed: bool) -> Self {
        debug_assert!(usize::from(size) <= SIZE);
        Self(size | (if compressed { COMPRESSED_FLAG } else { 0 }))
    }

    pub fn compressed(self) -> bool {
        self.0 & COMPRESSED_FLAG == COMPRESSED_FLAG
    }

    pub fn size(self) -> u16 {
        self.0 & !COMPRESSED_FLAG
    }
}
