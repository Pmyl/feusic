mod timer;

use cpal::traits::HostTrait;
use rand::{thread_rng, Rng};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::iter::Peekable;
use std::path::PathBuf;
use std::str::Chars;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self};
use std::time::{Duration, Instant};
use timer::FeusicTimer;
use zip::ZipArchive;

use super::feusic::loader::MusicLoader;
use super::feusic::Feusic;

pub enum PlayerState {
    Playing,
    Paused,
    Stopped,
}

pub struct FeusicPlayer<M> {
    state: PlayerState,

    playlist: Vec<Feusic<M>>,
    current_feusic_index: usize,

    sinks: Vec<Arc<Sink>>,
    current_sink_index: usize,
    music_duration: Duration,

    action_sender: Sender<PlayerAction>,
    timer: FeusicTimer,

    stream_handle: OutputStreamHandle,
    _stream: SendOutputStream,
}

struct SendOutputStream(OutputStream);
unsafe impl Send for SendOutputStream {}

#[derive(Debug)]
enum PlayerAction {
    Play,
    Pause,
    Resume,
    Stop,
    Next,
    CrossfadeNext(Duration),
    CrossfadeWith(Duration, usize),
}

pub struct FeusicPlayerController<M> {
    action_sender: Sender<PlayerAction>,
    player: Arc<Mutex<FeusicPlayer<M>>>,
}

impl<M: MusicLoader> FeusicPlayerController<M> {
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

    pub fn music_position(&self) -> Duration {
        let player = self.player.lock().unwrap();
        player.music_position()
    }

    pub fn music_duration(&self) -> Duration {
        let player = self.player.lock().unwrap();
        player.music_duration()
    }
}

impl<M: MusicLoader> FeusicPlayer<M> {
    pub fn new(
        playlist: Vec<Feusic<M>>,
    ) -> Result<FeusicPlayerController<M>, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No default output device")?;

        let (action_sender, action_receiver) = mpsc::channel();

        let Ok((_stream, stream_handle)) = OutputStream::try_from_device(&device) else {
            return Err("Error finding output device".into());
        };

        let player = Self {
            sinks: vec![],
            _stream: SendOutputStream(_stream),
            stream_handle,
            playlist,
            current_sink_index: 0,
            current_feusic_index: 0,
            action_sender: action_sender.clone(),
            timer: FeusicTimer::new(action_sender.clone(), 0, vec![]),
            state: PlayerState::Stopped,
            music_duration: Duration::from_secs(0),
        };

        let player_arc = Arc::new(Mutex::new(player));

        run(player_arc.clone(), action_receiver);

        let controller = FeusicPlayerController {
            action_sender,
            player: player_arc,
        };

        Ok(controller)
    }

    fn play_feusic(&mut self, feusic_index: usize) -> Result<(), Box<dyn Error>> {
        if !self.sinks.is_empty() {
            self.fade_out(Duration::from_millis(2000));

            for sink in self.sinks.iter() {
                sink.clear();
            }
        }

        let feusic = &self.playlist[feusic_index];

        self.current_sink_index = feusic.first_music;
        self.current_feusic_index = feusic_index;

        let mut sinks = Vec::new();
        for music in &self.playlist[feusic_index].musics {
            let sink = Sink::try_new(&self.stream_handle)?;
            self.music_duration = music.loader.load_to_sink(&sink)?;
            // let repeating_source = RepeatN::new(source, feusic.repeat, {
            //     let sender = self.action_sender.clone();
            //     move || {
            //         sender.send(PlayerAction::Next).unwrap();
            //     }
            // });
            // sink.append(repeating_source);
            sink.pause();
            sinks.push(Arc::new(sink));
            println!("Loaded audio file: {}", music.name);
        }

        self.sinks = sinks;
        self.timer = FeusicTimer::new(
            self.action_sender.clone(),
            self.current_sink_index,
            feusic
                .musics
                .iter()
                .map(|m| m.next_choices.clone())
                .collect::<Vec<_>>(),
        );
        self.play_internal();

        Ok(())
    }

    fn play_feusic_by_name(&mut self, feusic_name: &str) -> Result<(), Box<dyn Error>> {
        self.play_feusic(
            self.playlist
                .iter()
                .position(|p| p.name == feusic_name)
                .ok_or_else(|| format!("Cannot find feusic with name {}", feusic_name))?,
        )
    }

    fn play(&mut self) -> Result<(), Box<dyn Error>> {
        if self.sinks.is_empty() {
            self.play_feusic_by_name(&self.playlist[0].name.clone())?;
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
                    self.playlist[self.current_feusic_index].musics[i].name
                );
                sink.set_volume(1.0);
            } else {
                println!(
                    "Play audio {} at volume 0",
                    self.playlist[self.current_feusic_index].musics[i].name
                );
                sink.set_volume(0.0);
            }
            sink.play();
        }
        self.state = PlayerState::Playing;
    }

    fn pause(&mut self) {
        for sink in self.sinks.iter() {
            println!("Pausing audio.");
            sink.pause();
        }
        self.state = PlayerState::Paused;
    }

    fn resume(&mut self) -> Result<(), Box<dyn Error>> {
        self.play()
    }

    fn stop(&mut self) {
        for sink in self.sinks.iter() {
            println!("Stopping audio.");
            sink.stop();
        }
        self.state = PlayerState::Stopped;
    }

    fn next(&mut self) -> Result<(), Box<dyn Error>> {
        self.play_feusic_by_name(
            self.playlist[(self.current_feusic_index + 1) % self.playlist.len()]
                .name
                .clone()
                .as_str(),
        )
    }

    fn crossfade_next(&mut self, duration: Duration) -> Result<(), Box<dyn Error>> {
        self.crossfade_with(duration, (self.current_sink_index + 1) % self.sinks.len())
    }

    fn crossfade_with(
        &mut self,
        duration: Duration,
        next_music_index: usize,
    ) -> Result<(), Box<dyn Error>> {
        match self.state {
            PlayerState::Paused => {
                println!("Not crossfading, paused");
            }
            PlayerState::Stopped => {
                println!("Not crossfading, stopped");
            }
            PlayerState::Playing => {
                if next_music_index >= self.sinks.len() {
                    eprintln!("Target music index {} does not exists.", next_music_index);
                    return Ok(());
                }

                if self.sinks.len() < 2 {
                    println!("Crossfade requires at least two audio files in a feusic.");
                    return Ok(());
                }

                let current_sink = self.sinks[self.current_sink_index].clone();
                let next_sink = self.sinks[next_music_index].clone();

                thread::spawn(move || {
                    println!("Crossfading...");

                    let steps = 100;
                    let step_duration = duration / steps;

                    for i in 0..=steps {
                        let volume1 = 1.0 - (i as f32 / steps as f32);
                        let volume2 = i as f32 / steps as f32;

                        current_sink.set_volume(volume1);
                        next_sink.set_volume(volume2);

                        thread::sleep(step_duration);
                    }

                    println!("Crossfade complete.");
                });

                self.current_sink_index = next_music_index;
            }
        }

        Ok(())
    }

    fn fade_out(&mut self, duration: Duration) {
        // TODO: fading out should be done in its own thread
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

    pub fn music_position(&self) -> Duration {
        self.sinks
            .get(0)
            .map(|s| s.get_pos())
            .unwrap_or(Duration::from_secs(0))
    }

    pub fn music_duration(&self) -> Duration {
        self.music_duration
    }
}

fn run<M: MusicLoader>(player: Arc<Mutex<FeusicPlayer<M>>>, receiver: Receiver<PlayerAction>) {
    thread::spawn(move || loop {
        for action in receiver.try_iter() {
            println!("Received {:?} locking", action);
            let mut player = player.lock().unwrap();
            println!("Locked");
            match action {
                PlayerAction::Play => {
                    if let Err(e) = player.play() {
                        eprintln!("Error playing: {}", e);
                    }
                }
                PlayerAction::Pause => {
                    player.pause();
                }
                PlayerAction::Resume => {
                    if let Err(e) = player.resume() {
                        eprintln!("Error resuming: {}", e);
                    }
                }
                PlayerAction::Stop => {
                    player.stop();
                }
                PlayerAction::Next => {
                    if let Err(e) = player.next() {
                        eprintln!("Error playing next: {}", e);
                    }
                }
                PlayerAction::CrossfadeNext(duration) => {
                    if let Err(e) = player.crossfade_next(duration) {
                        eprintln!("Error crossfading next: {}", e);
                    }
                }
                PlayerAction::CrossfadeWith(duration, index) => {
                    if let Err(e) = player.crossfade_with(duration, index) {
                        eprintln!("Error crossfading index {}: {}", index, e);
                    }
                }
            }
            drop(player);
        }

        let mut player = player.lock().unwrap();
        player.timer.tick();
        drop(player);

        thread::sleep(Duration::from_millis(1000));
    });
}
