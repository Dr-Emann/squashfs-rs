//! User/Group IDs

use packed_serialize::PackedStruct;

/// UID/GIDs are both stored as u32s. Both UIDs and GIDs are treated as IDs
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Id(u32);

/// The index of a user ID in the uid_gid list
#[derive(Debug, Copy, Clone, PartialEq, Eq, PackedStruct)]
pub struct Idx(u16);
