use std::sync::{Arc, Mutex};

use crate::models::Library;
use crate::state::PlaybackState;

pub type SharedAppState = Arc<Mutex<AppState>>;

#[derive(Debug)]
pub struct AppState {
    pub library: Library,
    pub selected_index: usize,
    pub current_index: Option<usize>,
    pub playback_state: PlaybackState,
    pub status_message: String,
}

impl AppState {
    pub fn new(library: Library) -> Self {
        Self {
            library,
            selected_index: 0,
            current_index: None,
            playback_state: PlaybackState::Stopped,
            status_message: "Ready. Press Enter to play.".to_string(),
        }
    }

    pub fn select_next(&mut self) {
        if self.library.is_empty() {
            return;
        }

        self.selected_index = (self.selected_index + 1) % self.library.len();
    }

    pub fn select_previous(&mut self) {
        if self.library.is_empty() {
            return;
        }

        self.selected_index = if self.selected_index == 0 {
            self.library.len() - 1
        } else {
            self.selected_index - 1
        };
    }

    pub fn set_current_index(&mut self, index: usize) {
        if index < self.library.len() {
            self.current_index = Some(index);
            self.selected_index = index;
        }
    }

    pub fn current_song_title(&self) -> String {
        self.current_index
            .and_then(|idx| self.library.get(idx))
            .map(|song| song.title.clone())
            .unwrap_or_else(|| "No song playing".to_string())
    }

    pub fn set_status_message(&mut self, message: impl Into<String>) {
        self.status_message = message.into();
    }
}
