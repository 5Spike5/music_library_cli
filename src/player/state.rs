use std::sync::{Arc, Mutex, mpsc};
use slint::{Timer, TimerMode, VecModel, ModelRc};
use crate::player::playback::{self, PlaybackState, PlayerEvent, PlayStatus};
use music_library_cli::{Library,MUSIC_FILE_PATH};
use music_library_cli::{load_library};
/// 应用全局状态 —— 所有 Arc<Mutex<>> 收拢在此
pub struct AppState{
    pub library:Arc<Mutex<Library>>,
    pub play_state : Arc<Mutex<PlayerEvent>>,
    /// 后台线程发过来给 Timer 轮询的通道接收端
    pub rx:Arc<Mutex<mpsc::Receiver<PlayerEvent>>>,
    /// 播放线程发消息用的发送端
    pub tx : mpsc::Sender<PlayerEvent>,
}
impl AppState {
    /// 构造共享状态 + 初始加载歌库
    pub fn new(app:&crate::AppWindow) -> Self{
        let library = Arc::new(Mutex::new(
            load_library(MUSIC_FILE_PATH).unwrap_or_default()
        ));
        let play_state = Arc::new(Mutex::new(PlaybackState::d));
    }
}