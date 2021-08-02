use crate::compress_threads::ParallelCompressor;
use crate::write::two_level;
use indexmap::IndexSet;
use std::io;
use std::sync::Arc;

pub struct Table {
    ids: IndexSet<repr::uid_gid::Id>,
}

impl Table {
    pub fn new() -> Self {
        Self {
            ids: IndexSet::new(),
        }
    }

    pub fn add(&mut self, id: repr::uid_gid::Id) -> repr::uid_gid::Idx {
        let (idx, _) = self.ids.insert_full(id);

        repr::uid_gid::Idx(idx as u16)
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
