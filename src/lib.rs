use std::error::Error;
use std::path::Path;

pub mod app;
pub mod commands;
pub mod events;
pub mod handlers;
pub mod models;
pub mod playback;
pub mod state;
pub mod ui_terminal;

use models::library::Library;
use models::song::Song;

pub const MUSIC_FILE_PATH: &str = "library.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Scan { folder: String },
    List,
    Search { keyword: String },
    Play,
}

pub fn parse_args(args: Vec<String>) -> Result<CliCommand, String> {
    if args.is_empty() {
        return Ok(CliCommand::Play);
    }

    match args[0].as_str() {
        "list" => {
            if args.len() != 1 {
                return Err("`list` does not accept extra arguments.".to_string());
            }
            Ok(CliCommand::List)
        }
        "scan" => {
            if args.len() != 2 {
                return Err("Usage: cargo run -- scan <folder>".to_string());
            }
            let folder = args[1].trim();
            if folder.is_empty() {
                return Err("folder cannot be empty.".to_string());
            }
            Ok(CliCommand::Scan {
                folder: folder.to_string(),
            })
        }
        "search" => {
            if args.len() < 2 {
                return Err("Usage: cargo run -- search <keyword>".to_string());
            }
            let keyword = args[1..].join(" ");
            if keyword.trim().is_empty() {
                return Err("keyword cannot be empty.".to_string());
            }
            Ok(CliCommand::Search { keyword })
        }
        "play" => {
            if args.len() != 1 {
                return Err("`play` does not accept extra arguments.".to_string());
            }
            Ok(CliCommand::Play)
        }
        cmd => Err(format!(
            "Unknown command: {cmd}. Available commands: play, list, scan, search"
        )),
    }
}

pub fn load_library(path: &str) -> Result<Library, Box<dyn Error>> {
    if !Path::new(path).exists() {
        return Ok(Library::default());
    }

    let content = std::fs::read_to_string(path)?;
    let raw: StoredLibrary = serde_json::from_str(&content)?;
    Ok(raw.into())
}

pub fn save_library(path: &str, library: &Library) -> Result<(), Box<dyn Error>> {
    let raw = StoredLibrary::from(library.clone());
    let content = serde_json::to_string_pretty(&raw)?;
    std::fs::write(path, content)?;
    Ok(())
}

pub fn scan_folder(folder: &str, library: &mut Library) -> Result<(), Box<dyn Error>> {
    let path = Path::new(folder);

    if !path.exists() {
        return Err(format!("{} does not exist.", path.display()).into());
    }
    if !path.is_dir() {
        return Err(format!("{} is not a directory.", path.display()).into());
    }

    let mut next_id = library.songs.iter().map(|song| song.id).max().unwrap_or(0) + 1;

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        if !entry_path.is_file() || !is_music_file(&entry_path) {
            continue;
        }

        let already_exists = library.songs.iter().any(|song| song.path == entry_path);
        if already_exists {
            continue;
        }

        let title = entry_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        library.songs.push(Song {
            id: next_id,
            title,
            path: entry_path,
        });
        next_id += 1;
    }

    Ok(())
}

pub fn list_songs(library: &Library) {
    if library.is_empty() {
        println!("Library is empty. Run `cargo run -- scan <folder>` first.");
        return;
    }

    println!("{:<6}  {:<36}  {}", "Index", "Title", "Path");
    for (idx, song) in library.songs.iter().enumerate() {
        println!("{:<6}  {:<36}  {}", idx + 1, song.title, song.path.display());
    }
}

pub fn search_songs(library: &Library, keyword: &str) {
    if library.is_empty() {
        println!("Library is empty. Run `cargo run -- scan <folder>` first.");
        return;
    }

    let keyword = keyword.to_ascii_lowercase();
    let found = library
        .songs
        .iter()
        .filter(|song| song.title.to_ascii_lowercase().contains(&keyword))
        .collect::<Vec<_>>();

    if found.is_empty() {
        println!("No songs matched `{keyword}`.");
        return;
    }

    println!("{:<6}  {:<36}  {}", "Index", "Title", "Path");
    for (idx, song) in found.iter().enumerate() {
        println!("{:<6}  {:<36}  {}", idx + 1, song.title, song.path.display());
    }
}

pub fn is_music_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };

    matches!(
        ext.to_ascii_lowercase().as_str(),
        "mp3" | "flac" | "wav" | "ogg" | "aac" | "m4a" | "wma" | "opus"
    )
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct StoredSong {
    id: usize,
    title: String,
    path: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct StoredLibrary {
    songs: Vec<StoredSong>,
}

impl From<StoredLibrary> for Library {
    fn from(value: StoredLibrary) -> Self {
        Self {
            songs: value
                .songs
                .into_iter()
                .map(|song| Song {
                    id: song.id,
                    title: song.title,
                    path: song.path.into(),
                })
                .collect(),
        }
    }
}

impl From<Library> for StoredLibrary {
    fn from(value: Library) -> Self {
        Self {
            songs: value
                .songs
                .into_iter()
                .map(|song| StoredSong {
                    id: song.id,
                    title: song.title,
                    path: song.path.to_string_lossy().to_string(),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_defaults_to_play_when_no_args() {
        assert_eq!(parse_args(vec![]).unwrap(), CliCommand::Play);
    }

    #[test]
    fn detects_supported_music_extensions() {
        assert!(is_music_file(Path::new("song.mp3")));
        assert!(is_music_file(Path::new("song.FLAC")));
        assert!(!is_music_file(Path::new("cover.png")));
    }
}
