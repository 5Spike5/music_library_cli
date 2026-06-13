use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiAction {
    MoveUp,
    MoveDown,
    PlaySelected,
    TogglePause,
    Next,
    Previous,
    Stop,
    Quit,
    None,
}

pub fn key_to_action(key: KeyEvent) -> UiAction {
    if key.kind != KeyEventKind::Press {
        return UiAction::None;
    }

    match key.code {
        KeyCode::Up => UiAction::MoveUp,
        KeyCode::Down => UiAction::MoveDown,
        KeyCode::Enter => UiAction::PlaySelected,
        KeyCode::Char(' ') => UiAction::TogglePause,
        KeyCode::Char('n') | KeyCode::Right => UiAction::Next,
        KeyCode::Char('p') | KeyCode::Left => UiAction::Previous,
        KeyCode::Char('s') => UiAction::Stop,
        KeyCode::Char('q') | KeyCode::Esc => UiAction::Quit,
        _ => UiAction::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyModifiers, KeyEventState};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn maps_enter_to_play_selected() {
        assert_eq!(key_to_action(key(KeyCode::Enter)), UiAction::PlaySelected);
    }

    #[test]
    fn maps_space_to_toggle_pause() {
        assert_eq!(key_to_action(key(KeyCode::Char(' '))), UiAction::TogglePause);
    }
}
