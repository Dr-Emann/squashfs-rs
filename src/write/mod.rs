mod datablocks;
mod dir;
mod fragments;
mod inode;
mod metablock_writer;
mod two_level;
mod uid_gid;

use chrono::{DateTime, Utc};
use positioned_io::RandomAccessFile;
use std::io::Write;
use std::path::Path;
use std::{fmt, mem, ptr};
use std::{fs, io};

use bstr::BString;

use crate::config::FragmentMode;
use crate::shared_position_file::{Positioned, SharedWriteAt};

use crate::compress_threads::ParallelCompressor;
use crate::compression;
use crate::compression::Compressor;
use crate::errors::Result;
use crate::Mode;
use slog::Logger;
use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryInto;
use std::sync::Arc;
use thread_local::ThreadLocal;
use zerocopy::AsBytes;

const MODE_DEFAULT_DIRECTORY: Mode = Mode::O755;
const MODE_DEFAULT_FILE: Mode = Mode::O644;

pub struct Archive {
    file: Box<dyn SharedWriteAt>,
    mtime: DateTime<Utc>,
    block_size: u32,
    compression: Arc<ParallelCompressor>,

    flags: repr::superblock::Flags,
    items: Vec<Item>,
    root: ItemRef,

    inodes: inode::Table,
    dir_table: dir::Table,
    uid_gids: uid_gid::Table,

    logger: Logger,
}

pub struct SubdirBuilder;

impl SubdirBuilder {
    pub fn begin_dir<S: Into<BString>>(&self, name: S) -> SubdirBuilder {
        self._begin_dir(name.into())
    }

    fn _begin_dir(&self, name: BString) -> SubdirBuilder {
        todo!()
    }

    pub fn done_subdirs(&self) -> DirBuilder {
        todo!()
    }
}

impl Archive {
    pub fn begin_root(&self) -> SubdirBuilder {
        todo!()
    }
}

#[derive(Debug, Clone)]
struct Item {
    uid: repr::uid_gid::Id,
    gid: repr::uid_gid::Id,
    mode: repr::Mode,
    mtime: DateTime<Utc>,

    // TODO: xattrs
    data: Data,
}

impl Item {
    pub(crate) fn kind(&self) -> repr::inode::Kind {
        use repr::inode::Kind;

        match self.data {
            Data::Directory { .. } => Kind::BASIC_DIR,
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ItemRef(usize);

#[derive(Debug, Clone)]
enum Data {
    Symlink { target: BString },
    Directory { entries: BTreeMap<BString, ItemRef> },
    BlockDev(repr::inode::DeviceNumber),
    CharDev(repr::inode::DeviceNumber),
    Fifo,
    Socket,
    // TODO
    File {},
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct BaseData {}

#[derive(Debug)]
pub struct DirBuilder {
    uid: repr::uid_gid::Id,
    gid: repr::uid_gid::Id,
    mode: repr::Mode,
    mtime: DateTime<Utc>,
    entries: BTreeMap<BString, ItemRef>,
    logger: Logger,
}

impl DirBuilder {
    fn new(logger: Logger) -> Self {
        DirBuilder {
            uid: repr::uid_gid::Id(0),
            gid: repr::uid_gid::Id(0),
            mode: MODE_DEFAULT_DIRECTORY,
            mtime: Utc::now(),
            entries: BTreeMap::new(),
            logger,
        }
    }

    pub fn set_uid(&mut self, id: u32) -> &mut Self {
        self.uid = repr::uid_gid::Id(id);
        self
    }

    pub fn set_gid(&mut self, id: u32) -> &mut Self {
        self.gid = repr::uid_gid::Id(id);
        self
    }

    pub fn set_mode(&mut self, mode: crate::Mode) -> &mut Self {
        self.mode = mode;
        self
    }

    pub fn set_modified_time(&mut self, date_time: DateTime<Utc>) -> &mut Self {
        self.mtime = date_time;
        self
    }

    pub fn add_item<S: Into<BString>>(&mut self, name: S, item: ItemRef) -> &mut Self {
        self._add_item(name.into(), item);
        self
    }

    fn _add_item(&mut self, name: BString, item: ItemRef) {
        self.entries.insert(name, item);
    }

    pub fn finish(self, archive: &mut Archive) -> ItemRef {
        // This is safe because self will not be dropped
        let entries = unsafe { ptr::read(&self.entries) };
        let item = Item {
            uid: self.uid,
            gid: self.gid,
            mode: self.mode,
            mtime: self.mtime,
            data: Data::Directory { entries },
        };
        mem::forget(self);

        archive.add_item(item)
    }
}

impl Drop for DirBuilder {
    fn drop(&mut self) {
        slog::warn!(
            self.logger,
            "Leaking directory builder containing {:?}",
            self.entries.keys().collect::<Vec<_>>()
        );
    }
}

pub struct FileBuilder {
    uid: repr::uid_gid::Id,
    gid: repr::uid_gid::Id,
    mode: repr::Mode,
    mtime: DateTime<Utc>,
    contents: Box<dyn io::Read>,
}

impl FileBuilder {
    pub fn set_uid(&mut self, id: u32) -> &mut Self {
        self.uid = repr::uid_gid::Id(id);
        self
    }

    pub fn set_gid(&mut self, id: u32) -> &mut Self {
        self.gid = repr::uid_gid::Id(id);
        self
    }

    pub fn set_mode(&mut self, mode: crate::Mode) -> &mut Self {
        self.mode = mode;
        self
    }

    pub fn set_modified_time(&mut self, date_time: DateTime<Utc>) -> &mut Self {
        self.mtime = date_time;
        self
    }

    pub fn set_contents(&mut self, contents: Box<dyn io::Read>) -> &mut Self {
        self.contents = contents;
        self
    }

    pub fn finish(self, archive: &mut Archive) -> ItemRef {
        todo!()
    }
}

impl Archive {
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        ArchiveBuilder::new().build_path(path)
    }

    pub fn from_writer(writer: Box<dyn SharedWriteAt>) -> Self {
        ArchiveBuilder::new().build(writer)
    }

    pub fn create_dir(&mut self) -> DirBuilder {
        DirBuilder::new(self.logger.clone())
    }

    pub fn create_file(&self) -> FileBuilder {
        todo!()
    }

    fn get(&self, item_ref: ItemRef) -> &Item {
        &self.items[item_ref.0]
    }

    fn get_mut(&mut self, item_ref: ItemRef) -> &mut Item {
        &mut self.items[item_ref.0]
    }

    fn add_item(&mut self, item: Item) -> ItemRef {
        self.uid_gids.add(item.uid);
        self.uid_gids.add(item.gid);

        let item_ref = ItemRef(self.items.len());
        self.items.push(item);
        item_ref
    }

    pub fn set_root(&mut self, item_ref: ItemRef) {
        assert!(matches!(self.get(item_ref).data, Data::Directory { .. }));
        self.root = item_ref;
    }

    pub fn flush(&mut self) -> Result<()> {
        let mut superblock = repr::superblock::Superblock {
            magic: repr::superblock::MAGIC,
            inode_count: self.items.len().try_into().expect("too many items"),
            modification_time: date_time_to_mtime(self.mtime, &self.logger),
            block_size: self.block_size,
            fragment_entry_count: 0,                     // TODO
            compression_id: repr::compression::Id::GZIP, // TODO
            block_log: self.block_size.trailing_zeros() as _,
            flags: self.flags,
            id_count: self.uid_gids.len(),
            version_major: repr::superblock::VERSION_MAJOR,
            version_minor: repr::superblock::VERSION_MINOR,
            root_inode_ref: repr::inode::Ref::default(), // TODO
            bytes_used: 0,
            id_table_start: u64::MAX,
            xattr_id_table_start: u64::MAX,
            inode_table_start: u64::MAX,
            directory_table_start: u64::MAX,
            fragment_table_start: u64::MAX,
            export_table_start: u64::MAX,
        };
        // TODO: data blocks? compression_options
        superblock.inode_table_start = mem::size_of_val(&superblock).try_into().unwrap();

        let mut writer = Positioned::with_position(&*self.file, superblock.inode_table_start);

        self.file.write_all_at(&superblock.as_bytes(), 0)?;
        unimplemented!();
    }
}

impl Drop for Archive {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

impl fmt::Debug for Archive {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Archive")
            .field("items", &self.items)
            .field("root", &self.root)
            .field("uid_gid", &self.uid_gids)
            .field("mtime", &self.mtime)
            .field("block_size", &self.block_size)
            .field("compression", &self.compression)
            .field("flags", &self.flags)
            .finish()
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
    pub compressor_kind: compression::Kind,

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
            compressor_kind: compression::Kind::default(),
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

        let modification_time = date_time_to_mtime(self.modified_time, &logger);

        let compression = Arc::new(ParallelCompressor::new(Compressor::new(
            self.compressor_kind,
        )));
        let compression_or_none = |use_compressor: bool| {
            if use_compressor {
                Some(Arc::clone(&compression))
            } else {
                None
            }
        };
        let inodes = inode::Table::new(compression_or_none(self.compressed_inodes));
        let dir_table = dir::Table::new(compression_or_none(self.compressed_inodes));
        let uid_gids = uid_gid::Table::new();
        Archive {
            file: writer,
            mtime: self.modified_time,
            block_size: self.block_size,
            compression,
            root: ItemRef(usize::MAX),
            inodes,
            dir_table,
            uid_gids,
            items: Vec::new(),

            flags: repr::superblock::Flags::default(),
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

fn date_time_to_mtime(date_time: DateTime<Utc>, logger: &Logger) -> repr::Time {
    let mtime = date_time.timestamp();
    let underlying_time = if mtime > u32::MAX.into() {
        slog::warn!(logger, "Modification time is out of range for squashfs"; "date" => %date_time);
        u32::MAX
    } else if mtime < u32::MIN.into() {
        slog::warn!(logger, "Modification time is out of range for squashfs"; "date" => %date_time);
        u32::MIN
    } else {
        mtime as u32
    };
    repr::Time(underlying_time)
}
