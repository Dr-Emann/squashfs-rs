//! Metadata blocks are compressed in 8KiB blocks. A metadata block is prefixed by a u16 header.
//! The highest bit of the header is set if the block is stored uncompressed (this will happen if
//! the block grew when compressed, or e.g. the `UNCOMPRESSED_INODES` superblock flag is set).
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

use packed_serialize::PackedStruct;

pub const SIZE: usize = 8 * 1024;

pub const COMPRESSED_FLAG: u16 = 0x8000;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Header(pub u16);

impl Header {
    pub fn compressed(self) -> bool {
        self.0 & COMPRESSED_FLAG == COMPRESSED_FLAG
    }

    pub fn size(self) -> u16 {
        self.0 & !COMPRESSED_FLAG
    }
}
