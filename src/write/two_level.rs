use super::metablock_writer::MetablockWriter;
use crate::compress_threads::ParallelCompressor;
use std::marker::PhantomData;
use std::mem;
use std::sync::Arc;
use zerocopy::AsBytes;

pub struct Table<T> {
    data_writer: MetablockWriter,
    index: Vec<u32>,
    _phantom: PhantomData<T>,
}

impl<T: AsBytes> Table<T> {
    pub fn new(compressor: Option<Arc<ParallelCompressor>>) -> Self {
        Self::with_capacity(compressor, 0)
    }

    pub fn with_capacity(compressor: Option<Arc<ParallelCompressor>>, cap: usize) -> Self {
        assert_eq!(repr::metablock::SIZE % mem::size_of::<T>(), 0);
        assert!(mem::size_of::<T>() < repr::metablock::SIZE);

        // Round up to number of metablocks needed to store `cap` `T`s
        let index_size =
            (cap * mem::size_of::<T>() + repr::metablock::SIZE - 1) / repr::metablock::SIZE;
        Self {
            data_writer: MetablockWriter::with_capacity(compressor, cap),
            index: Vec::with_capacity(index_size),
            _phantom: PhantomData,
        }
    }

    pub async fn write(&mut self, item: &T) {
        self.data_writer.write(item).await;
        let position = self.data_writer.position();
        if position.start_offset() == 0 {
            self.index.push(position.block_start());
        }
    }

    // Return (table data, index data)
    pub async fn finish(self) -> (Vec<u8>, Vec<u32>) {
        let table_data = self.data_writer.finish().await;
        (table_data, self.index)
    }
}
