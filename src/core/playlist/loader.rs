use std::{
    error::Error,
    fs::{self, File},
};

use crate::core::feusic::{
    loader::{FeusicMusicLoader, MusicLoader},
    Feusic,
};

pub trait FolderPlaylistLoader<M: MusicLoader>: Clone {
    fn load(&self, folder_path: &str) -> Result<Vec<Feusic<M>>, Box<dyn Error>>;
}

#[derive(Clone)]
pub struct BasicFolderPlaylistLoader;

impl FolderPlaylistLoader<FeusicMusicLoader> for BasicFolderPlaylistLoader {
    fn load(&self, folder_path: &str) -> Result<Vec<Feusic<FeusicMusicLoader>>, Box<dyn Error>> {
        let files =
            fs::read_dir(folder_path).map_err(|e| format!("Folder path should exist. {}", e))?;

        println!("Loading files from folder {}", folder_path);

        files
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
            .collect::<Result<Vec<_>, _>>()
            .inspect(|playlist| println!("Playlist of {}", playlist.len()))
    }
}
