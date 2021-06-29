use crate::compress_threads::ParallelCompressor;
use crate::write::metablock_writer::MetablockWriter;
use crate::write::two_level;
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
}
