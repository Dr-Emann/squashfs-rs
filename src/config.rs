#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FragmentMode {
    /// Never create fragments
    ///
    /// Small files will be stored as full blocks
    Never,
    /// Store small files in fragments
    ///
    /// Files smaller than the block size will be packed into fragments
    SmallFiles,
    /// Store small files, and the end of files which are not a multiple of the block size
    Always,
}

impl Default for FragmentMode {
    fn default() -> Self {
        FragmentMode::Always
    }
}
