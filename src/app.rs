use std::sync::{mpsc, Arc, Mutex};

use crate::playback::engine::spawn_playback_thread;
use crate::state::AppState;
use crate::ui_terminal::terminal::run_terminal_ui;
use crate::{load_library, MUSIC_FILE_PATH};

pub fn run() -> Result<(), String> {
    let library = load_library(MUSIC_FILE_PATH).map_err(|err| err.to_string())?;
    let state = Arc::new(Mutex::new(AppState::new(library.clone())));

    let (command_tx, command_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();

    let playback_handle = spawn_playback_thread(library, command_rx, event_tx);
    let ui_result = run_terminal_ui(state, command_tx, event_rx);

    let _ = playback_handle.join();
    ui_result
}
