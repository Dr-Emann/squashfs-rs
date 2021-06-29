//! User/Group IDs

use zerocopy::{AsBytes, FromBytes, Unaligned};

/// UID/GIDs are both stored as u32s. Both UIDs and GIDs are treated as IDs
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, AsBytes, FromBytes, Unaligned,
)]
#[repr(C, packed)]
pub struct Id(pub u32);

/// The index of a user ID in the uid_gid list
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, AsBytes, FromBytes, Unaligned,
)]
#[repr(C, packed)]
pub struct Idx(pub u16);
