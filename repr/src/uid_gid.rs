/// UID/GIDs are both stored as u32s. Both UIDs and GIDs are treated as IDs
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Id(u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Idx(u16);
