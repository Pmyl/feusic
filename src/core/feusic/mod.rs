use std::{
    error::Error,
    fs::{self, File},
    io::Read,
    iter::Peekable,
    path::PathBuf,
    str::Chars,
    time::Duration,
};

use loader::FeusicMusicLoader;
use serde::Deserialize;

pub mod loader;

#[derive(Debug)]
pub struct Feusic<M> {
    pub name: String,
    pub musics: Vec<Music<M>>,
    pub first_music: usize,
    pub duration: Duration,
    pub looping: Option<Looping>,
}

#[derive(Debug)]
pub struct Looping {
    pub start: f64,
    pub end: f64,
}

#[derive(Debug)]
pub struct Music<M> {
    pub name: String,
    pub loader: M,
    pub next_choices: Vec<Next>,
}

#[derive(Debug, Clone)]
pub struct Next {
    pub probability_weight: usize,
    pub target_music: usize,
    pub wait: (usize, usize),
}

#[derive(Deserialize)]
struct FeusicConfig {
    timing: String,
    duration: u64,
    loop_start: Option<f64>,
    loop_end: Option<f64>,
}

impl Feusic<FeusicMusicLoader> {
    pub fn from_feusic_zip_file(file_path: &PathBuf, file: &File) -> Result<Self, Box<dyn Error>> {
        println!("Parsing {:?}", file_path);
        let mut zip = zip::ZipArchive::new(file)?;

        let musics_names = zip
            .file_names()
            .filter(|f| f.ends_with(".mp3"))
            .map(|n| n.to_string())
            .collect::<Vec<_>>();

        let mut feusic_toml_file = zip
            .by_name("feusic.toml")
            .map_err(|e| format!("feusic.toml should be in a .feusic file. {}", e))?;

        let mut feusic_toml = String::new();
        feusic_toml_file.read_to_string(&mut feusic_toml)?;

        let config: FeusicConfig = toml::from_str(&feusic_toml)
            .map_err(|e| format!("Failed to read feusic.toml. {}", e))?;

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
            duration: Duration::from_secs(config.duration),
            looping: match (config.loop_start, config.loop_end) {
                (Some(start), Some(end)) => Some(Looping { start, end }),
                _ => None,
            },
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
                        loader: FeusicMusicLoader::ZipFeusic {
                            feusic_path: file_path.to_str().unwrap().to_string(),
                            music_name: name,
                        },
                        next_choices: vec![Next {
                            probability_weight: 100,
                            target_music: target_music_index,
                            wait: wait.ok_or_else(|| "no wait defined")?,
                        }],
                    });
                }

                println!("Loaded feusic {:?}", musics);
                musics
            },
        })
    }

    pub fn from_feusic_folder(folder_path: &PathBuf) -> Result<Self, Box<dyn Error>> {
        println!("Parsing folder {:?}", folder_path);

        let files =
            fs::read_dir(folder_path).map_err(|e| format!("Folder path should exist. {}", e))?;

        let mut musics_names = vec![];
        let mut feusic_toml_file = None;

        for entry in files
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_file())
        {
            let path = entry.path();
            let extension = path.extension();
            println!("Checking {:?} in folder {:?}", path, folder_path);

            if let Some(ext) = extension {
                if ext == "mp3" {
                    musics_names.push(path.to_str().unwrap().to_string());
                } else if let Some(name) = path.file_name() {
                    if name == "feusic.toml" {
                        feusic_toml_file = Some(File::open(&entry.path())?);
                    }
                }
            }
        }

        let mut feusic_toml = String::new();
        feusic_toml_file
            .expect("feusic.toml should be in the .feusic folder")
            .read_to_string(&mut feusic_toml)?;

        let config: FeusicConfig = toml::from_str(&feusic_toml)
            .map_err(|e| format!("Failed to read feusic.toml. {}", e))?;

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
            name: folder_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            duration: Duration::from_secs(config.duration),
            looping: match (config.loop_start, config.loop_end) {
                (Some(start), Some(end)) => Some(Looping { start, end }),
                _ => None,
            },
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
                        loader: FeusicMusicLoader::FolderFeusic { music_path: name },
                        next_choices: vec![Next {
                            probability_weight: 100,
                            target_music: target_music_index,
                            wait: wait.ok_or_else(|| "no wait defined")?,
                        }],
                    });
                }

                println!("Loaded feusic {:?}", musics);
                musics
            },
        })
    }
}
