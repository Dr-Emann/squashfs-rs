//! A squashfs filesystem consists of a maximum of nine parts, packed together on a byte alignment:
//!
//! * [Superblock](superblock/index.html)
//! * [Compression Options](compression/options/index.html)
//! * [Datablocks & Fragments]
//! * [Inode Table](inode/index.html)
//! * [Directory Table](directory/index.html)
//! * [Fragment Table](fragment/index.html)
//! * [Export Table]
//! * [UID/GID Lookup Table](uid_gid/index.html)
//! * [Xattr Table](xattr/index.html)

use bitflags::bitflags;
use packed_serialize::PackedStruct;

use std::fmt;
use std::fmt::Write;

pub mod compression;
pub mod directory;
pub mod fragment;
pub mod inode;
pub mod metablock;
pub mod superblock;
pub mod uid_gid;
pub mod xattr;

pub const BLOCK_LOG_MIN: u16 = 12;
pub const BLOCK_LOG_MAX: u16 = 20;
pub const BLOCK_LOG_DEFAULT: u16 = 17;

pub const BLOCK_SIZE_MIN: u32 = 1 << BLOCK_LOG_MIN as u32;
pub const BLOCK_SIZE_MAX: u32 = 1 << BLOCK_LOG_MAX as u32;
pub const BLOCK_SIZE_DEFAULT: u32 = 1 << BLOCK_LOG_DEFAULT as u32;

/// The header stored before a metadata block
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct MetablockHeader(pub u16);

impl MetablockHeader {
    /// Return true if the following block is compressed
    pub fn is_compressed(self) -> bool {
        (self.0 & 0x8000) != 0
    }

    /// The size in bytes (on disk) of the following metadata block
    pub fn size_on_disk(self) -> u16 {
        self.0 & 0x7FFF
    }
}

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
    pub const O777: Mode = Mode { bits: 0o000_777 };
    pub const O755: Mode = Mode { bits: 0o000_755 };
    pub const O644: Mode = Mode { bits: 0o000_644 };
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
            // Both
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
            // Both
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
            // Both
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

#[test]
fn mode_tests() {
    let mode = Mode { bits: 0o754 } | Mode::TYPE_FILE;
    assert_eq!(&format!("{}", mode), "-rwxr-xr--");
    let mode = mode | Mode::BIT_STICKY;
    assert_eq!(&format!("{}", mode), "-rwxr-xr-T");
}
