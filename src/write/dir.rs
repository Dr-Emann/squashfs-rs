pub struct Table {
    data: Vec<u8>,
}

impl Table {
    pub fn start_dir(&mut self) -> DirBuilder<'_> {
        unimplemented!()
    }
}

pub struct DirBuilder<'a> {
    table: &'a Table,
}
