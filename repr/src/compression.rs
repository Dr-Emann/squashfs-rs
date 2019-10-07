use packed_serialize::PackedStruct;

pub mod options;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Id(pub u16);

impl Id {
    pub const GZIP: Id = Id(1);
    pub const LZMA: Id = Id(2);
    pub const LZO: Id = Id(3);
    pub const XZ: Id = Id(4);
    pub const LZ4: Id = Id(5);
    pub const ZSTD: Id = Id(6);

    pub const MAX: Id = Id::ZSTD;
}

