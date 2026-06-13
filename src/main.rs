use music_library_cli::{
    app, list_songs, load_library, parse_args, save_library, scan_folder, search_songs, CliCommand,
    MUSIC_FILE_PATH,
};

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let command = parse_args(args)?;

    match command {
        CliCommand::Play => app::run(),
        CliCommand::List => {
            let library = load_library(MUSIC_FILE_PATH).map_err(|err| err.to_string())?;
            list_songs(&library);
            Ok(())
        }
        CliCommand::Search { keyword } => {
            let library = load_library(MUSIC_FILE_PATH).map_err(|err| err.to_string())?;
            search_songs(&library, &keyword);
            Ok(())
        }
        CliCommand::Scan { folder } => {
            let mut library = load_library(MUSIC_FILE_PATH).map_err(|err| err.to_string())?;
            scan_folder(&folder, &mut library).map_err(|err| err.to_string())?;
            save_library(MUSIC_FILE_PATH, &library).map_err(|err| err.to_string())?;
            println!("Scan complete. Library now has {} songs.", library.len());
            Ok(())
        }
    }
}
