// slint::include_modules!();
//GUI
// mod player;
// use player::state::AppState;
//程序入口，只负责调用 `app::run()`
fn main() {
    // let app = AppWindow::new()?;
    // let state = AppState::;
}






















// TUI
// use music_library_cli::*;
// fn main(){
//     if let Err(msg) = run() {
//         eprintln!("Error:{msg}")   ;
//         std::process::exit(1);
//     }
// }

// fn run() ->Result<(),String>{
//     let args:Vec<String> = std::env::args().skip(1).collect();
//     let command = music_library_cli::parse_args(args)?;
//     let mut musics_library = load_library(MUSIC_FILE_PATH).map_err(|e| e.to_string())?;

//     match command {
//         Command::List => list_songs(&musics_library),
//         Command::Scan { folder } => {
//             scan_folder(folder.as_str(), &mut musics_library).map_err(|e|e.to_string())?;
//             save_library(MUSIC_FILE_PATH, &mut musics_library).map_err(|e|e.to_string())?;
//         },
//         Command::Search { keyword } => search_songs(&musics_library, keyword.as_str()),
//         Command::Play =>{
//             play_interactive(&musics_library).map_err(|e|e.to_string())?;
//         }
//     }
//     Ok(())
// }