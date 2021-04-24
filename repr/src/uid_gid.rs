//! User/Group IDs

/// UID/GIDs are both stored as u32s. Both UIDs and GIDs are treated as IDs
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C, packed)]
pub struct Id(pub u32);
unsafe impl crate::Repr for Id {}

/// The index of a user ID in the uid_gid list
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C, packed)]
pub struct Idx(pub u16);
unsafe impl crate::Repr for Idx {}
