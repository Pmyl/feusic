use std::fs::File;

use serde::{Deserialize, Serialize};

pub mod egui;
pub mod terminal;

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
