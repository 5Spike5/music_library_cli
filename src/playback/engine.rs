use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use crate::commands::PlaybackCommand;
use crate::events::PlaybackEvent;
use crate::models::Library;
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
    ) -> Result<Self, String> {
        Ok(Self {
            library,
            command_rx,
            event_tx,
            player: RodioPlayer::new()?,
            current_index: None,
        })
    }

    pub fn run(&mut self) {
        loop {
            match self.command_rx.recv_timeout(Duration::from_millis(120)) {
                Ok(command) => {
                    if self.handle_command(command) {
                        break;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    self.auto_advance_if_finished();
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        self.player.stop();
        let _ = self.event_tx.send(PlaybackEvent::Stopped);
    }

    fn handle_command(&mut self, command: PlaybackCommand) -> bool {
        match command {
            PlaybackCommand::Play(index) => self.play_index(index),
            PlaybackCommand::Pause => {
                self.player.pause();
                let _ = self.event_tx.send(PlaybackEvent::Paused);
            }
            PlaybackCommand::Resume => {
                self.player.resume();
                let _ = self.event_tx.send(PlaybackEvent::Resumed);
            }
            PlaybackCommand::TogglePause => {
                if self.player.is_paused() {
                    self.player.resume();
                    let _ = self.event_tx.send(PlaybackEvent::Resumed);
                } else {
                    self.player.pause();
                    let _ = self.event_tx.send(PlaybackEvent::Paused);
                }
            }
            PlaybackCommand::Stop => {
                self.player.stop();
                self.current_index = None;
                let _ = self.event_tx.send(PlaybackEvent::Stopped);
            }
            PlaybackCommand::Next => self.play_relative(true),
            PlaybackCommand::Previous => self.play_relative(false),
            PlaybackCommand::Shutdown => return true,
        }

        false
    }

    fn play_index(&mut self, index: usize) {
        let Some(song) = self.library.get(index) else {
            let _ = self
                .event_tx
                .send(PlaybackEvent::Error(format!("song index {} does not exist", index + 1)));
            return;
        };

        match self.player.play_file(&song.path) {
            Ok(()) => {
                self.current_index = Some(index);
                let _ = self.event_tx.send(PlaybackEvent::Started(index));
            }
            Err(err) => {
                self.current_index = None;
                let _ = self.event_tx.send(PlaybackEvent::Error(err));
            }
        }
    }

    fn play_relative(&mut self, next: bool) {
        if self.library.is_empty() {
            let _ = self
                .event_tx
                .send(PlaybackEvent::Error("library is empty".to_string()));
            return;
        }

        let target = match (self.current_index, next) {
            (Some(current), true) => self.library.next_index(current),
            (Some(current), false) => self.library.previous_index(current),
            (None, true) => Some(0),
            (None, false) => Some(self.library.len() - 1),
        };

        if let Some(index) = target {
            self.play_index(index);
            let _ = self.event_tx.send(PlaybackEvent::SwitchedTo(index));
        }
    }

    fn auto_advance_if_finished(&mut self) {
        let Some(current) = self.current_index else {
            return;
        };

        if self.player.is_paused() || !self.player.is_empty() {
            return;
        }

        let _ = self.event_tx.send(PlaybackEvent::Finished(current));

        if let Some(next) = self.library.next_index(current) {
            self.play_index(next);
            let _ = self.event_tx.send(PlaybackEvent::SwitchedTo(next));
        } else {
            self.current_index = None;
            let _ = self.event_tx.send(PlaybackEvent::Stopped);
        }
    }
}

pub fn spawn_playback_thread(
    library: Library,
    command_rx: Receiver<PlaybackCommand>,
    event_tx: Sender<PlaybackEvent>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || match PlaybackEngine::new(library, command_rx, event_tx.clone()) {
        Ok(mut engine) => engine.run(),
        Err(err) => {
            let _ = event_tx.send(PlaybackEvent::Error(err));
        }
    })
}
