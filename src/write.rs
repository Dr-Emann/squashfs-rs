use chrono::{DateTime, Utc};
use packed_serialize::PackedStruct;
use positioned_io::RandomAccessFile;
use std::fmt;
use std::fs::File;
use std::io;
use std::path::Path;

use crate::config::FragmentMode;
use crate::shared_position_file::SharedWriteAt;

use crate::CompressionType;
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(e) => fmt::Display::fmt(e, f),
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

struct SharedArchive {
    id_table: Vec<u32>,
}

pub struct Archive {
    file: Box<dyn SharedWriteAt>,
    shared: Arc<Mutex<SharedArchive>>,
    modified_time: Option<DateTime<Utc>>,
}

pub struct Directory<'a> {
    archive: &'a Archive,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub block_size: u32,
    pub xattrs: bool,
    pub compressed_inodes: bool,
    pub compressed_data: bool,
    pub compressed_fragments: bool,
    pub compressed_xattrs: bool,
    pub compressed_ids: bool,
    pub find_duplicates: bool,
    pub exportable: bool,
    pub fragment_mode: FragmentMode,
    pub compressor: CompressionType,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            block_size: repr::BLOCK_SIZE_DEFAULT,
            xattrs: true,
            compressed_inodes: true,
            compressed_data: true,
            compressed_fragments: true,
            compressed_xattrs: true,
            compressed_ids: true,
            find_duplicates: true,
            exportable: true,
            fragment_mode: FragmentMode::default(),
            compressor: CompressionType::default(),
        }
    }
}

impl Archive {
    pub fn create<P: AsRef<Path>>(path: P, config: &Config) -> Result<Self> {
        Self::_create(path.as_ref(), config)
    }

    fn _create(path: &Path, config: &Config) -> Result<Self> {
        config.validate();
        let file = RandomAccessFile::try_new(File::create(path)?)?;
        Ok(Self::from_writer_unchecked(Box::new(file), config))
    }

    pub fn from_writer(writer: Box<dyn SharedWriteAt>, config: &Config) -> Self {
        config.validate();
        Self::from_writer_unchecked(writer, config)
    }

    fn from_writer_unchecked(writer: Box<dyn SharedWriteAt>, config: &Config) -> Self {
        Self {
            file: writer,
            modified_time: None,
            shared: Arc::new(Mutex::new(SharedArchive {
                id_table: Vec::new(),
            })),
        }
    }

    pub fn root_dir(&self) -> Directory {
        unimplemented!()
    }

    pub fn flush(self) -> Result<()> {
        let mod_time = self.modified_time.unwrap_or_else(|| Utc::now());
        let superblock = repr::superblock::Superblock {
            magic: repr::superblock::MAGIC,
            inode_count: 1,
            modification_time: mod_time.timestamp() as i32,
            block_size: repr::BLOCK_SIZE_DEFAULT,
            fragment_entry_count: 0,
            compression_id: repr::compression::Id::GZIP,
            block_log: repr::BLOCK_LOG_DEFAULT,
            flags: repr::superblock::Flags::empty(),
            id_count: 0,
            version_major: repr::superblock::VERSION_MAJOR,
            version_minor: repr::superblock::VERSION_MINOR,
            root_inode_ref: repr::inode::Ref(0),
            bytes_used: repr::superblock::Superblock::size() as u64,
            id_table_start: repr::superblock::Superblock::size() as u64,
            xattr_id_table_start: !0,
            inode_table_start: !0,
            directory_table_start: !0,
            fragment_table_start: !0,
            export_table_start: !0,
        };

        self.file.write_all_at(&superblock.to_packed(), 0)?;

        Ok(())
    }
}

impl Config {
    fn validate(&self) {
        if self.block_size < repr::BLOCK_SIZE_MIN
            || self.block_size > repr::BLOCK_SIZE_MAX
            || !self.block_size.is_power_of_two()
        {
            panic!(
                "block size must be a power of two between {} and {}",
                repr::BLOCK_SIZE_MIN,
                repr::BLOCK_SIZE_MAX
            );
        }
    }

    pub fn new() -> Self {
        Default::default()
    }
}

impl fmt::Debug for Archive {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Archive").finish()
    }
}
