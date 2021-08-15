use crate::compress_threads::ParallelCompressor;
use crate::write::two_level;
use std::future::Future;
use std::sync::Arc;

pub struct Table {
    inner: two_level::Table<repr::fragment::Entry>,
    count: usize,
}

impl Table {
    pub fn new(compressor: Option<Arc<ParallelCompressor>>) -> Self {
        Self {
            inner: two_level::Table::new(compressor),
            count: 0,
        }
    }

    pub async fn add_fragment(
        &mut self,
        location: repr::datablock::Ref,
        size: repr::datablock::Size,
    ) {
        let entry = repr::fragment::Entry {
            start: location,
            size,
            _unused: 0,
        };
        self.inner.write(&entry).await;
        self.count += 1;
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn finish(self) -> impl Future<Output = (Vec<u8>, Vec<u32>)> {
        self.inner.finish()
    }
}
