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
    pub looping: Looping,
}

#[derive(Debug)]
pub enum Looping {
    Whole(Duration),
    Partial {
        duration: Duration,
        start: f64,
        end: f64,
    },
    None,
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

        let mut feusic_toml = String::new();
        zip.by_name("feusic.toml")
            .map_err(|e| format!("feusic.toml should be in a .feusic file. {}", e))?
            .read_to_string(&mut feusic_toml)?;

        let feusic_path = file_path.file_name().unwrap().to_str().unwrap().to_string();

        Self::from_feusic(
            feusic_path.clone(),
            &musics_names,
            feusic_toml,
            |_, music_name| FeusicMusicLoader::ZipFeusic {
                feusic_path: feusic_path.clone(),
                music_name,
            },
        )
        .inspect(|feusic| println!("Loaded musics {:?}", feusic.musics))
    }

    pub fn from_feusic_folder(folder_path: &PathBuf) -> Result<Self, Box<dyn Error>> {
        println!("Parsing folder {:?}", folder_path);

        let files =
            fs::read_dir(folder_path).map_err(|e| format!("Folder path should exist. {}", e))?;

        let mut musics_paths = vec![];
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
                    musics_paths.push(path.to_str().unwrap().to_string());
                    musics_names.push(path.file_name().unwrap().to_str().unwrap().to_string());
                } else if let Some(name) = path.file_name() {
                    if name == "feusic.toml" {
                        feusic_toml_file = Some(File::open(&entry.path())?);
                    }
                }
            }
        }

        let mut feusic_toml = String::new();
        feusic_toml_file
            .ok_or_else(|| "feusic.toml should be in the .feusic folder")?
            .read_to_string(&mut feusic_toml)?;

        let feusic_path = folder_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        Self::from_feusic(feusic_path, &musics_names, feusic_toml, |music_index, _| {
            FeusicMusicLoader::FolderFeusic {
                music_path: musics_paths[music_index].clone(),
            }
        })
        .inspect(|feusic| println!("Loaded musics {:?}", feusic.musics))
    }

    pub fn from_audio_file(file_path: &PathBuf) -> Result<Self, Box<dyn Error>> {
        println!("Parsing {:?}", file_path);
        let filename = file_path.file_name().unwrap().to_str().unwrap().to_string();

        Ok(Self {
            first_music: 0,
            looping: Looping::None,
            name: filename.clone(),
            musics: vec![Music {
                name: filename,
                next_choices: vec![],
                loader: FeusicMusicLoader::FolderFeusic {
                    music_path: file_path.to_str().unwrap().to_string(),
                },
            }],
        })
        .inspect(|feusic| println!("Loaded musics {:?}", feusic.musics))
    }

    fn from_feusic<F: Fn(usize, String) -> FeusicMusicLoader>(
        feusic_name: String,
        musics_names: &Vec<String>,
        feusic_toml: String,
        music_loader_factory: F,
    ) -> Result<Self, Box<dyn Error>> {
        let config: FeusicConfig = toml::from_str(&feusic_toml)
            .map_err(|e| format!("Failed to read feusic.toml. {}", e))?;

        let parsed_timing = ParsedTiming::try_from(&config.timing.as_str())?;

        Ok(Self {
            name: feusic_name,
            looping: match (config.loop_start, config.loop_end) {
                (Some(start), Some(end)) => Looping::Partial {
                    duration: Duration::from_secs(config.duration),
                    start,
                    end,
                },
                _ => Looping::Whole(Duration::from_secs(config.duration)),
            },
            first_music: parsed_timing.first_music_index,
            musics: parsed_timing
                .timing_musics
                .map(|parsed_timing_music| {
                    let parsed_timing_music = match parsed_timing_music {
                        Ok(pc) => pc,
                        Err(e) => return Err(e),
                    };

                    if parsed_timing_music
                        .choices
                        .iter()
                        .any(|c| c.target_music_index >= musics_names.len())
                    {
                        return Err(format!(
                            "target index {} does not exists",
                            parsed_timing_music
                                .choices
                                .iter()
                                .find(|c| musics_names.len() < c.target_music_index)
                                .unwrap()
                                .target_music_index
                        )
                        .into());
                    }

                    let name = musics_names
                        .get(parsed_timing_music.music_index)
                        .map(|s| s.to_string())
                        .ok_or_else(|| {
                            format!("No music at index {}", parsed_timing_music.music_index)
                        })?;

                    Ok(Music {
                        name: name.clone(),
                        loader: music_loader_factory(parsed_timing_music.music_index, name),
                        next_choices: parsed_timing_music
                            .choices
                            .into_iter()
                            .map(|parsed_choice| parsed_choice.into())
                            .collect::<Vec<_>>(),
                    })
                })
                .collect::<Result<Vec<Music<FeusicMusicLoader>>, Box<dyn Error>>>()?,
        })
    }
}

struct ParsedTiming<'a> {
    first_music_index: usize,
    timing_musics: ParsedTimingMusicIterator<'a>,
}

struct ParsedTimingMusic {
    music_index: usize,
    choices: Vec<ParsedTimingChoice>,
}

struct ParsedTimingChoice {
    probability_weight: usize,
    target_music_index: usize,
    wait: (usize, usize),
}

struct ParsedTimingMusicIterator<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> ParsedTiming<'a> {
    fn try_from(s: &'a str) -> Result<Self, Box<dyn Error>> {
        let mut chars = s.chars().peekable();

        match chars.next() {
            Some('s') => {}
            _ => return Err("Timing should start with 's'".into()),
        }

        let first_music_index = read_number(&mut chars)?;

        Ok(Self {
            first_music_index,
            timing_musics: ParsedTimingMusicIterator { chars },
        })
    }
}

impl<'a> Iterator for ParsedTimingMusicIterator<'a> {
    type Item = Result<ParsedTimingMusic, Box<dyn Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut choices = vec![];

        let mut wait = None;
        let mut probability_weight = None;

        match self.chars.next() {
            Some('|') => {}
            _ => return None,
        }

        let music_index = match read_number(&mut self.chars) {
            Ok(number) => number,
            Err(e) => return Some(Err(e)),
        };

        match self.chars.next() {
            Some(':') => {}
            _ => return Some(Err("Expected ':'".into())),
        }

        let mut choices_with_probability = 0;
        loop {
            match self.chars.next() {
                Some('w') => {
                    let wait_lower = match read_number(&mut self.chars) {
                        Ok(number) => number,
                        Err(e) => return Some(Err(e)),
                    };
                    let wait_higher = match self.chars.peek() {
                        Some('-') => {
                            self.chars.next();
                            match read_number(&mut self.chars) {
                                Ok(number) => number,
                                Err(e) => return Some(Err(e)),
                            }
                        }
                        _ => wait_lower,
                    };
                    wait = Some((wait_lower, wait_higher));
                }
                Some('p') => {
                    choices_with_probability += 1;

                    probability_weight = Some(match read_number(&mut self.chars) {
                        Ok(number) => number,
                        Err(e) => return Some(Err(e)),
                    });
                }
                _ => return Some(Err("Expected either 'w' or 'p'".into())),
            }

            match self.chars.next() {
                Some(':') => {}
                _ => return Some(Err("Expected ':'".into())),
            }

            let target_music_index = match read_number(&mut self.chars) {
                Ok(number) => number,
                Err(e) => return Some(Err(e)),
            };

            let Some(wait) = wait else {
                return Some(Err("no wait defined".into()));
            };

            choices.push(ParsedTimingChoice {
                probability_weight: probability_weight.unwrap_or(100),
                target_music_index,
                wait,
            });

            match self.chars.peek() {
                Some('/') => {
                    self.chars.next();
                }
                _ => break,
            }
        }

        if choices.len() > 1
            && (choices.iter().any(|c| c.probability_weight == 0)
                || choices.len() != choices_with_probability)
        {
            return Some(Err(
                r#"When setting multiple choices they all need probability specified different than 0.
                    e.g. s0|0:w120000-150000:1|1:w15000-60000;p30:2/w5000-10000;p70:0|2:w6000-20000:1"#.into(),
            ));
        }

        Some(Ok(ParsedTimingMusic {
            music_index,
            choices,
        }))
    }
}

impl Into<Next> for ParsedTimingChoice {
    fn into(self) -> Next {
        Next {
            probability_weight: self.probability_weight,
            target_music: self.target_music_index,
            wait: self.wait,
        }
    }
}

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

impl Looping {
    pub fn duration(&self) -> Option<Duration> {
        match self {
            Looping::Whole(duration) => Some(*duration),
            Looping::Partial { duration, .. } => Some(*duration),
            Looping::None => None,
        }
    }
}
