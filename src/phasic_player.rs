use cpal::traits::HostTrait;
use rand::{thread_rng, Rng};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::iter::Peekable;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::Chars;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use zip::ZipArchive;

#[derive(Debug)]
pub struct Phasic<MusicLoader> {
    pub name: String,
    pub musics: Vec<Music<MusicLoader>>,
    pub first_music: usize,
    pub repeat: usize,
}

pub trait MusicLoader {
    fn load_to_sink(&self, sink: &Sink) -> Result<(), Box<dyn Error>>;
}

#[derive(Debug)]
pub struct FesicMusicLoader {
    pub fesic_path: String,
    pub music_name: String,
}

impl MusicLoader for FesicMusicLoader {
    fn load_to_sink(&self, sink: &Sink) -> Result<(), Box<dyn Error>> {
        let file = File::open(&self.fesic_path)
            .map_err(|e| format!("cannot open {}. {}", self.fesic_path, e))?;
        let mut zip = ZipArchive::new(file)
            .map_err(|e| format!("cannot open zip file {}. {}", self.fesic_path, e))?;
        let mut music = zip.by_name(&self.music_name).map_err(|e| {
            format!(
                "cannot find {} in zip file {}. {}",
                self.music_name, self.fesic_path, e
            )
        })?;

        let mut buf = vec![];
        music.read_to_end(&mut buf)?;

        let source = Decoder::new(BufReader::new(Cursor::new(buf)))?;
        sink.append(source);

        Ok(())
    }
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

pub struct PhasicPlayer<MusicLoader> {
    sinks: Vec<Sink>,
    current_sink_index: usize,

    playlist: Vec<Phasic<MusicLoader>>,
    current_phasic_index: usize,

    action_sender: Sender<PlayerAction>,
    timer: PhasicTimer,

    stream_handle: OutputStreamHandle,
    _stream: OutputStream,
}

#[derive(Clone, Debug)]
pub enum PhasicTiming {
    Off,
    FireEmblem,
    Balatro,
}

impl TryFrom<&str> for PhasicTiming {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Fire Emblem" => Ok(Self::FireEmblem),
            "Balatro" => Ok(Self::Balatro),
            _ => Err(format!("Invalid timing {}", value)),
        }
    }
}

struct PhasicTimer {
    _handle: JoinHandle<()>,
    drop_sender: Sender<()>,
}

const CROSSFADE_TIME_FIRE_EMBLEM: Duration = Duration::from_millis(1000);

impl PhasicTimer {
    fn new(sender: Sender<PlayerAction>, start: usize, timing: Vec<Vec<Next>>) -> Self {
        println!("Start timer");
        let (drop_sender, drop_receiver) = mpsc::channel();

        Self {
            drop_sender,
            _handle: thread::spawn(move || {
                if timing.is_empty() {
                    println!("End empty timer");
                    return;
                }

                let mut current = &timing[start];

                loop {
                    let mut probability_total = current.iter().map(|c| c.probability_weight).sum();
                    let mut random_probability = thread_rng().gen_range(0..probability_total);

                    let case = current
                        .iter()
                        .find(|c| {
                            if random_probability < c.probability_weight {
                                true
                            } else {
                                random_probability -= c.probability_weight;
                                false
                            }
                        })
                        .unwrap();

                    let time_to_wait = thread_rng().gen_range(case.wait.0..=case.wait.1);
                    println!("TIMING:wait:{}", time_to_wait);
                    thread::sleep(Duration::from_millis(time_to_wait as u64));

                    if let Ok(_) = drop_receiver.try_recv() {
                        break;
                    }
                    println!("TIMING:goto:{}", case.target_music);
                    sender
                        .send(PlayerAction::CrossfadeWith(
                            CROSSFADE_TIME_FIRE_EMBLEM,
                            case.target_music,
                        ))
                        .unwrap();

                    current = &timing[case.target_music];
                }

                println!("End timer");
            }),
        }
    }
}

impl Drop for PhasicTimer {
    fn drop(&mut self) {
        self.drop_sender.send(()).ok();
    }
}

enum PlayerAction {
    Play,
    Pause,
    Resume,
    Stop,
    Next,
    CrossfadeNext(Duration),
    CrossfadeWith(Duration, usize),
}

pub struct PhasicPlayerController {
    action_sender: Sender<PlayerAction>,
}

impl PhasicPlayerController {
    pub fn play(&self) {
        self.action_sender.send(PlayerAction::Play).unwrap();
    }

    pub fn pause(&self) {
        self.action_sender.send(PlayerAction::Pause).unwrap();
    }

    pub fn resume(&self) {
        self.action_sender.send(PlayerAction::Resume).unwrap();
    }

    pub fn stop(&self) {
        self.action_sender.send(PlayerAction::Stop).unwrap();
    }

    pub fn next(&self) {
        self.action_sender.send(PlayerAction::Next).unwrap();
    }

    pub fn crossfade(&self, duration: Duration) {
        self.action_sender
            .send(PlayerAction::CrossfadeNext(duration))
            .unwrap();
    }
}

impl<M: MusicLoader + Send + Sync + 'static> PhasicPlayer<M> {
    pub fn new(
        playlist: Vec<Phasic<M>>,
    ) -> Result<PhasicPlayerController, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No default output device")?;

        let (action_sender, action_receiver) = mpsc::channel();

        let controller = PhasicPlayerController {
            action_sender: action_sender.clone(),
        };

        thread::spawn(move || {
            let Ok((_stream, stream_handle)) = OutputStream::try_from_device(&device) else {
                eprintln!("Error finding output device");
                return;
            };

            let mut player = Self {
                sinks: vec![],
                _stream,
                stream_handle,
                playlist,
                current_sink_index: 0,
                current_phasic_index: 0,
                action_sender: action_sender.clone(),
                timer: PhasicTimer::new(action_sender.clone(), 0, vec![]),
            };

            player.run(action_receiver);
        });

        Ok(controller)
    }

    fn play_phasic(&mut self, phasic_name: &str) -> Result<(), Box<dyn Error>> {
        if !self.sinks.is_empty() {
            self.fade_out(Duration::from_millis(2000));

            for sink in &self.sinks {
                sink.clear();
            }
        }

        let phasic_i = self
            .playlist
            .iter()
            .position(|p| p.name == phasic_name)
            .ok_or_else(|| format!("Cannot find phasic with name {}", phasic_name))?;
        let phasic = &self.playlist[phasic_i];

        self.current_sink_index = phasic.first_music;
        self.current_phasic_index = phasic_i;

        let mut sinks = Vec::new();
        for music in &self.playlist[phasic_i].musics {
            let sink = Sink::try_new(&self.stream_handle)?;
            music.loader.load_to_sink(&sink)?;
            // let repeating_source = RepeatN::new(source, phasic.repeat, {
            //     let sender = self.action_sender.clone();
            //     move || {
            //         sender.send(PlayerAction::Next).unwrap();
            //     }
            // });
            // sink.append(repeating_source);
            sink.pause();
            sinks.push(sink);
            println!("Loaded audio file: {}", music.name);
        }

        self.sinks = sinks;
        self.timer = PhasicTimer::new(
            self.action_sender.clone(),
            self.current_sink_index,
            phasic
                .musics
                .iter()
                .map(|m| m.next_choices.clone())
                .collect::<Vec<_>>(),
        );
        self.play_internal();

        Ok(())
    }

    pub fn play(&mut self) -> Result<(), Box<dyn Error>> {
        if self.sinks.is_empty() {
            self.play_phasic(&self.playlist[0].name.clone())?;
        } else {
            self.play_internal();
        }

        Ok(())
    }

    fn play_internal(&mut self) {
        for (i, sink) in self.sinks.iter().enumerate() {
            if self.current_sink_index == i {
                println!(
                    "Play audio {} at volume 1",
                    self.playlist[self.current_phasic_index].musics[i].name
                );
                sink.set_volume(1.0);
            } else {
                println!(
                    "Play audio {} at volume 0",
                    self.playlist[self.current_phasic_index].musics[i].name
                );
                sink.set_volume(0.0);
            }
            sink.play();
        }
    }

    pub fn pause(&self) {
        for sink in &self.sinks {
            println!("Pausing audio.");
            sink.pause();
        }
    }

    pub fn resume(&self) {
        for sink in &self.sinks {
            println!("Resuming audio");
            sink.play();
        }
    }

    pub fn stop(&self) {
        for sink in &self.sinks {
            println!("Stopping audio.");
            sink.stop();
        }
    }

    pub fn next(&mut self) -> Result<(), Box<dyn Error>> {
        self.play_phasic(
            self.playlist[(self.current_phasic_index + 1) % self.playlist.len()]
                .name
                .clone()
                .as_str(),
        )
    }

    pub fn crossfade_next(&mut self, duration: Duration) {
        self.crossfade_with(duration, (self.current_sink_index + 1) % self.sinks.len());
    }

    pub fn crossfade_with(&mut self, duration: Duration, music_index: usize) {
        println!("Crossfading...");

        if music_index >= self.sinks.len() {
            eprintln!("Target music index {} does not exists.", music_index);
            return;
        }

        if self.sinks.len() < 2 {
            println!("Crossfade requires at least two audio files in a phasic.");
            return;
        }

        let steps = 100;
        let step_duration = duration / steps;

        let next_sink_index = music_index;

        let current_sink = &self.sinks[self.current_sink_index];
        let next_sink = &self.sinks[next_sink_index];

        for i in 0..=steps {
            let volume1 = 1.0 - (i as f32 / steps as f32);
            let volume2 = i as f32 / steps as f32;

            current_sink.set_volume(volume1);
            next_sink.set_volume(volume2);

            thread::sleep(step_duration);
        }

        self.current_sink_index = next_sink_index;

        println!("Crossfade complete.");
    }

    pub fn fade_out(&mut self, duration: Duration) {
        println!("Fading out...");

        let steps = 100;
        let step_duration = duration / steps;

        let current_sink = &self.sinks[self.current_sink_index];

        for i in 0..=steps {
            let volume = 1.0 - (i as f32 / steps as f32);
            current_sink.set_volume(volume);
            thread::sleep(step_duration);
        }

        println!("Fade out complete.");
    }

    fn run(&mut self, receiver: Receiver<PlayerAction>) {
        for action in receiver {
            match action {
                PlayerAction::Play => {
                    if let Err(e) = self.play() {
                        eprintln!("Error playing: {}", e);
                    }
                }
                PlayerAction::Pause => self.pause(),
                PlayerAction::Resume => self.resume(),
                PlayerAction::Stop => self.stop(),
                PlayerAction::Next => {
                    if let Err(e) = self.next() {
                        eprintln!("Error playing next: {}", e);
                    }
                }
                PlayerAction::CrossfadeNext(duration) => self.crossfade_next(duration),
                PlayerAction::CrossfadeWith(duration, index) => {
                    self.crossfade_with(duration, index)
                }
            }
        }
    }
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

#[derive(Deserialize)]
struct FesicConfig {
    timing: String,
    repeat: usize,
}
