use std::sync::mpsc::Sender;

#[derive(Debug, Clone, PartialEq, Eq)]
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

pub fn send_play_command(
    tx: &Sender<PlaybackCommand>,
    command: PlaybackCommand,
) -> Result<(), String> {
    tx.send(command)
        .map_err(|_| "playback thread is not available".to_string())
}
