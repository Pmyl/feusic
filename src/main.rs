#![feature(fn_traits)]

mod core;
mod ui;

use core::player::controller::FeusicPlayerController;
use core::player::FeusicPlayer;
use core::playlist::loader::{BasicFolderPlaylistLoader, FolderPlaylistLoader};
use std::error::Error;

use ui::FilePreferencesHandler;

fn main() -> Result<(), Box<dyn Error>> {
    // let args: Vec<String> = std::env::args().collect();
    // let folder_path = &args[1];

    let playlist_loader = BasicFolderPlaylistLoader;
    // let playlist = playlist_loader.load(folder_path.as_str())?;
    let player = FeusicPlayer::new()?;
    let player_controller = FeusicPlayerController::new(player);

    println!("Music player initialized successfully.");

    // player_controller.set_playlist(playlist);
    // player_controller.play();

    ui::egui::run_ui(
        player_controller,
        playlist_loader,
        FilePreferencesHandler {
            file_path: ".preferences".to_string(),
        },
    )
    .into()
    // ui::terminal::run_ui(player_controller).into()
}
