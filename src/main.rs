#![feature(fn_traits)]

mod core;
mod ui;

use core::feusic::loader::FeusicMusicLoader;
use core::feusic::Feusic;
use core::player::controller::FeusicPlayerController;
use core::player::FeusicPlayer;
use std::error::Error;
use std::fs::{self, File};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let folder_path = &args[1];

    let files =
        fs::read_dir(folder_path).map_err(|e| format!("Folder path should exist. {}", e))?;

    println!("Loading files from folder {}", folder_path);

    let playlist: Vec<Feusic<FeusicMusicLoader>> = files
        .filter_map(|file| {
            file.inspect_err(|e| eprintln!("Skipping file because {}", e))
                .ok()
        })
        .inspect(|file| println!("Checking file {:?}", file.path()))
        .filter_map(|file| match file.path().extension() {
            Some(ext) if ext == "feusic" => Some(file),
            _ => None,
        })
        .filter_map(|entry| {
            let path = entry.path();

            if path.is_dir() {
                Some(Feusic::from_feusic_folder(&path))
            } else {
                File::open(&path)
                    .inspect_err(|e| eprintln!("Skipping opening file because {}", e))
                    .ok()
                    .map(|file| Feusic::from_feusic_zip_file(&path, &file))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    println!("Playlist of {}", playlist.len());

    let player = match FeusicPlayer::new() {
        Ok(player) => {
            println!("Music player initialized successfully.");
            player
        }
        Err(e) => {
            return Err(format!("Error initializing music player: {}", e).into());
        }
    };

    let player_controller = FeusicPlayerController::new(player);

    player_controller.set_playlist(playlist);
    player_controller.play();

    ui::egui::run_ui(player_controller).into()
    // ui::terminal::run_ui(player_controller).into()
}
