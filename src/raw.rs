use bitflags::bitflags;
use packed_serialize::PackedStruct;

pub mod inode;

pub const MAGIC: u32 = 0x7371_7368;

pub const BLOCK_SIZE_MIN: u32 = 4 * 1024;
pub const BLOCK_SIZE_MAX: u32 = 1024 * 1024;

pub const BLOCK_LOG_DEFAULT: u16 = 0x11;
pub const BLOCK_SIZE_DEFAULT: u32 = 0x2_0000;

pub const VERSION_MAJOR: u16 = 4;
pub const VERSION_MINOR: u16 = 0;

bitflags! {
    #[derive(PackedStruct)]
    pub struct Flags: u16 {
        const UNCOMPRESSED_INODES     = 1;
        const UNCOMPRESSED_DATA       = 1 << 1;
        const CHECK                   = 1 << 2;
        const UNCOMPRESSED_FRAGMENTS  = 1 << 3;
        const NO_FRAGMENTS            = 1 << 4;
        const ALWAYS_FRAGMENTS        = 1 << 5;
        const DUPLICATES              = 1 << 6;
        const EXPORTABLE              = 1 << 7;
        const UNCOMPRESSED_XATTRS     = 1 << 8;
        const NO_XATTRS               = 1 << 9;
        const COMPRESSOR_OPTIONS      = 1 << 10;
        const UNCOMPRESSED_IDS        = 1 << 11;
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct InodeRef(pub u64);

impl InodeRef {
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
pub struct CompressionId(pub u16);

impl CompressionId {
    pub const GZIP: CompressionId = CompressionId(1);
    pub const LZMA: CompressionId = CompressionId(2);
    pub const LZO: CompressionId = CompressionId(3);
    pub const XZ: CompressionId = CompressionId(4);
    pub const LZ4: CompressionId = CompressionId(5);
    pub const ZSTD: CompressionId = CompressionId(6);

    pub const MAX: CompressionId = CompressionId::ZSTD;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Superblock {
    pub magic: u32,
    pub inode_count: u32,
    pub modification_time: i32,
    pub block_size: u32,
    pub fragment_entry_count: u32,
    pub compression_id: CompressionId,
    pub block_log: u16,
    pub flags: Flags,
    pub id_count: u16,
    pub version_major: u16,
    pub version_minor: u16,
    pub root_inode_ref: InodeRef,
    pub bytes_used: u64,
    pub id_table_start: u64,
    pub xattr_id_table_start: u64,
    pub inode_table_start: u64,
    pub directory_table_start: u64,
    pub fragment_table_start: u64,
    pub export_table_start: u64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct GzipOptions {
    pub compression_level: u32,
    pub window_size: u16,
    pub strategies: u16,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct XzOptions {
    pub dictionary_size: u32,
    pub executable_filters: u32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Lz4Options {
    pub version: i32,
    pub flags: u32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct ZstdOptions {
    pub compression_level: u32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct MetablockSize(pub u16);

impl MetablockSize {
    pub fn is_compressed(self) -> bool {
        (self.0 & 0x8000) != 0
    }

    pub fn size_on_disk(self) -> u16 {
        self.0 & 0x7FFF
    }
}

use std::fmt;
use std::fmt::Write;

bitflags! {
    #[derive(Default, PackedStruct)]
    pub struct Mode: u16 {
        const OTHER_EXEC =  0o000_001;
        const OTHER_WRITE = 0o000_002;
        const OTHER_READ =  0o000_004;
        const GROUP_EXEC =  0o000_010;
        const GROUP_WRITE = 0o000_020;
        const GROUP_READ =  0o000_040;
        const USER_EXEC =   0o000_100;
        const USER_WRITE =  0o000_200;
        const USER_READ =   0o000_400;
        const BIT_STICKY =  0o001_000;
        const BIT_SGID =    0o002_000;
        const BIT_SUID =    0o004_000;

        const TYPE_FIFO =   0o010_000;
        const TYPE_CHAR =   0o020_000;
        const TYPE_DIR  =   0o040_000;
        const TYPE_BLOCK =  0o060_000;
        const TYPE_FILE =   0o100_000;
        const TYPE_LINK =   0o120_000;
        const TYPE_SOCKET = 0o140_000;

    }
}

impl Mode {
    pub const PERM_MASK: Mode = Mode { bits: 0o007_777 };
    pub const TYPE_MASK: Mode = Mode { bits: 0o170_000 };
    pub const NONE: Mode = Mode { bits: 0 };
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let type_char = match *self & Mode::TYPE_MASK {
            Mode::TYPE_DIR => 'd',
            Mode::TYPE_CHAR => 'c',
            Mode::TYPE_BLOCK => 'b',
            Mode::TYPE_FILE => '-',
            Mode::TYPE_LINK => 'l',
            Mode::TYPE_SOCKET => 's',
            Mode::TYPE_FIFO => 'p',
            _ => '?',
        };
        let user_r = if self.contains(Mode::USER_READ) {
            'r'
        } else {
            '-'
        };
        let user_w = if self.contains(Mode::USER_WRITE) {
            'w'
        } else {
            '-'
        };
        let user_x = match *self & (Mode::USER_EXEC | Mode::BIT_SUID) {
            Mode::NONE => '-',
            Mode::USER_EXEC => 'x',
            Mode::BIT_SUID => 'S',
            _ => 's',
        };

        let group_r = if self.contains(Mode::GROUP_READ) {
            'r'
        } else {
            '-'
        };
        let group_w = if self.contains(Mode::GROUP_WRITE) {
            'w'
        } else {
            '-'
        };
        let group_x = match *self & (Mode::GROUP_EXEC | Mode::BIT_SGID) {
            Mode::NONE => '-',
            Mode::GROUP_EXEC => 'x',
            Mode::BIT_SGID => 'S',
            _ => 's',
        };

        let other_r = if self.contains(Mode::OTHER_READ) {
            'r'
        } else {
            '-'
        };
        let other_w = if self.contains(Mode::OTHER_WRITE) {
            'w'
        } else {
            '-'
        };
        let other_x = match *self & (Mode::OTHER_EXEC | Mode::BIT_STICKY) {
            Mode::NONE => '-',
            Mode::OTHER_EXEC => 'x',
            Mode::BIT_STICKY => 'T',
            _ => 't',
        };

        f.write_char(type_char)?;
        f.write_char(user_r)?;
        f.write_char(user_w)?;
        f.write_char(user_x)?;
        f.write_char(group_r)?;
        f.write_char(group_w)?;
        f.write_char(group_x)?;
        f.write_char(other_r)?;
        f.write_char(other_w)?;
        f.write_char(other_x)?;

        Ok(())
    }
}
