use crate::compression::AnyCodec;
use crate::write::two_level;

pub struct Table {
    inner: two_level::Table<repr::fragment::Entry>,
    count: usize,
}

impl Table {
    pub fn new(compressor: Option<AnyCodec>) -> Self {
        Self {
            inner: two_level::Table::new(compressor),
            count: 0,
        }
    }

    pub fn add_fragment(&mut self, location: repr::datablock::Ref, size: repr::datablock::Size) {
        let entry = repr::fragment::Entry {
            start: location,
            size,
            _unused: 0,
        };
        self.inner.write(&entry);
        self.count += 1;
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn finish(self) -> (Vec<u8>, Vec<u32>) {
        self.inner.finish()
    }
}

pub(crate) struct BlockBuilder {}
