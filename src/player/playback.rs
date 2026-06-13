use std::sync::{Arc,Mutex,mpsc};
/// 播放线程 → 主线程的消息
pub enum PlayerEvent {
    Finished,
    Error(String),
}

#[derive(Default)]
pub struct PlaybackState {
    pub current_song_idx: usize,
    pub status: PlayStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PlayStatus {
    #[default]
    Stopped,
    Playing,
    Paused,
}
#[derive(Debug)]
pub enum PlaybackCommand {
    Play(usize),
    Pause,
    Resume,
    Next,
    Prev,
    Stop,
}