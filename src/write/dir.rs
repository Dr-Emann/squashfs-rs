use crate::compress_threads::ParallelCompressor;
use crate::write::metablock_writer::MetablockWriter;
use futures::{Stream, StreamExt};
use std::convert::TryInto;
use std::sync::Arc;
use std::{cmp, io, mem};
use zerocopy::AsBytes;

pub struct Table {
    writer: MetablockWriter,
    total_size: usize,
}

impl Table {
    pub fn new(compressor: Option<Arc<ParallelCompressor>>) -> Self {
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

    pub async fn dir(
        &mut self,
        mut contents: impl Stream<Item = Entry> + Unpin,
    ) -> io::Result<Vec<repr::directory::Ref>> {
        let mut builder = self.start_dir();
        let mut header_refs = Vec::new();

        while let Some(item) = contents.next().await {
            if let Some(header_ref) = builder.add_entry(item).await? {
                header_refs.push(header_ref);
            }
        }

        builder.flush().await?;
        Ok(header_refs)
    }

    pub async fn finish(self) -> io::Result<(usize, Vec<u8>)> {
        Ok((self.total_size, self.writer.finish().await?))
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
const MIN_INODE_NUM_REF: repr::inode::Idx = repr::inode::Idx(i16::MIN.unsigned_abs() as _);

impl DirBuilder<'_> {
    /// Add a dir entry, returning the header pos, if this required a new header
    pub async fn add_entry(&mut self, entry: Entry) -> io::Result<Option<repr::directory::Ref>> {
        let need_header = self.crossed_metablock
            || self.header.count >= 256
            || self.header.start != entry.inode.block_start()
            || inode_diff(self.header.inode_number, entry.inode_num).is_none();

        let header_pos = if need_header {
            self.flush().await?;
            self.header.start = entry.inode.block_start();

            // Don't set the reference num lower than a ref num which can go all the way to zero
            self.header.inode_number = cmp::max(entry.inode_num, MIN_INODE_NUM_REF);
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
        Ok(header_pos)
    }

    fn total_size(&self) -> usize {
        self.table.total_size + mem::size_of_val(&self.header) + self.entries.len()
    }

    async fn flush(&mut self) -> io::Result<()> {
        if self.header.count != 0 {
            self.table.total_size = self.total_size();
            self.table.writer.write(&self.header).await?;
            self.table.writer.write_raw(&self.entries).await?;

            self.entries.clear();
            self.header = repr::directory::Header {
                count: 0,
                start: 0,
                inode_number: repr::inode::Idx(0),
            };
            self.crossed_metablock = false;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        let compressor = ParallelCompressor::new(
            1,
            crate::compression::Compressor::new(crate::compression::Kind::default()),
        );
        let mut table = Table::new(Some(Arc::new(compressor)));
        futures::executor::block_on(async {
            let entries = (0..1000).map(|i| Entry {
                inode: repr::inode::Ref::new(i / 100, i as _),
                inode_num: repr::inode::Idx(i * 50),
                inode_kind: repr::inode::Kind::BASIC_FILE,
                name: format!("b{:03}", i).into_bytes(),
            });
            let header_refs = table.dir(futures::stream::iter(entries)).await.unwrap();

            let (uncompressed_size, data) = table.finish().await.unwrap();
            assert!(data.len() < uncompressed_size);
        });
    }
}
