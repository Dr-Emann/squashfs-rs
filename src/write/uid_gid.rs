use crate::compress_threads::ParallelCompressor;
use crate::write::metablock_writer::MetablockWriter;
use crate::write::two_level;
use byteorder::{LittleEndian, WriteBytesExt};
use std::collections::HashMap;
use std::io;
use std::sync::Arc;

pub struct Table {
    ids: Vec<repr::uid_gid::Id>,
    known_ids: HashMap<repr::uid_gid::Id, repr::uid_gid::Idx>,
}

impl Table {
    pub fn new() -> Self {
        Self {
            ids: Vec::new(),
            known_ids: HashMap::new(),
        }
    }

    pub fn add(&mut self, id: repr::uid_gid::Id) -> repr::uid_gid::Idx {
        if let Some(&idx) = self.known_ids.get(&id) {
            return idx;
        }

        assert!(self.ids.len() < usize::from(u16::MAX));

        let idx = repr::uid_gid::Idx(self.ids.len() as u16);
        self.known_ids.insert(id, idx);
        self.ids.push(id);
        idx
    }

    pub async fn write_at<W: io::Write>(
        &mut self,
        mut writer: W,
        start_offset: u64,
        compressor: Option<Arc<ParallelCompressor>>,
    ) -> io::Result<()> {
        let mut table = two_level::Table::with_capacity(compressor, self.ids.len());
        for id in &self.ids {
            table.write(id).await;
        }
        let (data_table, indexes) = table.finish().await;

        writer.write_all(&data_table)?;
        for &idx in &indexes {
            let block_offset = start_offset + u64::from(idx);
            writer.write_all(&block_offset.to_le_bytes())?;
        }

        unimplemented!()
    }
}
