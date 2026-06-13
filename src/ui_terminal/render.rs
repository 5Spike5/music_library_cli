use std::io::Write;

use crossterm::{
    cursor,
    queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{Clear, ClearType},
};

use crate::state::AppState;

const MAX_VISIBLE_SONGS: usize = 18;

pub fn render(stdout: &mut impl Write, state: &AppState) -> Result<(), String> {
    queue!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))
        .map_err(|err| err.to_string())?;

    queue!(
        stdout,
        SetForegroundColor(Color::Cyan),
        SetAttribute(Attribute::Bold),
        Print("Rust Music Player - TUI\n"),
        ResetColor,
        SetAttribute(Attribute::Reset),
        Print("Up/Down: select  Enter: play  Space: pause/resume  n/p: next/previous  s: stop  q: quit\n"),
        Print("--------------------------------------------------------------------------------\n")
    )
    .map_err(|err| err.to_string())?;

    queue!(
        stdout,
        Print(format!(
            "State: {:<8}  Now: {}\n",
            state.playback_state.label(),
            state.current_song_title()
        )),
        Print(format!("Status: {}\n", state.status_message)),
        Print("--------------------------------------------------------------------------------\n")
    )
    .map_err(|err| err.to_string())?;

    if state.library.is_empty() {
        queue!(
            stdout,
            SetForegroundColor(Color::Yellow),
            Print("Library is empty.\n"),
            Print("Run: cargo run -- scan <your-music-folder>\n"),
            ResetColor
        )
        .map_err(|err| err.to_string())?;
        stdout.flush().map_err(|err| err.to_string())?;
        return Ok(());
    }

    let start = visible_start(state.selected_index, state.library.len());
    let end = (start + MAX_VISIBLE_SONGS).min(state.library.len());

    for index in start..end {
        let song = &state.library.songs[index];
        let is_selected = index == state.selected_index;
        let is_current = Some(index) == state.current_index;

        if is_selected {
            queue!(
                stdout,
                SetForegroundColor(Color::Black),
                SetAttribute(Attribute::Reverse)
            )
            .map_err(|err| err.to_string())?;
        } else if is_current {
            queue!(stdout, SetForegroundColor(Color::Green)).map_err(|err| err.to_string())?;
        }

        let marker = match (is_selected, is_current) {
            (true, true) => "> *",
            (true, false) => ">  ",
            (false, true) => "  *",
            (false, false) => "   ",
        };

        queue!(
            stdout,
            Print(format!(
                "{marker} {:>3}. {:<46} {}\n",
                index + 1,
                truncate(&song.title, 46),
                truncate(&song.path.display().to_string(), 46)
            )),
            ResetColor,
            SetAttribute(Attribute::Reset)
        )
        .map_err(|err| err.to_string())?;
    }

    queue!(
        stdout,
        Print("--------------------------------------------------------------------------------\n"),
        Print(format!(
            "Showing {}-{} of {} songs\n",
            start + 1,
            end,
            state.library.len()
        ))
    )
    .map_err(|err| err.to_string())?;

    stdout.flush().map_err(|err| err.to_string())
}

fn visible_start(selected: usize, total: usize) -> usize {
    if total <= MAX_VISIBLE_SONGS {
        return 0;
    }

    let half = MAX_VISIBLE_SONGS / 2;
    selected
        .saturating_sub(half)
        .min(total.saturating_sub(MAX_VISIBLE_SONGS))
}

fn truncate(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();

    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}
