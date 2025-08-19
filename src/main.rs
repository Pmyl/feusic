mod core;
mod ui;

use core::player::controller::FeusicPlayerController;
use core::player::FeusicPlayer;
use core::playlist::loader::BasicFolderPlaylistLoader;
use core::youtube::downloader::YoutubeDownloader;
use std::env;
use std::error::Error;

use ui::FilePreferencesHandler;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "--download" {
        println!("--download arg found, downloading.");
        download(args)
    } else {
        println!("No --download arg found, running player.");
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
}

fn download(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.len() < 3 {
        Err("--download needs another argument, either a youtube link or a path to a file with youtube links, then one more optional to specify the folder to download the audio to".into())
    } else {
        let mut source = None;
        let mut download_dir = None;
        let mut prefix = None;
        let mut skip = None;

        let mut index = 2;
        loop {
            let option = args.get(index).unwrap();
            match option.as_str() {
                "--src" => {
                    index += 1;
                    source = args.get(index).cloned();
                }
                "--dest" => {
                    index += 1;
                    download_dir = args.get(index).cloned();
                }
                "--prefix" => {
                    index += 1;
                    prefix = args.get(index).cloned();
                }
                "--skip" => {
                    index += 1;
                    skip = args.get(index).cloned().map(|s| {
                        s.parse::<usize>()
                            .map_err(|e| format!("{e} -> When parsing --skip"))
                    });
                }
                _ => return Err(format!("Unknown option {}", option).into()),
            }
            index += 1;
            if args.len() <= index {
                break;
            }
        }
        let source = source.ok_or_else(|| "Missing --src")?;
        let download_dir =
            download_dir.unwrap_or(std::env::current_dir().unwrap().display().to_string());
        let prefix = prefix.map(|p| format!("{}_", p)).unwrap_or("".to_string());
        let skip = skip.unwrap_or(Ok(0))?;
        println!("Downloading from {} in folder {}", source, download_dir);
        let list_of_links = if source.starts_with("http") {
            vec![source]
        } else {
            std::fs::read_to_string(source)?
                .lines()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        };
        let downloader = YoutubeDownloader::new(download_dir)?;
        let mut count = skip;
        for link in list_of_links.iter().skip(skip) {
            downloader.download_audio_blocking_with_filename(
                &link,
                &format!("{}{:03}_", prefix, count + 1),
            )?;
            count += 1;
        }
        Ok(())
    }
}
