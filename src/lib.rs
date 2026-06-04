use std::{error::Error, path::Path};
use std::fs::File;
use std::io::{stdin, stdout, Write};
use crossterm::cursor;
use serde::{Serialize,Deserialize};
use crossterm::{
    event::{self,Event,KeyCode,KeyEvent,KeyEventKind},
    execute,
    terminal::{disable_raw_mode,enable_raw_mode,Clear,ClearType},
    cursor::{Hide,Show}
};
use rodio::{Decoder, DeviceSinkBuilder, Player};
pub const MUSIC_FILE_PATH:&str = "library.json";

pub enum Command {
    Scan{folder:String},
    List,
    Search{keyword:String},
    Play
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Song{
    id:usize,
    title:String,
    path:String
}
#[derive(Debug, Serialize, Deserialize)]
pub struct  Library{
    songs:Vec<Song>
}
pub fn parse_args(args:Vec<String>) ->Result<Command,String>{
    if args.is_empty() {
        return Err(
            "Usage:\n cargo run -- list\n cargo run -- scan folder\n cargo run -- search <keyword> -- cargo run -- play"
                .to_string(),
        );
    }
    match args[0].as_str() {
        "list" =>{
            if args.len() != 1 {
                return Err("`list` does not accept extra arguments.".to_string());
            } else {
                Ok(Command::List)
            }
        },
        "scan" =>{
            if args.len() < 2 {
                return Err(
                    "`scan` requires a file_path. Example: cargo run -- scan {your file_path}"
                        .to_string(),
                );
            }
            if args.len() > 2 {
                return Err("`scan` does not accept extra arguments.".to_string());
            }
            let folder = args[1].clone();
            if folder.is_empty() {
                return Err("file_path cannot be empty".to_string());
            }

            Ok(Command::Scan { folder })
        },
        "search" =>{
            if args.len() < 2 {
                return Err(
                    "`search` requires a keyword. Example: cargo run -- search keyword"
                        .to_string(),
                );
            }
            let keyword = args[1..].join(" ");
            if keyword.trim().is_empty() {
                return Err("keyword cannot be empty".to_string());
            }
            Ok(Command::Search { keyword })
        },
        "play" => {
            if args.len() != 1 {
                return Err("`play` does not accept extra arguments.".to_string());
            } else {
                Ok(Command::Play)
            }
        },
        cmd => Err(format!(
            "Unknown command: {}. Available commands: list, search, scan, play",
            cmd
        )),
    }
}
pub fn load_library(path:&str) ->Result<Library,Box<dyn Error>>{
    if !std::path::Path::new(path).exists() {
        return Ok(Library { songs: Vec::new() });
    }
    let content = std::fs::read_to_string(path)?;
    let library:Library = serde_json::from_str(&content)?;
    Ok(library)
}
pub fn save_library(path:&str,library:&Library) ->Result<(),Box<dyn Error>>{
    let content = serde_json::to_string_pretty(library)?;
    std::fs::write(path, content)?;
    Ok(())
}
pub fn scan_folder(folder:&str,library:&mut Library) ->Result<(),Box<dyn Error>>{
    let path = Path::new(folder);
    
    if !path.exists() {
        return Err(format!("{} does not exist.", path.display()).into());
    }
    if !path.is_dir() {
        return Err(format!("{} is not a directory.", path.display()).into());
    }
    let mut next_id = library.songs.iter().map(|song| song.id).max().unwrap_or(0)+1;
    for entry in std::fs::read_dir(path)? {
        
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.is_file() && is_music_file(&entry_path) {
            let song_path = entry_path.to_string_lossy().to_string();
            let title = entry_path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown").to_string();
            library.songs.push(Song { id: next_id, title, path: song_path });
            next_id +=1;
        }
    }
    Ok(())
}
pub fn list_songs(library:&Library) {
    if library.songs.is_empty() {
        println!("📂 曲库为空，请先用 scan 命令添加音乐");
        return;
    }
    println!("{:<6}  {:<30}  {}", "序号", "标题", "路径");
    for (idx, song) in library.songs.iter().enumerate() {
        println!("{:<6}  {:<30}  {}", idx + 1, song.title, song.path);
    }
}
pub fn search_songs(library:&Library,keyword:&str) {
    if library.songs.is_empty() {
        println!("📂 曲库为空，请先用 scan 命令添加音乐");
        return;
    }
    let keyword = keyword.to_ascii_lowercase();
    let searched_songs = library.songs
            .iter()
            .filter(|song|song.title.to_ascii_lowercase().contains(&keyword))
            .collect::<Vec<_>>();
    if searched_songs.is_empty() {
        println!("未找到匹配「{}」的歌曲", keyword);
        return;
    }
    println!("{:<6}  {:<30}  {}", "序号", "标题", "路径");
    for (idx, song) in searched_songs.iter().enumerate() {
        println!("{:<6}  {:<30}  {}", idx + 1, song.title, song.path);
    }
}
pub fn is_music_file(path:&Path) ->bool{
    let extension_name = match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => ext.to_ascii_lowercase(),
        None => return false,
    };
    matches!(extension_name.as_str(), "mp3" | "flac" | "wav" | "ogg" | "aac" | "m4a" | "wma" | "opus")
}

pub fn play_interactive(library:&Library)->Result<(),Box<dyn Error>> {
    if library.songs.is_empty() {
        println!("📂 曲库为空，请先用 scan 命令添加音乐");
        return Ok(());
    }
    
    // 启用 raw mode（单键立即响应，不需要按回车）
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout,Hide)?;
    
    while event::poll(std::time::Duration::from_millis(0))? {
        let _ = event::read()?;
    }
    let mut selected:usize = 0;
    // 监听循环
    loop {
        // 清屏 + 绘制当前列表（高亮行用不同颜色或 >> 标记）
        execute!(stdout,Clear(ClearType::All),cursor::MoveTo(0,0))?;
        for (i,song) in library.songs.iter().enumerate() {
            if i == selected{
                 println!("→ {:<30}  {}\r", song.title, song.path);  // 高亮行
            }else {
                println!("  {:<30}  {}\r", song.title, song.path);
            }
        }
        stdout.flush()?;   // ← 在 for 循环画完列表之后、event::read() 之前加上

        match event::read()? {
            Event::Key(KeyEvent { code:KeyCode::Up,kind: KeyEventKind::Press,.. }) =>{
                if selected > 0 {
                    selected -= 1;
                }
            },
            Event::Key(KeyEvent { code:KeyCode::Down,kind: KeyEventKind::Press,.. }) =>{
                if selected < library.songs.len()-1 {
                    selected += 1;
                }
            },
            Event::Key(KeyEvent { code: KeyCode::Enter, kind: KeyEventKind::Press,.. }) => {
                let song = &library.songs[selected];

                execute!(stdout,Show)?;
                disable_raw_mode()?;
                execute!(stdout,Clear(ClearType::All),cursor::MoveTo(0,0))?;

                println!("正在播放: {}", song.title);
                println!("路径: {}", song.path);
                println!();

                let result = play_song(song);

                match result {
                    Ok(()) => println!("播放结束，按回车返回歌曲列表。"),
                    Err(err) => println!("播放失败: {err}\n按回车返回歌曲列表。"),
                }

                let mut input = String::new();
                stdin().read_line(&mut input)?;

                enable_raw_mode()?;
                execute!(stdout,Hide)?;
            },
            Event::Key(KeyEvent { code: KeyCode::Char('q'), kind: KeyEventKind::Press,.. }) => {
                break;  // 退出
            },
            _ => {}
        }
    }
    execute!(stdout,Show)?;
    disable_raw_mode()?;
    Ok(())
}

fn play_song(song: &Song) -> Result<(), Box<dyn Error>> {
    let sink_handle = DeviceSinkBuilder::open_default_sink()?;//打开系统默认音频输出设备
    let player = Player::connect_new(sink_handle.mixer());//创建一个播放器

    let file = File::open(&song.path)?;
    let source = Decoder::try_from(file)?;//把音频文件解码成可播放音频流

    player.append(source);//把当前歌曲放进播放器
    player.sleep_until_end();//阻塞当前线程，等歌曲播完
    Ok(())
}
