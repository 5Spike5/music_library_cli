use std::io::{stdout, Stdout};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::commands::PlaybackCommand;
use crate::handlers::{event_handler, playback_handler};
use crate::state::SharedAppState;
use crate::ui_terminal::input::{key_to_action, UiAction};
use crate::{events::PlaybackEvent, ui_terminal::render};

pub fn run_terminal_ui(
    state: SharedAppState,
    command_tx: Sender<PlaybackCommand>,
    event_rx: Receiver<PlaybackEvent>,
) -> Result<(), String> {
    let mut stdout = stdout();
    let _guard = TerminalGuard::enter(&mut stdout)?;

    loop {
        drain_playback_events(&state, &event_rx);
        render_current_state(&mut stdout, &state)?;

        if event::poll(Duration::from_millis(80)).map_err(|err| err.to_string())? {
            let Event::Key(key) = event::read().map_err(|err| err.to_string())? else {
                continue;
            };

            let should_quit = handle_action(&state, &command_tx, key_to_action(key))?;
            if should_quit {
                break;
            }
        }
    }

    let _ = playback_handler::shutdown(&command_tx);
    Ok(())
}

fn drain_playback_events(state: &SharedAppState, event_rx: &Receiver<PlaybackEvent>) {
    while let Ok(event) = event_rx.try_recv() {
        if let Ok(mut state) = state.lock() {
            event_handler::apply_playback_event(&mut state, event);
        }
    }
}

fn render_current_state(stdout: &mut Stdout, state: &SharedAppState) -> Result<(), String> {
    let state = state
        .lock()
        .map_err(|_| "failed to lock app state".to_string())?;
    render::render(stdout, &state)
}

fn handle_action(
    state: &SharedAppState,
    command_tx: &Sender<PlaybackCommand>,
    action: UiAction,
) -> Result<bool, String> {
    match action {
        UiAction::MoveUp => {
            state
                .lock()
                .map_err(|_| "failed to lock app state".to_string())?
                .select_previous();
        }
        UiAction::MoveDown => {
            state
                .lock()
                .map_err(|_| "failed to lock app state".to_string())?
                .select_next();
        }
        UiAction::PlaySelected => {
            let index = state
                .lock()
                .map_err(|_| "failed to lock app state".to_string())?
                .selected_index;
            playback_handler::play_selected(command_tx, index)?;
        }
        UiAction::TogglePause => playback_handler::toggle_pause(command_tx)?,
        UiAction::Next => playback_handler::next(command_tx)?,
        UiAction::Previous => playback_handler::previous(command_tx)?,
        UiAction::Stop => playback_handler::stop(command_tx)?,
        UiAction::Quit => return Ok(true),
        UiAction::None => {}
    }

    Ok(false)
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter(stdout: &mut Stdout) -> Result<Self, String> {
        enable_raw_mode().map_err(|err| err.to_string())?;
        execute!(stdout, EnterAlternateScreen, Hide).map_err(|err| err.to_string())?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), Show, LeaveAlternateScreen);
    }
}
