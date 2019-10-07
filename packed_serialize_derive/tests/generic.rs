use packed_serialize::PackedStruct;

#[test]
fn embedded() {
    #[derive(Copy, Clone, Debug, PackedStruct, PartialEq, Eq)]
    pub struct Inner(u64);

    #[derive(Copy, Clone, Debug, PackedStruct, PartialEq, Eq)]
    pub struct Outer {
        inner1: Inner,
        x: u8,
        inner2: Inner,
    }

    let outer = Outer {
        inner1: Inner(0x11121314_15161718),
        x: 0xFF,
        inner2: Inner(0x21222324_25262728),
    };

    assert_eq!(
        outer.to_packed().as_slice(),
        &[
            0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12, 0x11, 0xFF, 0x28, 0x27, 0x26, 0x25, 0x24,
            0x23, 0x22, 0x21,
        ][..]
    );

    let outer_2 = Outer::from_packed(&outer.to_packed());
    assert_eq!(outer, outer_2);
}

#[test]
fn large_struct() {
    #[derive(Debug, PackedStruct, PartialEq, Eq)]
    pub struct Superblock {
        magic: u32,
        inode_count: u32,
        modification_time: i32,
        block_size: u32,
        fragment_entry_count: u32,
        compression_id: u16,
        block_log: u16,
        flags: u16,
        id_count: u16,
        version_major: u16,
        version_minor: u16,
        root_inode_ref: u64,
        bytes_used: u64,
        id_table_start: u64,
        xattr_id_table_start: u64,
        inode_table_start: u64,
        directory_table_start: u64,
        fragment_table_start: u64,
        export_table_start: u64,
    }
    use packed_serialize::generic_array::typenum::Unsigned;
    assert_eq!(
        <<Superblock as PackedStruct>::Size as Unsigned>::to_usize(),
        96
    );
    let superblock = Superblock {
        magic: 0x01020304,
        inode_count: 0x11121314,
        modification_time: 0x21222324,
        block_size: 0x31323334,
        fragment_entry_count: 0x41424344,
        compression_id: 0x5152,
        block_log: 0x5354,
        flags: 0x6162,
        id_count: 0x6364,
        version_major: 0x7172,
        version_minor: 0x7374,
        root_inode_ref: 0x81828384_85868788,
        bytes_used: 0x91929394_95969798,
        id_table_start: 0xA1A2A3A4_A5A6A7A8,
        xattr_id_table_start: 0xB1B2B3B4_B5B6B7B8,
        inode_table_start: 0xC1C2C3C4_C5C6C7C8,
        directory_table_start: 0xD1D2D3D4_D5D6D7D8,
        fragment_table_start: 0xE1E2E3E4_E5E6E7E8,
        export_table_start: 0xF1F2F3F4_F5F6F7F8,
    };
    assert_eq!(
        superblock.to_packed().as_slice(),
        &[
            0x04, 0x03, 0x02, 0x01, 0x14, 0x13, 0x12, 0x11, 0x24, 0x23, 0x22, 0x21, 0x34, 0x33,
            0x32, 0x31, 0x44, 0x43, 0x42, 0x41, 0x52, 0x51, 0x54, 0x53, 0x62, 0x61, 0x64, 0x63,
            0x72, 0x71, 0x74, 0x73, 0x88, 0x87, 0x86, 0x85, 0x84, 0x83, 0x82, 0x81, 0x98, 0x97,
            0x96, 0x95, 0x94, 0x93, 0x92, 0x91, 0xA8, 0xA7, 0xA6, 0xA5, 0xA4, 0xA3, 0xA2, 0xA1,
            0xB8, 0xB7, 0xB6, 0xB5, 0xB4, 0xB3, 0xB2, 0xB1, 0xC8, 0xC7, 0xC6, 0xC5, 0xC4, 0xC3,
            0xC2, 0xC1, 0xD8, 0xD7, 0xD6, 0xD5, 0xD4, 0xD3, 0xD2, 0xD1, 0xE8, 0xE7, 0xE6, 0xE5,
            0xE4, 0xE3, 0xE2, 0xE1, 0xF8, 0xF7, 0xF6, 0xF5, 0xF4, 0xF3, 0xF2, 0xF1,
        ][..]
    );

    let superblock_2 = Superblock::from_packed(&superblock.to_packed());
    assert_eq!(superblock, superblock_2);
}
