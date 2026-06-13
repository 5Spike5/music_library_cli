use crate::events::PlaybackEvent;
use crate::state::{AppState, PlaybackState};

pub fn apply_playback_event(state: &mut AppState, event: PlaybackEvent) {
    match event {
        PlaybackEvent::Started(index) => {
            state.set_current_index(index);
            state.playback_state = PlaybackState::Playing;
            state.set_status_message(format!("Playing: {}", state.current_song_title()));
        }
        PlaybackEvent::Paused => {
            state.playback_state = PlaybackState::Paused;
            state.set_status_message("Paused");
        }
        PlaybackEvent::Resumed => {
            state.playback_state = PlaybackState::Playing;
            state.set_status_message(format!("Playing: {}", state.current_song_title()));
        }
        PlaybackEvent::Stopped => {
            state.playback_state = PlaybackState::Stopped;
            state.set_status_message("Stopped");
        }
        PlaybackEvent::Finished(index) => {
            state.set_status_message(format!("Finished song index {}", index + 1));
        }
        PlaybackEvent::SwitchedTo(index) => {
            state.set_current_index(index);
            state.playback_state = PlaybackState::Playing;
            state.set_status_message(format!("Auto switched: {}", state.current_song_title()));
        }
        PlaybackEvent::Error(message) => {
            state.playback_state = PlaybackState::Stopped;
            state.set_status_message(format!("Playback error: {message}"));
        }
    }
}
