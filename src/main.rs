#![feature(fn_traits)]

mod phasic_player;
mod source_repeat_n;

use phasic_player::{FesicMusicLoader, Phasic, PhasicPlayer};
use serde::Deserialize;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, Write};
use std::time::Duration;

#[derive(Deserialize)]
struct PlaylistConfig {
    phasic: Vec<PhasicConfig>,
}

#[derive(Deserialize)]
struct PhasicConfig {
    name: String,
    files: Vec<String>,
    timing: String,
    repeat: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let folder_path = &args[1];

    let files =
        fs::read_dir(folder_path).map_err(|e| format!("Folder path should exist. {}", e))?;

    println!("Loading files from folder {}", folder_path);

    let playlist: Vec<Phasic<FesicMusicLoader>> = files
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
        .map(|(file_name, file)| Phasic::from_fesic_file(&file_name, &file))
        .collect::<Result<Vec<_>, _>>()?;

    println!("Playlist of {}", playlist.len());

    // let playlist_content = fs::read_to_string(folder_path).expect("Failed to read playlist.toml");

    // let playlist: PlaylistConfig =
    //     toml::from_str(&playlist_content).expect("Failed to parse playlist.toml");

    // let playlist: Vec<Phasic> = playlist
    //     .phasic
    //     .into_iter()
    //     .map(
    //         |group| group.into(), /*Phasic {
    //                                   name: group.name,
    //                                   audios_paths: group.files,
    //                                   timing: PhasicTiming::try_from(group.timing.as_str()).expect("Invalid timing"),
    //                                   repeat: group.repeat,
    //                               }*/
    //     )
    //     .collect();

    let player = match PhasicPlayer::new(playlist) {
        Ok(player) => {
            println!("Music player initialized successfully.");
            player
        }
        Err(e) => {
            return Err(format!("Error initializing music player: {}", e).into());
        }
    };

    player.play();

    loop {
        println!("Commands: pause, resume, stop, loop, crossfade, next, exit");
        io::stdout().flush()?;

        let mut command = String::new();
        io::stdin().read_line(&mut command)?;
        let mut commands = command.trim().split(" ").collect::<Vec<_>>();
        let command = commands.remove(0);

        println!("Command received: {}", command);

        match command {
            "pause" => player.pause(),
            "resume" => player.resume(),
            "stop" => player.stop(),
            "next" => player.next(),
            "crossfade" => player.crossfade(Duration::from_secs(1)),
            "exit" => break,
            _ => println!("Unknown command"),
        }
    }

    Ok(())
}
