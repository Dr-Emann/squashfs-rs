use crate::SparseRead;
use std::fs::File;

// Accept the default impl of never skipping any holes
impl SparseRead for File {}
