use repr::compression::Id as CompressionId;
use std::{fmt, io};

#[cfg(feature = "gzip")]
pub mod gzip;

#[cfg(feature = "zstd")]
pub mod zstd;

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

#[derive(Debug)]
pub struct Codec<C: CodecImpl> {
    config: C::Config,
    comp: C::Compressor,
    decomp: C::Decompressor,
}

impl<C: CodecImpl> Codec<C> {
    fn new() -> Self {
        Self {
            config: Default::default(),
            comp: C::compressor(Default::default()),
            decomp: C::decompressor(Default::default()),
        }
    }

    fn configured(data: &[u8]) -> io::Result<Self> {
        let config = C::read_config(data)?;
        Ok(Self::with_config(config))
    }

    fn with_config(config: C::Config) -> Self {
        Self {
            config: config.clone(),
            comp: C::compressor(config.clone()),
            decomp: C::decompressor(config),
        }
    }
}

impl<C: CodecImpl> Compressor for Codec<C> {
    fn compress(&mut self, src: &[u8], dst: &mut [u8]) -> io::Result<usize> {
        self.comp.compress(src, dst)
    }
}

impl<C: CodecImpl> Decompressor for Codec<C> {
    fn decompress(&mut self, src: &[u8], dst: &mut [u8]) -> io::Result<usize> {
        self.decomp.decompress(src, dst)
    }
}

impl<C: CodecImpl> Clone for Codec<C>
where
    C::Config: Clone,
{
    fn clone(&self) -> Self {
        let config = self.config.clone();
        Self {
            config: config.clone(),
            comp: C::compressor(config.clone()),
            decomp: C::decompressor(config),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AnyCodec {
    #[cfg(feature = "gzip")]
    Gzip(Codec<gzip::Gzip>),
    #[cfg(feature = "zstd")]
    Zstd(Codec<zstd::Zstd>),
}

impl AnyCodec {
    pub fn new(kind: Kind) -> AnyCodec {
        match kind {
            #[cfg(feature = "gzip")]
            Kind::ZLib => AnyCodec::Gzip(Codec::new()),
            #[cfg(feature = "zstd")]
            Kind::Zstd => AnyCodec::Zstd(Codec::new()),
            _ => panic!("Unsupported compressor kind {}", kind),
        }
    }

    pub fn configured(kind: Kind, data: &[u8]) -> io::Result<Self> {
        let result = match kind {
            #[cfg(feature = "gzip")]
            Kind::ZLib => AnyCodec::Gzip(Codec::configured(data)?),
            #[cfg(feature = "zstd")]
            Kind::Zstd => AnyCodec::Zstd(Codec::configured(data)?),
            _ => panic!("Unsupported compressor kind {}", kind),
        };
        Ok(result)
    }

    pub fn config(&self) -> &dyn fmt::Debug {
        match self {
            #[cfg(feature = "gzip")]
            AnyCodec::Gzip(codec) => &codec.config,
            #[cfg(feature = "zstd")]
            AnyCodec::Zstd(codec) => &codec.config,
        }
    }

    pub fn compress(&mut self, src: &[u8], dst: &mut [u8]) -> io::Result<usize> {
        match self {
            #[cfg(feature = "gzip")]
            AnyCodec::Gzip(gzip) => gzip.comp.compress(src, dst),
            #[cfg(feature = "zstd")]
            AnyCodec::Zstd(zstd) => zstd.comp.compress(src, dst),
        }
    }

    pub fn decompress(&mut self, src: &[u8], dst: &mut [u8]) -> io::Result<usize> {
        match self {
            #[cfg(feature = "gzip")]
            AnyCodec::Gzip(gzip) => gzip.decomp.decompress(src, dst),
            #[cfg(feature = "zstd")]
            AnyCodec::Zstd(zstd) => zstd.decomp.decompress(src, dst),
        }
    }

    pub fn kind(&self) -> Kind {
        match *self {
            #[cfg(feature = "gzip")]
            AnyCodec::Gzip(_) => Kind::ZLib,
            #[cfg(feature = "zstd")]
            AnyCodec::Zstd(_) => Kind::Zstd,
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

pub trait Compressor {
    fn compress(&mut self, src: &[u8], dst: &mut [u8]) -> io::Result<usize>;
}

pub trait Decompressor {
    fn decompress(&mut self, src: &[u8], dst: &mut [u8]) -> io::Result<usize>;
}

pub trait Config: fmt::Debug + Default + Clone {
    fn set(&mut self, field: &str, value: &str) -> io::Result<()>;

    fn key_values(&self) -> Vec<(&'static str, ConfigValue<'_>)>;
}

pub enum ConfigValue<'a> {
    Str(&'a str),
    String(String),
    Int(i64),
}

pub trait CodecImpl {
    type Compressor: Compressor;
    type Decompressor: Decompressor;
    type Config: Config;

    fn read_config(data: &[u8]) -> io::Result<Self::Config>;
    fn compressor(config: Self::Config) -> Self::Compressor;
    fn decompressor(config: Self::Config) -> Self::Decompressor;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip<C: CodecImpl>() {
        let mut c = Codec::<C>::new();
        let src: &[u8] = b"11111111111111111111111111111111111c111";
        let mut dest = [0; 64];
        let mut clear_dest = vec![0u8; src.len()];
        let dest_size = c.compress(src, &mut dest).expect("compression");
        let clear_size = c
            .decompress(&dest[..dest_size], &mut clear_dest)
            .expect("decompression");
        assert_eq!(&src[..], &clear_dest[..clear_size]);
    }

    fn small_dst<C: CodecImpl>() {
        let mut c = Codec::<C>::new();
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
        round_trip::<gzip::Gzip>();
        small_dst::<gzip::Gzip>();
    }

    #[cfg(feature = "zstd")]
    #[test]
    fn zstd_compressor() {
        round_trip::<zstd::Zstd>();
        small_dst::<zstd::Zstd>();
    }
}
