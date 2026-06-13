//UI 发给播放线程的命令
use std::sync::mpsc::Sender;
#[derive(Debug, Clone)]
pub enum PlaybackCommand {
    Play(usize),
    Pause,
    Resume,
    TogglePause,
    Stop,
    Next,
    Previous,
    Shutdown,
}
/// 向播放线程发送指令的统一入口
pub fn send_play_command(tx: &Sender<PlaybackCommand>,command: PlaybackCommand,) -> Result<(), String>{
    // 这里可以添加日志记录，方便调试时观察 UI 发出了什么指令
    println!("UI 发送指令: {:?}", command);
    tx.send(command)
        .map_err(|e| {
            // 当接收端（播放线程）已经关闭时，send 会失败
            format!("无法发送指令，播放线程可能已崩溃或关闭: {}", e)
    })
}