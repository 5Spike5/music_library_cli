//models::数据结构，比如歌曲和曲库。
use crate::models::song::Song;

#[derive(Debug,Default,Clone)]
pub struct Library{
    pub songs : Vec<Song>,
}

impl Library{
    // 判断库是否为空
    pub fn is_empty(&self) -> bool {
        self.songs.is_empty()
    }

    // 获取歌曲总数
    pub fn len(&self) -> usize {
        self.songs.len()
    }

    // 根据索引获取歌曲引用
    pub fn get(&self, index: usize) -> Option<&Song> {
        self.songs.get(index)
    }

    // 计算下一首的索引（循环播放逻辑）
    pub fn next_index(&self, current: usize) -> Option<usize> {
        if self.is_empty() {
            return None;
        }
        // 如果到了最后一首，回到第一首 (0)
        Some((current + 1) % self.len())
    }

    // 计算前一首的索引（循环播放逻辑）
    pub fn previous_index(&self, current: usize) -> Option<usize> {
        if self.is_empty() {
            return None;
        }
        // 如果在第一首，跳到最后一首
        if current == 0 {
            Some(self.len() - 1)
        } else {
            Some(current - 1)
        }
    }
}