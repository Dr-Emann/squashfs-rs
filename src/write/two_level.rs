use super::metablock_writer::MetablockWriter;
use crate::compression::Compressor;
use std::marker::PhantomData;
use std::{fmt, mem};
use zerocopy::AsBytes;

pub struct Table<T, Comp> {
    data_writer: MetablockWriter<Comp>,
    index: Vec<u32>,
    _phantom: PhantomData<T>,
}

impl<T: AsBytes, Comp: Compressor> Table<T, Comp> {
    const _T_SIZE_ASSERT: () = assert!(repr::metablock::SIZE % mem::size_of::<T>() == 0);

    pub fn new(compressor: Option<Comp>) -> Self {
        Self::with_capacity(compressor, 0)
    }

    pub fn with_capacity(compressor: Option<Comp>, cap: usize) -> Self {
        assert_eq!(repr::metablock::SIZE % mem::size_of::<T>(), 0);
        assert!(mem::size_of::<T>() < repr::metablock::SIZE);

        let index_size = cap * mem::size_of::<T>() / repr::metablock::SIZE;
        Self {
            data_writer: MetablockWriter::with_capacity(compressor, cap),
            index: Vec::with_capacity(index_size),
            _phantom: PhantomData,
        }
    }

    pub fn write(&mut self, item: &T) {
        self.data_writer.write(item);
        let position = self.data_writer.position();
        if position.start_offset() == 0 {
            self.index.push(position.block_start());
        }
    }

    // Return (table data, index data)
    pub fn finish(self) -> (Vec<u8>, Vec<u32>) {
        let table_data = self.data_writer.finish();
        (table_data, self.index)
    }
}

impl<T, Comp> fmt::Debug for Table<T, Comp> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Table")
            .field("data_writer", &self.data_writer)
            .field("index", &self.index)
            .finish()
    }
}

impl<T, Comp: Default> Default for Table<T, Comp> {
    fn default() -> Self {
        Self {
            data_writer: MetablockWriter::default(),
            index: Vec::default(),
            _phantom: PhantomData::default(),
        }
    }
}
