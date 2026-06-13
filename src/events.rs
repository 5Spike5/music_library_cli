//播放线程发回 UI 的事件
#[derive(Debug, Clone)]
pub enum PlaybackEvent {
    Started(usize),//某首歌开始播放
    Paused,//播放已暂停
    Resumed,//播放已继续
    Stopped,//播放已停止
    Finished(usize),//某首歌自然播放结束
    SwitchedTo(usize),//自动切换或手动切换到了另一首
    Error(String),//播放失败，比如文件不存在或解码失败
}