use repr::compression::Id as CompressionId;
use std::{fmt, io};

#[cfg(feature = "gzip")]
pub mod gzip;

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

trait Compress: Default {
    fn configure(&mut self, options: &[u8]) -> io::Result<()>;

    fn compress(&self, src: &[u8], dst: &mut [u8]) -> io::Result<usize>;
    fn decompress(&self, src: &[u8], dst: &mut [u8]) -> io::Result<usize>;
}

#[derive(Debug)]
pub enum Compressor {
    #[cfg(feature = "gzip")]
    Gzip(gzip::Gzip),
}

impl Compressor {
    pub fn configure(&mut self, options: &[u8]) -> io::Result<()> {
        match *self {
            #[cfg(feature = "gzip")]
            Compressor::Gzip(ref mut gzip) => gzip.configure(options),
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

    pub fn compressor(self) -> Compressor {
        match self {
            Kind::ZLib => Compressor::Gzip(Default::default()),
            Kind::Lzma | Kind::Lzo | Kind::Xz | Kind::Lz4 | Kind::Zstd | Kind::Unknown => {
                unimplemented!()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gzip_compressor() {
        let c = gzip::Gzip::default();
        let src: &[u8] = b"11111111111111111111111111111111111c111";
        let mut dest = [0; 64];
        let mut clear_dest = vec![0u8; src.len()];
        let dest_size = c.compress(src, &mut dest).expect("compression");
        let clear_size = c
            .decompress(&dest[..dest_size], &mut clear_dest)
            .expect("decompression");
        assert_eq!(&src[..], &clear_dest[..clear_size]);
    }
}
