use repr::compression::Id as CompressionId;
use std::{fmt, io};

#[cfg(feature = "gzip")]
pub mod gzip;
#[cfg(feature = "gzip")]
use self::gzip::Gzip;

#[cfg(feature = "zstd")]
pub mod zstd;
#[cfg(feature = "zstd")]
use self::zstd::Zstd;

#[repr(u16)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    ZLib = CompressionId::GZIP.0,
    Lzma = CompressionId::LZMA.0,
    Lzo = CompressionId::LZO.0,
    Xz = CompressionId::XZ.0,
    Lz4 = CompressionId::LZ4.0,
    Zstd = CompressionId::ZSTD.0,
    Unknown = 0,
}

#[derive(Debug, Clone)]
pub enum Compressor {
    #[cfg(feature = "gzip")]
    Gzip(gzip::Gzip),
    #[cfg(feature = "zstd")]
    Zstd(zstd::Zstd),
}

impl Compressor {
    pub fn new(kind: Kind) -> Compressor {
        match kind {
            #[cfg(feature = "gzip")]
            Kind::ZLib => Compressor::Gzip(Default::default()),
            #[cfg(feature = "zstd")]
            Kind::Zstd => Compressor::Zstd(Default::default()),
            _ => panic!("Unsupported compressor kind {}", kind),
        }
    }

    pub fn configured(kind: Kind, data: &[u8]) -> io::Result<Self> {
        let result = match kind {
            #[cfg(feature = "gzip")]
            Kind::ZLib => Compressor::Gzip(Gzip::configured(data)?),
            #[cfg(feature = "zstd")]
            Kind::Zstd => Compressor::Zstd(Zstd::configured(data)?),
            _ => panic!("Unsupported compressor kind {}", kind),
        };
        Ok(result)
    }

    pub fn config(&self) -> &dyn fmt::Debug {
        match self {
            #[cfg(feature = "gzip")]
            Compressor::Gzip(gzip) => gzip.config(),
            #[cfg(feature = "zstd")]
            Compressor::Zstd(zstd) => zstd.config(),
        }
    }

    pub fn compress(&mut self, src: &[u8], dst: &mut [u8]) -> io::Result<usize> {
        match self {
            #[cfg(feature = "gzip")]
            Compressor::Gzip(gzip) => gzip.compress(src, dst),
            #[cfg(feature = "zstd")]
            Compressor::Zstd(zstd) => zstd.compress(src, dst),
        }
    }

    pub fn decompress(&mut self, src: &[u8], dst: &mut [u8]) -> io::Result<usize> {
        match self {
            #[cfg(feature = "gzip")]
            Compressor::Gzip(gzip) => gzip.decompress(src, dst),
            #[cfg(feature = "zstd")]
            Compressor::Zstd(zstd) => zstd.decompress(src, dst),
        }
    }

    pub fn kind(&self) -> Kind {
        match *self {
            #[cfg(feature = "gzip")]
            Compressor::Gzip(_) => Kind::ZLib,
            #[cfg(feature = "zstd")]
            Compressor::Zstd(_) => Kind::Zstd,
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl Default for Kind {
    fn default() -> Self {
        Kind::ZLib
    }
}

impl Kind {
    pub fn from_name(name: &str) -> Kind {
        match name {
            "gzip" => Kind::ZLib,
            "lzma" => Kind::Lzma,
            "lzo" => Kind::Lzo,
            "xz" => Kind::Xz,
            "lz4" => Kind::Lz4,
            "zstd" => Kind::Zstd,
            _ => Kind::Unknown,
        }
    }

    pub fn from_id(id: CompressionId) -> Kind {
        match id {
            CompressionId::GZIP => Kind::ZLib,
            CompressionId::LZMA => Kind::Lzma,
            CompressionId::LZO => Kind::Lzo,
            CompressionId::XZ => Kind::Xz,
            CompressionId::LZ4 => Kind::Lz4,
            CompressionId::ZSTD => Kind::Zstd,
            _ => Kind::Unknown,
        }
    }

    pub fn id(self) -> u16 {
        self as u16
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::ZLib => "gzip",
            Kind::Lzma => "lzma",
            Kind::Lzo => "lzo",
            Kind::Xz => "xz",
            Kind::Lz4 => "lz4",
            Kind::Zstd => "zstd",
            Kind::Unknown => "unknown",
        }
    }

    pub fn supported(self) -> bool {
        match self {
            Kind::ZLib => cfg!(feature = "gzip"),
            Kind::Lzma => cfg!(feature = "lzma"),
            Kind::Lzo => cfg!(feature = "lzo"),
            Kind::Xz => cfg!(feature = "xz"),
            Kind::Lz4 => cfg!(feature = "lz4"),
            Kind::Zstd => cfg!(feature = "zstd"),
            Kind::Unknown => false,
        }
    }
}

trait Codec: Default {
    type Config: fmt::Debug;

    fn with_config(config: Self::Config) -> Self
    where
        Self: Sized;

    fn configured(options: &[u8]) -> io::Result<Self>
    where
        Self: Sized;

    fn compress(&mut self, src: &[u8], dst: &mut [u8]) -> io::Result<usize>;
    fn decompress(&mut self, src: &[u8], dst: &mut [u8]) -> io::Result<usize>;

    fn config(&self) -> &Self::Config;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip<C: Codec>() {
        let mut c = C::default();
        let src: &[u8] = b"11111111111111111111111111111111111c111";
        let mut dest = [0; 64];
        let mut clear_dest = vec![0u8; src.len()];
        let dest_size = c.compress(src, &mut dest).expect("compression");
        let clear_size = c
            .decompress(&dest[..dest_size], &mut clear_dest)
            .expect("decompression");
        assert_eq!(&src[..], &clear_dest[..clear_size]);
    }

    fn small_dst<C: Codec>() {
        let mut c = C::default();
        let src: &[u8] = b"11111111111111111111111111111111111c111";
        let mut dest = [0; 1];
        c.decompress(src, &mut dest)
            .expect_err("cannot compress to 1 bytes");

        let src: &[u8] = b"11111111111111111111111111111111111c111";
        let mut dest = [0; 1];
        c.compress(src, &mut dest)
            .expect_err("cannot compress to 1 bytes");
    }

    #[cfg(feature = "gzip")]
    #[test]
    fn gzip_compressor() {
        round_trip::<Gzip>();
        small_dst::<Gzip>();
    }

    #[cfg(feature = "zstd")]
    #[test]
    fn zstd_compressor() {
        round_trip::<Zstd>();
        small_dst::<Zstd>();
    }
}
