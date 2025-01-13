use std::time::Duration;

use egui::Ui;

use crate::core::{
    feusic::loader::MusicLoader, player::controller::FeusicPlayerController,
    playlist::loader::FolderPlaylistLoader,
};

pub(super) fn render<M: MusicLoader>(
    ui: &mut Ui,
    player: &FeusicPlayerController<M>,
    playlist_loader: &impl FolderPlaylistLoader<M>,
) {
    ui.horizontal(|ui| {
        if ui.button("Crossfade").clicked() {
            player.crossfade(Duration::from_millis(1000));
        }

        if ui.button("Remove loop").clicked() {
            player.remove_loop();
        }

        if ui.button("Select playlist folder…").clicked() {
            player.pause();

            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                match playlist_loader.load(path.to_str().unwrap()) {
                    Ok(playlist) => {
                        player.set_playlist(playlist);
                        player.play();
                    }
                    Err(e) => {
                        player.resume();
                        eprintln!("Error loading playlist: {}", e);
                    }
                }
            } else {
                player.resume();
            }
        }
    });
}
