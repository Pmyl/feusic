mod timer;

use cpal::traits::HostTrait;
use rodio::source::EmptyCallback;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::error::Error;
use std::ops::Sub;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self};
use std::time::Duration;
use timer::FeusicTimer;

use super::feusic::loader::MusicLoader;
use super::feusic::{Feusic, Music};

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
    repeat_count: usize,

    action_sender: Sender<PlayerAction>,
    timer: FeusicTimer,

    stream_handle: OutputStreamHandle,
    _stream: SendOutputStream,
}

struct SendOutputStream(#[allow(unused)] OutputStream);
// OutputStream can't be Send only because Android limitation
// I don't target Android so this is fine
// Also the limitation should probably be lifted https://github.com/RustAudio/cpal/issues/818
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
    NextRepeat,
    SeekToOneSecondLeft,
}

pub struct FeusicPlayerController<M> {
    action_sender: Sender<PlayerAction>,
    player: Arc<Mutex<FeusicPlayer<M>>>,
}

impl<M: MusicLoader> FeusicPlayer<M> {
    pub fn new(
        playlist: Vec<Feusic<M>>,
    ) -> Result<FeusicPlayerController<M>, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No default output device")?;

        let Ok((_stream, stream_handle)) = OutputStream::try_from_device(&device) else {
            return Err("Error finding output device".into());
        };

        let (action_sender, action_receiver) = mpsc::channel();

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
            repeat_count: 0,
        };

        let player = Arc::new(Mutex::new(player));

        run(player.clone(), action_receiver);

        Ok(FeusicPlayerController {
            action_sender,
            player,
        })
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
            let sink = Arc::new(sink);
            self.music_duration = self.load_music_into_sink(music, &sink)?;
            sink.pause();
            sinks.push(sink);
            println!("Loaded audio file: {}", music.name);
        }

        self.repeat_count = 0;
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

    fn seek_to_one_second_left(&mut self) -> Result<(), Box<dyn Error>> {
        for sink in self.sinks.iter() {
            sink.try_seek(self.music_duration.sub(Duration::from_secs(1)))?;
        }

        println!("Seeked to one second before");
        Ok(())
    }

    fn next_repeat(&mut self) -> Result<(), Box<dyn Error>> {
        self.repeat_count += 1;
        let feusic = &self.playlist[self.current_feusic_index];

        if self.repeat_count < feusic.repeat {
            for sink in self.sinks.iter() {
                sink.clear();
            }

            for (music, sink) in feusic.musics.iter().zip(self.sinks.iter()) {
                self.music_duration = self.load_music_into_sink(music, sink)?;
            }

            for sink in self.sinks.iter() {
                sink.play();
            }

            println!("Repeat: {}", self.repeat_count);
            Ok(())
        } else {
            self.next()
        }
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

    fn load_music_into_sink(
        &self,
        music: &Music<M>,
        sink: &Arc<Sink>,
    ) -> Result<Duration, Box<dyn Error>> {
        let sender = self.action_sender.clone();
        let loaded_music = music.loader.read()?;
        let decoder = Decoder::new(loaded_music.reader)?;

        let music_duration = decoder
            .total_duration()
            .unwrap_or_else(|| Duration::from_secs(1));

        sink.append(decoder);
        sink.append(EmptyCallback::<f32>::new(Box::new(move || {
            sender.send(PlayerAction::NextRepeat).unwrap();
        })));

        Ok(music_duration)
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

    pub fn paused(&self) -> bool {
        matches!(self.state, PlayerState::Paused)
    }
}

fn run<M: MusicLoader>(player: Arc<Mutex<FeusicPlayer<M>>>, receiver: Receiver<PlayerAction>) {
    thread::spawn({
        let player = player.clone();
        move || {
            for action in receiver {
                let mut player = player.lock().unwrap();
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
                    PlayerAction::NextRepeat => {
                        if let Err(e) = player.next_repeat() {
                            eprintln!("Error going next: {}", e);
                        }
                    }
                    PlayerAction::SeekToOneSecondLeft => {
                        if let Err(e) = player.seek_to_one_second_left() {
                            eprintln!("Error seeking to one second left: {}", e);
                        }
                    }
                }
                drop(player);
            }
        }
    });

    thread::spawn(move || loop {
        let mut player = player.lock().unwrap();
        player.timer.tick();
        drop(player);

        thread::sleep(Duration::from_millis(1000));
    });
}

impl<M: MusicLoader> FeusicPlayerController<M> {
    pub fn play(&self) {
        self.action_sender.send(PlayerAction::Play).ok();
    }

    pub fn pause(&self) {
        self.action_sender.send(PlayerAction::Pause).ok();
    }

    pub fn resume(&self) {
        self.action_sender.send(PlayerAction::Resume).ok();
    }

    pub fn stop(&self) {
        self.action_sender.send(PlayerAction::Stop).ok();
    }

    pub fn next(&self) {
        self.action_sender.send(PlayerAction::Next).ok();
    }

    pub fn next_repeat(&self) {
        self.action_sender.send(PlayerAction::NextRepeat).ok();
    }

    pub fn crossfade(&self, duration: Duration) {
        self.action_sender
            .send(PlayerAction::CrossfadeNext(duration))
            .ok();
    }

    pub fn seek_to_one_second_left(&self) {
        self.action_sender
            .send(PlayerAction::SeekToOneSecondLeft)
            .ok();
    }

    pub fn music_position(&self) -> Duration {
        let player = self.player.lock().unwrap();
        player.music_position()
    }

    pub fn music_duration(&self) -> Duration {
        let player = self.player.lock().unwrap();
        player.music_duration()
    }

    pub fn paused(&self) -> bool {
        let player = self.player.lock().unwrap();
        player.paused()
    }
}
