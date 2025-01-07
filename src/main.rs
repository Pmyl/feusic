#![feature(fn_traits)]

mod core;
mod ui;

use core::feusic::loader::FesicMusicLoader;
use core::feusic::Feusic;
use core::player::FeusicPlayer;
use std::error::Error;
use std::fs::{self, File};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let folder_path = &args[1];

    let files =
        fs::read_dir(folder_path).map_err(|e| format!("Folder path should exist. {}", e))?;

    println!("Loading files from folder {}", folder_path);

    let playlist: Vec<Feusic<FesicMusicLoader>> = files
        .filter_map(|file| {
            file.inspect_err(|e| eprintln!("Skipping file because {}", e))
                .ok()
        })
        .inspect(|file| println!("Checking file {:?}", file.path()))
        .filter_map(|file| {
            if let Some(ext) = file.path().extension() {
                if ext == "fesic" {
                    Some(file)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .filter_map(|file| {
            File::open(file.path())
                .inspect_err(|e| eprintln!("Skipping opening file because {}", e))
                .ok()
                .map(|f| (file.path(), f))
        })
        .map(|(file_name, file)| Feusic::from_fesic_file(&file_name, &file))
        .collect::<Result<Vec<_>, _>>()?;

    println!("Playlist of {}", playlist.len());

    let player = match FeusicPlayer::new(playlist) {
        Ok(player) => {
            println!("Music player initialized successfully.");
            player
        }
        Err(e) => {
            return Err(format!("Error initializing music player: {}", e).into());
        }
    };

    player.play();

    ui::run_ui(&player)

    // loop {
    //     println!("Commands: pause, resume, stop, loop, crossfade, next, exit");
    //     io::stdout().flush()?;

    //     let mut command = String::new();
    //     io::stdin().read_line(&mut command)?;
    //     let mut commands = command.trim().split(" ").collect::<Vec<_>>();
    //     let command = commands.remove(0);

    //     println!("Command received: {}", command);

    //     match command {
    //         "pause" => player.pause(),
    //         "resume" => player.resume(),
    //         "stop" => player.stop(),
    //         "next" => player.next(),
    //         "crossfade" => player.crossfade(Duration::from_secs(1)),
    //         "exit" => break,
    //         _ => println!("Unknown command"),
    //     }
    // }

    // Ok(())
}
