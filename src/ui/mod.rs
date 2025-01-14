use std::{error::Error, fs::File};

use serde::{Deserialize, Serialize};

use crate::core::{
    feusic::loader::MusicLoader, player::controller::FeusicPlayerController,
    playlist::loader::FolderPlaylistLoader,
};

pub mod egui;
pub mod terminal;

#[derive(Debug)]
#[allow(unused)]
pub enum FeusicPlayerUi {
    Egui,
    Terminal,
}

pub fn run_ui<M: MusicLoader, P: FolderPlaylistLoader<M>, PH: PreferencesHandler>(
    ui: FeusicPlayerUi,
    player: FeusicPlayerController<M>,
    playlist_loader: P,
    preferences_handler: PH,
) -> Result<(), Box<dyn Error>> {
    let preferences = preferences_handler.load_preferences();
    if let Some(ref playlist_path) = preferences.last_playlist_path {
        player.set_playlist(playlist_loader.load(playlist_path.as_str())?);
        player.play();
    }

    match ui {
        FeusicPlayerUi::Egui => {
            egui::run_ui(player, playlist_loader, preferences, preferences_handler)
        }
        FeusicPlayerUi::Terminal => terminal::run_ui(player),
    }
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Preferences {
    pub last_playlist_path: Option<String>,
}

pub trait PreferencesHandler {
    fn load_preferences(&self) -> Preferences;
    fn save_preferences(&self, preferences: &Preferences);
}

pub struct FilePreferencesHandler {
    pub file_path: String,
}

impl FilePreferencesHandler {
    pub fn new(file_path: impl Into<String>) -> Self {
        Self {
            file_path: file_path.into(),
        }
    }
}

impl PreferencesHandler for FilePreferencesHandler {
    fn load_preferences(&self) -> Preferences {
        File::open(&self.file_path)
            .map_err(|_| "No preferences found".to_string())
            .and_then(|file| serde_json::from_reader(file).map_err(|e| e.to_string()))
            .inspect(|_| println!("Loaded preferences from {}", self.file_path))
            .unwrap_or_else(|e| {
                println!("Failed to read preferences: {:?}", e);
                Preferences::default()
            })
    }

    fn save_preferences(&self, preferences: &Preferences) {
        File::create(&self.file_path)
            .map_err(|e| e.to_string())
            .and_then(|file| serde_json::to_writer(file, &preferences).map_err(|e| e.to_string()))
            .inspect(|_| println!("Saved preferences in {}", self.file_path))
            .unwrap_or_else(|e| eprintln!("Failed to save preferences: {:?}", e));
    }
}
