mod core;
mod ui;

use core::player::controller::FeusicPlayerController;
use core::player::FeusicPlayer;
use core::playlist::loader::BasicFolderPlaylistLoader;
use std::error::Error;

use ui::FilePreferencesHandler;

// TODO: use rustube to download music from youtube

fn main() -> Result<(), Box<dyn Error>> {
    let player = FeusicPlayer::new()?;
    let player_controller = FeusicPlayerController::new(player);
    let playlist_loader = BasicFolderPlaylistLoader;
    let preferences_handler = FilePreferencesHandler::new(".preferences");

    println!("Music player initialized successfully.");

    ui::run_ui(
        ui::FeusicPlayerUi::Egui,
        player_controller,
        playlist_loader,
        preferences_handler,
    )
}
