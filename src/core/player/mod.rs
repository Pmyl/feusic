pub mod controller;
mod read_seek_source;
mod timer;

use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use kira::sound::{FromFileError, PlaybackState};
use kira::track::{TrackBuilder, TrackHandle};
use kira::{AudioManager, AudioManagerSettings, Decibels, Easing, PlaybackRate, StartTime, Tween};
use read_seek_source::ReadSeekSource;
use std::error::Error;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;
use timer::FeusicTimer;

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

    audio_manager: AudioManager,
    tracks: Vec<(TrackHandle, StreamingSoundHandle<FromFileError>)>,
    current_track_index: usize,
    music_duration: Duration,

    pub(super) action_sender: Sender<PlayerAction>,
    action_receiver: Receiver<PlayerAction>,
    timer: FeusicTimer,
}

const INSTANT_TWEEN: Tween = Tween {
    duration: Duration::from_millis(0),
    easing: Easing::Linear,
    start_time: StartTime::Immediate,
};

#[derive(Debug)]
pub(super) enum PlayerAction {
    Play,
    Pause,
    Resume,
    Stop,
    Next,
    CrossfadeNext(Duration),
    CrossfadeWith(Duration, usize),
    Seek(Duration),
    RemoveLoop,
}

impl<M: MusicLoader> FeusicPlayer<M> {
    pub fn new(playlist: Vec<Feusic<M>>) -> Result<FeusicPlayer<M>, Box<dyn std::error::Error>> {
        let (action_sender, action_receiver) = mpsc::channel();
        let manager = AudioManager::new(AudioManagerSettings::default())?;

        Ok(Self {
            playlist,
            current_track_index: 0,
            current_feusic_index: 0,
            action_sender: action_sender.clone(),
            action_receiver,
            timer: FeusicTimer::new(action_sender.clone(), 0, Duration::from_secs(0), vec![]),
            state: PlayerState::Stopped,
            music_duration: Duration::from_secs(0),

            audio_manager: manager,
            tracks: vec![],
        })
    }

    fn play_feusic(&mut self, feusic_index: usize) -> Result<(), Box<dyn Error>> {
        self.tracks.drain(..);

        let feusic = &self.playlist[feusic_index];

        self.current_track_index = feusic.first_music;
        self.current_feusic_index = feusic_index;

        let mut tracks = Vec::new();
        for music in &self.playlist[feusic_index].musics {
            let mut track = self.audio_manager.add_sub_track(TrackBuilder::default())?;
            let loaded_music = music.loader.read()?;
            let media_source = ReadSeekSource::new(loaded_music.reader);
            let sound_data = StreamingSoundData::from_media_source(media_source)?;
            self.music_duration = sound_data.duration();

            let mut handle = track.play(sound_data)?;
            if let Some(looping) = &feusic.looping {
                handle.set_loop_region(looping.start..looping.end);
            } else {
                handle.set_loop_region(..);
            }
            handle.pause(INSTANT_TWEEN);

            tracks.push((track, handle));
            println!("Loaded audio file: {}", music.name);
        }

        self.tracks = tracks;
        self.timer = FeusicTimer::new(
            self.action_sender.clone(),
            self.current_track_index,
            feusic.duration,
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
        if self.tracks.is_empty() {
            self.play_feusic_by_name(&self.playlist[0].name.clone())?;
        } else {
            self.play_internal();
        }

        Ok(())
    }

    fn play_internal(&mut self) {
        for (i, (_, handle)) in self.tracks.iter_mut().enumerate() {
            if self.current_track_index == i {
                println!(
                    "Play audio {} at volume 1",
                    self.playlist[self.current_feusic_index].musics[i].name
                );
                handle.set_volume(Decibels::IDENTITY, INSTANT_TWEEN);
            } else {
                println!(
                    "Play audio {} at volume 0",
                    self.playlist[self.current_feusic_index].musics[i].name
                );
                handle.set_volume(Decibels::SILENCE, INSTANT_TWEEN);
            }
            handle.resume(INSTANT_TWEEN);
        }
        self.state = PlayerState::Playing;
    }

    fn seek(&mut self, duration: Duration) {
        for (_, handle) in self.tracks.iter_mut() {
            handle.seek_to(duration.as_secs_f64());
        }

        println!("Seeked to {:?}", duration);
    }

    fn remove_loop(&mut self) {
        for (_, handle) in self.tracks.iter_mut() {
            handle.set_loop_region(None);
        }
        println!("Removed loop");
    }

    fn pause(&mut self) {
        for (_, handle) in self.tracks.iter_mut() {
            println!("Pausing audio.");
            handle.pause(INSTANT_TWEEN);
        }
        self.state = PlayerState::Paused;
    }

    fn resume(&mut self) -> Result<(), Box<dyn Error>> {
        self.play()
    }

    fn stop(&mut self) {
        for (_, handle) in self.tracks.iter_mut() {
            println!("Stopping audio.");
            handle.stop(INSTANT_TWEEN);
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
        self.crossfade_with(duration, (self.current_track_index + 1) % self.tracks.len())
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
                if next_music_index >= self.tracks.len() {
                    eprintln!("Target music index {} does not exists.", next_music_index);
                    return Ok(());
                }

                if self.tracks.len() < 2 {
                    println!("Crossfade requires at least two audio files in a feusic.");
                    return Ok(());
                }

                self.tracks
                    .get_mut(next_music_index)
                    .map(|(_, next_handle)| {
                        next_handle.set_volume(
                            Decibels::IDENTITY,
                            Tween {
                                duration,
                                ..Default::default()
                            },
                        )
                    });
                self.tracks
                    .get_mut(self.current_track_index)
                    .map(|(_, current_handle)| {
                        current_handle.set_volume(
                            Decibels::SILENCE,
                            Tween {
                                duration,
                                ..Default::default()
                            },
                        )
                    });

                println!("Crossfade");

                self.current_track_index = next_music_index;
            }
        }

        Ok(())
    }

    pub fn tick(&mut self) {
        self.timer.tick();

        if self
            .tracks
            .get(0)
            .map(|(_, handle)| matches!(handle.state(), PlaybackState::Stopped))
            .unwrap_or(false)
        {
            self.action_sender.send(PlayerAction::Next).ok();
        }

        for action in self
            .action_receiver
            .try_iter()
            .collect::<Vec<PlayerAction>>()
        {
            match action {
                PlayerAction::Play => {
                    if let Err(e) = self.play() {
                        eprintln!("Error playing: {}", e);
                    }
                }
                PlayerAction::Pause => {
                    self.pause();
                }
                PlayerAction::Resume => {
                    if let Err(e) = self.resume() {
                        eprintln!("Error resuming: {}", e);
                    }
                }
                PlayerAction::Stop => {
                    self.stop();
                }
                PlayerAction::Next => {
                    if let Err(e) = self.next() {
                        eprintln!("Error playing next: {}", e);
                    }
                }
                PlayerAction::CrossfadeNext(duration) => {
                    if let Err(e) = self.crossfade_next(duration) {
                        eprintln!("Error crossfading next: {}", e);
                    }
                }
                PlayerAction::CrossfadeWith(duration, index) => {
                    if let Err(e) = self.crossfade_with(duration, index) {
                        eprintln!("Error crossfading index {}: {}", index, e);
                    }
                }
                PlayerAction::Seek(duration) => {
                    self.seek(duration);
                }
                PlayerAction::RemoveLoop => {
                    self.remove_loop();
                }
            }
        }
    }

    pub fn music_position(&self) -> Duration {
        self.tracks
            .get(0)
            .map(|(_, handle)| Duration::from_secs_f64(handle.position()))
            .unwrap_or(Duration::from_secs(0))
    }

    pub fn music_duration(&self) -> Duration {
        self.music_duration
    }

    pub fn paused(&self) -> bool {
        matches!(self.state, PlayerState::Paused)
    }
}
