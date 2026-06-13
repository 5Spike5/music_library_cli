use std::sync::mpsc::Sender;

use crate::commands::{send_play_command, PlaybackCommand};

pub fn play_selected(tx: &Sender<PlaybackCommand>, index: usize) -> Result<(), String> {
    send_play_command(tx, PlaybackCommand::Play(index))
}

pub fn toggle_pause(tx: &Sender<PlaybackCommand>) -> Result<(), String> {
    send_play_command(tx, PlaybackCommand::TogglePause)
}

pub fn next(tx: &Sender<PlaybackCommand>) -> Result<(), String> {
    send_play_command(tx, PlaybackCommand::Next)
}

pub fn previous(tx: &Sender<PlaybackCommand>) -> Result<(), String> {
    send_play_command(tx, PlaybackCommand::Previous)
}

pub fn stop(tx: &Sender<PlaybackCommand>) -> Result<(), String> {
    send_play_command(tx, PlaybackCommand::Stop)
}

pub fn shutdown(tx: &Sender<PlaybackCommand>) -> Result<(), String> {
    send_play_command(tx, PlaybackCommand::Shutdown)
}
