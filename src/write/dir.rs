use crate::compression::AnyCodec;
use crate::write::metablock_writer::MetablockWriter;
use futures::{Stream, StreamExt};
use std::convert::TryInto;
use std::mem;
use zerocopy::AsBytes;

pub struct DirectoryInfo {
    header_refs: Vec<repr::directory::Ref>,
    uncompressed_size: u32,
}

pub struct Table {
    writer: MetablockWriter,
    // TODO: Is this correct, or should this be u64?
    total_size: usize,
}

impl Table {
    pub fn new(compressor: Option<AnyCodec>) -> Self {
        Self {
            writer: MetablockWriter::new(compressor),
            total_size: 0,
        }
    }

    fn start_dir(&mut self) -> DirBuilder<'_> {
        DirBuilder {
            table: self,
            header: repr::directory::Header {
                count: 0,
                start: !0,
                inode_number: repr::inode::Idx(!0),
            },
            entries: Vec::new(),
            crossed_metablock: false,
        }
    }

    pub async fn dir(&mut self, mut contents: impl Stream<Item = Entry> + Unpin) -> DirectoryInfo {
        let start_size = self.total_size;

        let mut builder = self.start_dir();
        let mut header_refs = Vec::new();

        while let Some(item) = contents.next().await {
            if let Some(header_ref) = builder.add_entry(item) {
                header_refs.push(header_ref);
            }
        }

        builder.flush();

        let end_size = self.total_size;
        DirectoryInfo {
            header_refs,
            uncompressed_size: (end_size - start_size).try_into().unwrap(),
        }
    }

    pub fn finish(self) -> (usize, Vec<u8>) {
        (self.total_size, self.writer.finish())
    }
}

struct DirBuilder<'a> {
    table: &'a mut Table,
    header: repr::directory::Header,
    entries: Vec<u8>,
    crossed_metablock: bool,
}

#[derive(Debug)]
pub struct Entry {
    pub inode: repr::inode::Ref,
    pub inode_num: repr::inode::Idx,
    pub inode_kind: repr::inode::Kind,
    pub name: Vec<u8>,
}

fn inode_diff(ref_num: repr::inode::Idx, i: repr::inode::Idx) -> Option<i16> {
    (i.0 as i32 - ref_num.0 as i32).try_into().ok()
}

/// The minimum inode number reference to use in a header
///
/// This can reference all inode numbers all the way to zero
const MIN_INODE_NUM_REF: repr::inode::Idx = repr::inode::Idx(i16::MIN.unsigned_abs() as u32);
/// The minimum inode number reference to use in a header
///
/// This can reference all inode numbers all the way up to the max inode number
const MAX_INODE_NUM_REF: repr::inode::Idx = repr::inode::Idx(u32::MAX - i16::MAX as u32);

impl DirBuilder<'_> {
    /// Add a dir entry, returning the header pos, if this required a new header
    pub fn add_entry(&mut self, entry: Entry) -> Option<repr::directory::Ref> {
        let need_header = self.crossed_metablock
            || self.header.count >= 256
            || self.header.start != entry.inode.block_start()
            || inode_diff(self.header.inode_number, entry.inode_num).is_none();

        let header_pos = if need_header {
            self.flush();
            self.header.start = entry.inode.block_start();

            // Don't set the reference num lower than a ref num which can go all the way to zero, or higher than one
            // which can go to the max
            self.header.inode_number = entry.inode_num.clamp(MIN_INODE_NUM_REF, MAX_INODE_NUM_REF);
            Some(self.table.writer.position())
        } else {
            None
        };

        let prev_metablock = self.total_size() / repr::metablock::SIZE;
        self.header.count += 1;

        let name_len: u16 = entry.name.len().try_into().unwrap();
        let raw_entry = repr::directory::Entry {
            offset: entry.inode.start_offset(),
            inode_offset: inode_diff(self.header.inode_number, entry.inode_num).unwrap(),
            kind: entry.inode_kind.to_basic(),
            name_size: name_len - 1,
        };

        self.entries.extend_from_slice(raw_entry.as_bytes());
        self.entries.extend_from_slice(&entry.name);

        let current_metablock = self.total_size() / repr::metablock::SIZE;
        if current_metablock != prev_metablock {
            self.crossed_metablock = true;
        }
        header_pos
    }

    fn total_size(&self) -> usize {
        self.table.total_size + mem::size_of_val(&self.header) + self.entries.len()
    }

    fn flush(&mut self) {
        if self.header.count != 0 {
            self.table.total_size = self.total_size();
            self.table.writer.write(&self.header);
            self.table.writer.write_raw(&self.entries);

            self.entries.clear();
            self.header = repr::directory::Header {
                count: 0,
                start: 0,
                inode_number: repr::inode::Idx(0),
            };
            self.crossed_metablock = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        let compressor = crate::compression::AnyCodec::new(crate::compression::Kind::default());
        let mut table = Table::new(Some(compressor));
        let entries = (0..1000).map(|i| Entry {
            inode: repr::inode::Ref::new(i / 100, i as _),
            inode_num: repr::inode::Idx(i * 50),
            inode_kind: repr::inode::Kind::BASIC_FILE,
            name: format!("b{:03}", i).into_bytes(),
        });
        let header_refs = table.dir(futures::stream::iter(entries));

        let (uncompressed_size, data) = table.finish();
        assert!(data.len() < uncompressed_size);
    }

    #[test]
    fn can_reach_min_max() {
        let smallest = MIN_INODE_NUM_REF;
        let zero = repr::inode::Idx(0);
        assert_eq!(inode_diff(smallest, zero), Some(i16::MIN));

        let largest = MAX_INODE_NUM_REF;
        let max = repr::inode::Idx(u32::MAX);
        assert_eq!(inode_diff(largest, max), Some(i16::MAX));
    }
}
