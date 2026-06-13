//models::数据结构，比如歌曲和曲库。
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Song {
    pub id: usize,
    pub title: String,
    pub path: PathBuf,
}