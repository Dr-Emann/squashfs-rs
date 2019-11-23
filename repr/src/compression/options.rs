//! Compression Options
//!
//! If non-default compression options have been used, then these are stored here.

use bitflags::bitflags;
use packed_serialize::PackedStruct;

/// Compression options for the gzip compressor
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Gzip {
    /// Should be in range 1…9 (inclusive). Defaults to 9.
    pub compression_level: u32,
    /// Should be in range 8…15 (inclusive) Defaults to 15.
    pub window_size: u16,
    /// A bitfield describing the enabled strategies.
    ///
    /// See `GzipStrategies`.
    /// If no flags are set, the default strategy is implicitly used.
    pub strategies: u16,
}

impl Default for Gzip {
    fn default() -> Self {
        Self {
            compression_level: 9,
            window_size: 15,
            strategies: Default::default(),
        }
    }
}

bitflags! {
    /// A bitfield describing the enabled strategies.
    ///
    /// If no flags are set, the default strategy is implicitly used.
    #[derive(PackedStruct)]
    pub struct GzipStrategies: u16 {
        const DEFAULT = 0x01;
        const FILTERED = 0x02;
        const HUFFMAN_ONLY = 0x04;
        const RUN_LENGTH_ENCODED = 0x08;
        const FIXED = 0x10;
    }
}

impl Default for GzipStrategies {
    fn default() -> Self {
        GzipStrategies::DEFAULT
    }
}

/// Compression options for the xz compressor
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Xz {
    /// Should be > 8KiB, and must be either the sum of a power of two,
    /// or the sum of two sequential powers of two (2^n or 2^n + 2^(n+1))
    pub dictionary_size: u32,
    /// A bitfield describing the additional enabled filters attempted to
    /// better compress executable code.
    pub executable_filters: XzFilters,
}

bitflags! {
    /// A bitfield describing the additional enabled filters attempted to
    /// better compress executable code.
    #[derive(PackedStruct)]
    pub struct XzFilters: u32 {
        const X86 = 0x01;
        const POWERPC = 0x02;
        const IA64 = 0x04;
        const ARM = 0x08;
        const ARM_THUMB = 0x10;
        const SPARC = 0x20;
    }
}

/// Compression options for the lz4 compressor
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Lz4 {
    /// The only supported value is 1 (`LZ4_LEGACY`)
    pub version: i32,
    /// A bitfield describing the enabled LZ4 flags
    pub flags: Lz4Flags,
}

impl Default for Lz4 {
    fn default() -> Self {
        Self {
            version: 1,
            flags: Default::default(),
        }
    }
}

bitflags! {
    /// A bitfield describing the additional enabled filters attempted to
    /// better compress executable code.
    #[derive(PackedStruct, Default)]
    pub struct Lz4Flags: u32 {
        /// Use LZ4 High Compression(HC) mode
        const HIGH_COMPRESSION = 0x01;
    }
}

/// Compression options for the zstd compressor
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Zstd {
    /// Should be in range 1..22 (inclusive).
    /// The real maximum is the zstd defined `ZSTD_maxCLevel()`
    pub compression_level: u32,
}

/// Compression options for the lzo compressor
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Lzo {
    /// Should be in range 1..22 (inclusive).
    /// The real maximum is the zstd defined `ZSTD_maxCLevel()`
    pub algorithm: LzoAlgorithm,

    /// Compression level
    ///
    /// For lzo1x_999, this can be a value between 0 and 9 (defaults to 8).
    /// Has to be 0 for all other algorithms.
    pub level: u32,
}

impl Default for Lzo {
    fn default() -> Self {
        Self {
            algorithm: Default::default(),
            level: 8,
        }
    }
}

/// Which variant of LZO to use
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct LzoAlgorithm(pub u32);

impl LzoAlgorithm {
    pub const X_1: LzoAlgorithm = LzoAlgorithm(0);
    pub const X_1_11: LzoAlgorithm = LzoAlgorithm(1);
    pub const X_1_12: LzoAlgorithm = LzoAlgorithm(2);
    pub const X_1_15: LzoAlgorithm = LzoAlgorithm(3);
    pub const X_999: LzoAlgorithm = LzoAlgorithm(4);
}

impl Default for LzoAlgorithm {
    fn default() -> Self {
        LzoAlgorithm::X_999
    }
}
