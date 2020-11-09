use chrono::{DateTime, Utc};
use packed_serialize::PackedStruct;
use positioned_io::RandomAccessFile;
use std::fs;
use std::io;
use std::path::Path;
use std::{cmp, fmt};

use bstr::{BStr, BString};

use crate::config::FragmentMode;
use crate::shared_position_file::SharedWriteAt;

use crate::compression::Kind;
use crate::errors::{Error, Result};
use crate::Mode;
use slog::Logger;
use std::collections::BTreeMap;
use std::convert::TryInto;

const MODE_DEFAULT_DIRECTORY: Mode = Mode::O755;
const MODE_DEFAULT_FILE: Mode = Mode::O644;

pub struct Archive {
    file: Box<dyn SharedWriteAt>,
    superblock: repr::superblock::Superblock,
    logger: Logger,
}

struct Entry {
    name: BString,
    item: Item,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct BaseData {
    uid: u32,
    gid: u32,
    mode: repr::Mode,
    mtime: DateTime<Utc>,
    inode_number: repr::inode::Idx,
}

#[derive(Debug)]
pub struct Item {
    inode_ref: repr::inode::Ref,
    kind: repr::inode::Kind,
    logger: Logger,
    attached: bool,
}

impl Drop for Item {
    fn drop(&mut self) {
        if !self.attached {
            slog::warn!(self.logger, "Dropping unattached item");
        }
    }
}

#[derive(Debug)]
pub struct DirBuilder<'a> {
    archive: &'a mut Archive,
    base: BaseData,
    entries: BTreeMap<BString, Item>,
    built: bool,
}

impl<'a> DirBuilder<'a> {
    fn new(archive: &'a mut Archive) -> Self {
        DirBuilder {
            archive,
            base: BaseData {
                uid: 0,
                gid: 0,
                mode: MODE_DEFAULT_DIRECTORY,
                mtime: Utc::now(),
                inode_number: repr::inode::Idx(0),
            },
            entries: BTreeMap::new(),
            built: false,
        }
    }

    pub fn set_uid(&mut self, id: u32) -> &mut Self {
        self.base.uid = id;
        self
    }

    pub fn set_gid(&mut self, id: u32) -> &mut Self {
        self.base.gid = id;
        self
    }

    pub fn set_mode(&mut self, mode: crate::Mode) -> &mut Self {
        self.base.mode = mode;
        self
    }

    pub fn set_modified_time(&mut self, date_time: DateTime<Utc>) -> &mut Self {
        self.base.mtime = date_time;
        self
    }

    pub fn add_item<S: Into<BString>, I: Into<Item>>(&mut self, name: S, item: I) -> &mut Self {
        self._add_item(name.into(), item.into());
        self
    }

    fn _add_item(&mut self, name: BString, item: Item) {
        self.entries.insert(name, item);
    }

    pub fn build(mut self) -> Item {
        let idx = self.archive.next_inode_idx();
        self.base.inode_number = idx;
        self.built = true;
        unimplemented!()
    }
}

impl Drop for DirBuilder<'_> {
    fn drop(&mut self) {
        if !self.built {
            slog::warn!(
                self.archive.logger,
                "Leaking directory builder containing {:?}",
                self.entries
            );
        }
    }
}

impl Archive {
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        ArchiveBuilder::new().build_path(path)
    }

    pub fn from_writer(writer: Box<dyn SharedWriteAt>) -> Self {
        ArchiveBuilder::new().build(writer)
    }

    pub fn create_dir(&mut self) -> DirBuilder<'_> {
        unimplemented!()
    }

    fn next_inode_idx(&mut self) -> repr::inode::Idx {
        let idx = repr::inode::Idx(self.superblock.inode_count);
        self.superblock.inode_count += 1;
        idx
    }

    pub fn set_root(&mut self, item: Item) {
        assert_eq!(item.kind, repr::inode::Kind::BASIC_DIR);
        self.superblock.root_inode_ref = item.inode_ref;
    }

    pub fn flush(&mut self) -> Result<()> {
        self.file.write_all_at(&self.superblock.to_packed(), 0)?;

        Ok(())
    }
}

impl Drop for Archive {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

impl fmt::Debug for Archive {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Archive").finish()
    }
}

#[derive(Debug, Clone)]
pub struct ArchiveBuilder {
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
    pub compressor: Kind,

    modified_time: DateTime<Utc>,
    logger: Option<Logger>,
}

impl Default for ArchiveBuilder {
    fn default() -> Self {
        ArchiveBuilder {
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
            compressor: Kind::default(),
            modified_time: Utc::now(),
            logger: None,
        }
    }
}

impl ArchiveBuilder {
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

    pub fn set_modification_time(&mut self, time: DateTime<Utc>) -> &mut Self {
        self.modified_time = time;
        self
    }

    pub fn set_logger(&mut self, logger: Logger) -> &mut Self {
        self.logger = Some(logger);
        self
    }

    pub fn build(self, writer: Box<dyn SharedWriteAt>) -> Archive {
        self.validate();

        let logger = self.logger.unwrap_or_else(crate::default_logger);

        let mut modification_time = self.modified_time.timestamp();
        if modification_time < 0 || modification_time > u32::max_value().into() {
            slog::warn!(logger, "modification time is unrepresentable"; "time" => %self.modified_time);
            modification_time = cmp::max(cmp::min(modification_time, u32::max_value().into()), 0);
        }
        let modification_time = modification_time as u32;

        let superblock = repr::superblock::Superblock {
            magic: repr::superblock::MAGIC,
            inode_count: 0,
            modification_time,
            block_size: repr::BLOCK_SIZE_DEFAULT,
            fragment_entry_count: 0,
            compression_id: repr::compression::Id::GZIP,
            block_log: repr::BLOCK_LOG_DEFAULT,
            flags: 0,
            id_count: 0,
            version_major: repr::superblock::VERSION_MAJOR,
            version_minor: repr::superblock::VERSION_MINOR,
            root_inode_ref: repr::inode::Ref(0),
            bytes_used: repr::superblock::Superblock::SIZE as u64,
            id_table_start: repr::superblock::Superblock::SIZE as u64,
            xattr_id_table_start: !0,
            inode_table_start: !0,
            directory_table_start: !0,
            fragment_table_start: !0,
            export_table_start: !0,
        };
        Archive {
            file: writer,
            superblock,
            logger,
        }
    }

    pub fn build_path<P: AsRef<Path>>(self, path: P) -> Result<Archive> {
        self._build_path(path.as_ref())
    }

    fn _build_path(mut self, path: &Path) -> Result<Archive> {
        let logger = self.logger.take().unwrap_or_else(crate::default_logger);
        let path_str = path.display().to_string();
        self.logger = Some(logger.new(slog::o!("file" => path_str)));

        let file = RandomAccessFile::try_new(fs::File::create(path)?)?;
        Ok(self.build(Box::new(file)))
    }
}
