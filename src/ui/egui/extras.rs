use std::time::Duration;

use egui::Ui;

use crate::{
    core::{
        feusic::loader::MusicLoader,
        player::{controller::FeusicPlayerController, PlaylistOrder},
        playlist::loader::FolderPlaylistLoader,
    },
    ui::{Preferences, PreferencesHandler},
};

pub(super) fn render<M: MusicLoader>(
    ui: &mut Ui,
    player: &FeusicPlayerController<M>,
    playlist_loader: &impl FolderPlaylistLoader<M>,
    preferences: &mut Preferences,
    preferences_handler: &impl PreferencesHandler,
) {
    ui.horizontal(|ui| {
        if ui.button("Crossfade").clicked() {
            player.crossfade(Duration::from_millis(1000));
        }

        if ui.button("Remove loop").clicked() {
            player.remove_loop();
        }

        if ui.button("Select playlist folder…").clicked() {
            let was_paused = player.paused();
            player.pause();

            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                match playlist_loader.load(path.to_str().unwrap()) {
                    Ok(playlist) => {
                        preferences.last_playlist_path = Some(path.to_str().unwrap().to_string());
                        preferences_handler.save_preferences(preferences);

                        player.set_playlist(playlist);
                        if !was_paused {
                            player.play();
                        }
                    }
                    Err(e) => {
                        if !was_paused {
                            player.resume();
                        }
                        eprintln!("Error loading playlist: {}", e);
                    }
                }
            } else {
                if !was_paused {
                    player.resume();
                }
            }
        }

        if ui.button("Reload playlist").clicked() {
            let was_paused = player.paused();
            player.pause();

            if let Some(path) = &preferences.last_playlist_path {
                match playlist_loader.load(path.as_str()) {
                    Ok(playlist) => {
                        player.set_playlist(playlist);
                        if !was_paused {
                            player.play();
                        }
                    }
                    Err(e) => {
                        if !was_paused {
                            player.resume();
                        }
                        eprintln!("Error loading playlist: {}", e);
                    }
                }
            } else {
                if !was_paused {
                    player.resume();
                }
            }
        }

        let order_name = match player.playlist_order().get() {
            PlaylistOrder::Alphabetical => "Alphabetical",
            PlaylistOrder::Random => "Random",
        };
        ui.menu_button((order_name, " ▼"), |ui| {
            if ui.button("Alphabetical").clicked() {
                player.set_playlist_order(PlaylistOrder::Alphabetical);
            }

            if ui.button("Random").clicked() {
                player.set_playlist_order(PlaylistOrder::Random);
            }
        });
    });
}
