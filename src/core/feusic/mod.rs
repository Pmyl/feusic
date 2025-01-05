use std::{error::Error, fs::File, io::Read, iter::Peekable, path::PathBuf, str::Chars};

use loader::FesicMusicLoader;
use serde::Deserialize;

pub mod loader;

#[derive(Debug)]
pub struct Phasic<MusicLoader> {
    pub name: String,
    pub musics: Vec<Music<MusicLoader>>,
    pub first_music: usize,
    pub repeat: usize,
}

#[derive(Debug)]
pub struct Music<MusicLoader> {
    pub name: String,
    pub loader: MusicLoader,
    pub next_choices: Vec<Next>,
}

#[derive(Debug, Clone)]
pub struct Next {
    pub probability_weight: usize,
    pub target_music: usize,
    pub wait: (usize, usize),
}

#[derive(Deserialize)]
struct FesicConfig {
    timing: String,
    repeat: usize,
}

impl Phasic<FesicMusicLoader> {
    pub fn from_fesic_file(file_path: &PathBuf, file: &File) -> Result<Self, Box<dyn Error>> {
        println!("Parsing {:?}", file_path);
        let mut zip = zip::ZipArchive::new(file)?;

        let musics_names = zip
            .file_names()
            .filter(|f| f.ends_with(".mp3"))
            .map(|n| n.to_string())
            .collect::<Vec<_>>();

        let mut fesic_toml_file = zip
            .by_name("fesic.toml")
            .map_err(|e| format!("fesic.toml should be in a .fesic file. {}", e))?;

        let mut fesic_toml = String::new();
        fesic_toml_file.read_to_string(&mut fesic_toml)?;

        let config: FesicConfig =
            toml::from_str(&fesic_toml).map_err(|e| format!("Failed to read fesic.toml. {}", e))?;

        fn read_number(chars: &mut Peekable<Chars>) -> Result<usize, Box<dyn Error>> {
            let mut n: usize = 0;
            let mut found = false;

            while let Some(maybe_number) = chars.peek() {
                if let Some(number) = maybe_number.to_digit(10) {
                    found = true;
                    n = n * 10 + number as usize;
                    chars.next();
                } else {
                    break;
                }
            }

            if found {
                Ok(n)
            } else {
                Err("Expected number".into())
            }
        }

        let mut chars = config.timing.chars().peekable();

        match chars.next() {
            Some('s') => {}
            _ => return Err("Timing should start with 's'".into()),
        }

        let first_music = read_number(&mut chars)?;

        Ok(Self {
            name: file_path.file_name().unwrap().to_str().unwrap().to_string(),
            repeat: config.repeat,
            first_music,
            musics: {
                let mut musics = vec![];

                loop {
                    let music_index;
                    let wait;

                    match chars.next() {
                        Some('|') => {}
                        _ => break,
                    }

                    music_index = read_number(&mut chars)?;

                    match chars.next() {
                        Some(':') => {}
                        _ => return Err("Expected ':'".into()),
                    }

                    match chars.next() {
                        Some('w') => {
                            let wait_lower = read_number(&mut chars)?;
                            let wait_higher = match chars.peek() {
                                Some('-') => {
                                    chars.next();
                                    read_number(&mut chars)?
                                }
                                _ => wait_lower,
                            };
                            wait = Some((wait_lower, wait_higher));
                        }
                        Some('p') => {
                            todo!("implement parsing of probability weight")
                        }
                        _ => return Err("Expected either 'w' or 'p'".into()),
                    }

                    match chars.next() {
                        Some(':') => {}
                        _ => return Err("Expected ':'".into()),
                    }

                    let target_music_index = read_number(&mut chars)?;

                    if musics_names.len() < target_music_index {
                        return Err(
                            format!("target index {} does not exists", target_music_index).into(),
                        );
                    }

                    let name = musics_names
                        .get(music_index)
                        .map(|s| s.to_string())
                        .ok_or_else(|| format!("No music at index {}", music_index))?;

                    musics.push(Music {
                        name: name.clone(),
                        loader: FesicMusicLoader {
                            fesic_path: file_path.to_str().unwrap().to_string(),
                            music_name: name,
                        },
                        next_choices: vec![Next {
                            probability_weight: 100,
                            target_music: target_music_index,
                            wait: wait.ok_or_else(|| "no wait defined")?,
                        }],
                    });
                }

                println!("Loaded playlist {:?}", musics);
                musics
            },
        })
    }
}
