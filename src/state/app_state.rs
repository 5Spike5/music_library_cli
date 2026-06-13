//state::应用运行时状态
use crate::models::library::Library;
use crate::state::playback_state::PlaybackState;

#[derive(Debug)]
pub struct AppState {
    pub library: Library,
    pub current_index: Option<usize>,
    pub playback_state: PlaybackState,
    pub status_message: String,
}

impl AppState {
    /// 初始化应用状态
    pub fn new(library: Library) -> Self{
        Self {
            library,
            current_index: None,
            playback_state: PlaybackState::Stopped,
            status_message: String::from("欢迎使用 Rust 音乐播放器"),
        }
    }
    /// 获取当前正在播放的歌曲标题
    pub fn current_song_title(&self) -> String {
        // 利用 Option 的链式操作：
        // 1. 获取 current_index
        // 2. 通过 and_then 在 library 中查找对应的歌
        // 3. 获取歌名
        // 4. 如果中间任何一步返回 None，则提供默认值
        self.current_index
            .and_then(|idx| self.library.get(idx))
            .map(|song| song.title.clone())
            .unwrap_or_else(|| "未在播放".to_string())
    }

    /// 设置当前播放索引（带安全性检查）
    pub fn set_current_index(&mut self, index: usize) {
        if index < self.library.len() {
            self.current_index = Some(index);
        } else {
            // 如果索引越界，清空当前索引（实际应用中应避免这种情况）
            self.current_index = None;
        }
    }

    /// 修改播放状态
    pub fn set_playback_state(&mut self, state: PlaybackState) {
        self.playback_state = state;
    }

    /// 设置状态栏消息
    /// 使用 impl Into<String> 可以让你同时传入 &str 和 String，非常方便
    pub fn set_status_message(&mut self, message: impl Into<String>) {
        self.status_message = message.into();
    }
}