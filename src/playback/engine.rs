//playback::音频播放引擎，封装 `rodio`
use std::sync::mpsc::{Receiver, Sender};
use crate::commands::PlaybackCommand;
use crate::events::PlaybackEvent; // 假设你定义了事件发回给 UI
use crate::models::library::Library;
use crate::playback::rodio_player::RodioPlayer;

pub struct PlaybackEngine {
    library: Library,
    command_rx: Receiver<PlaybackCommand>,
    event_tx: Sender<PlaybackEvent>,
    player: RodioPlayer,
    current_index: Option<usize>,
}

impl PlaybackEngine {
    pub fn new(
        library: Library,
        command_rx: Receiver<PlaybackCommand>,
        event_tx: Sender<PlaybackEvent>,
    ) -> Self {
        let player = RodioPlayer::new().expect("初始化音频失败");
        Self {
            library,
            command_rx,
            event_tx,
            player,
            current_index: None,
        }
    }

    pub fn run(&mut self) {
        // 这是后台线程的主循环
        loop {
            // recv() 会阻塞线程，直到 UI 发来指令
            // 就像厨师盯着传递窗，没单子就坐着休息，不消耗 CPU
            match self.command_rx.recv() {
                Ok(PlaybackCommand::Play(index)) => {
                    if let Some(song) = self.library.get(index) {
                        if let Ok(_) = self.player.play_file(&song.path) {
                            self.current_index = Some(index);
                            // 告诉 UI：我已经开始播放了
                            let _ = self.event_tx.send(PlaybackEvent::Started(index));
                        }
                    }
                }
                Ok(PlaybackCommand::Pause) => self.player.pause(),
                Ok(PlaybackCommand::Resume) => self.player.resume(),
                Ok(PlaybackCommand::Stop) => self.player.stop(),
                Ok(PlaybackCommand::Shutdown) | Err(_) => {
                    break; // 退出循环，线程结束
                }
                // 这里还可以处理 Next, Previous 等逻辑
                _ => {}
            }
        }
    }
}

/// 真正的“开辟线程”函数
pub fn spawn_playback_thread(
    library: Library,
    command_rx: Receiver<PlaybackCommand>,
    event_tx: Sender<PlaybackEvent>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut engine = PlaybackEngine::new(library, command_rx, event_tx);
        engine.run();
    })
}