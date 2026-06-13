use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Song {
    pub id: usize,
    pub title: String,
    pub path: PathBuf,
}
