pub const SIZE: usize = 8 * 1024;

pub const COMPRESSED_FLAG: u16 = 0x8000;

pub struct Header(pub u16);

impl Header {
    pub fn compressed(self) -> bool {
        self.0 & COMPRESSED_FLAG == COMPRESSED_FLAG
    }

    pub fn size(self) -> u16 {
        self.0 & !COMPRESSED_FLAG
    }
}
