use std::fmt;
use zerocopy::{AsBytes, FromBytes, Unaligned};

/// The max size of a datablock: 1 MiB
pub const MAX_SIZE: usize = 1024 * 1024;

#[derive(Copy, Clone, PartialEq, Eq, AsBytes, FromBytes, Unaligned)]
#[repr(C, packed)]
pub struct Size(pub u32);

impl Size {
    pub const UNCOMPRESSED_FLAG: u32 = 1 << 24;
    pub const ZERO: Size = Size(0);

    pub fn new(mut size: u32, uncompressed: bool) -> Self {
        assert!(size <= (1 << 20));
        if uncompressed {
            size |= Self::UNCOMPRESSED_FLAG;
        }
        Self(size)
    }

    pub fn size(self) -> u32 {
        self.0 & !Self::UNCOMPRESSED_FLAG
    }

    pub fn uncompressed(self) -> bool {
        self.0 & Self::UNCOMPRESSED_FLAG != 0
    }
}

impl fmt::Debug for Size {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Size")
            .field("size", &self.size())
            .field("uncompressed", &self.uncompressed())
            .finish()
    }
}

/// Number of bytes from the start of the archive where the block starts
#[derive(Debug, Copy, Clone, PartialEq, Eq, AsBytes, FromBytes, Unaligned)]
#[repr(C, packed)]
pub struct Ref(pub u64);
