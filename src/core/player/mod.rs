pub mod controller;
mod read_seek_source;
pub mod shared_data;
mod timer;

use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use kira::sound::{FromFileError, PlaybackState};
use kira::track::{TrackBuilder, TrackHandle};
use kira::{AudioManager, AudioManagerSettings, Decibels, Easing, StartTime, Tween};
use read_seek_source::ReadSeekSource;
use shared_data::PlayerSharedData;
use std::error::Error;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;
use timer::FeusicTimer;

use crate::core::feusic::Looping;

use super::feusic::loader::MusicLoader;
use super::feusic::Feusic;

pub enum PlayerState {
    Playing,
    Paused,
    Stopped,
}

pub struct FeusicPlayer<M: MusicLoader> {
    state: PlayerState,

    feusics: Vec<Feusic<M>>,
    current_feusic_index: usize,

    audio_manager: AudioManager,
    musics: Vec<(TrackHandle, StreamingSoundHandle<FromFileError>)>,
    current_music_index: usize,

    pub(super) action_sender: Sender<PlayerAction<M>>,
    action_receiver: Receiver<PlayerAction<M>>,
    timer: FeusicTimer<M>,

    shared_data: Arc<PlayerSharedData>,
}

const INSTANT_TWEEN: Tween = Tween {
    duration: Duration::from_millis(0),
    easing: Easing::Linear,
    start_time: StartTime::Immediate,
};

#[derive(Debug)]
pub(super) enum PlayerAction<M: MusicLoader> {
    Play,
    PlayIndex(usize),
    Pause,
    Resume,
    Stop,
    Next,
    CrossfadeNext(Duration),
    CrossfadeWith(Duration, usize),
    Seek(Duration),
    RemoveLoop,
    SetPlaylist(Vec<Feusic<M>>),
}

impl<M: MusicLoader> FeusicPlayer<M> {
    pub fn new() -> Result<FeusicPlayer<M>, Box<dyn std::error::Error>> {
        let (action_sender, action_receiver) = mpsc::channel();
        let manager = AudioManager::new(AudioManagerSettings::default())?;

        Ok(Self {
            feusics: vec![],
            current_music_index: 0,
            current_feusic_index: 0,
            action_sender: action_sender.clone(),
            action_receiver,
            timer: FeusicTimer::new(action_sender.clone(), 0, None, vec![]),
            state: PlayerState::Stopped,

            audio_manager: manager,
            musics: vec![],
            shared_data: Arc::new(PlayerSharedData::default()),
        })
    }

    pub fn set_playlist(&mut self, playlist: Vec<Feusic<M>>) {
        self.reset();
        self.feusics = playlist;
        *self.shared_data.feusic_names.write().unwrap() =
            self.feusics.iter().map(|f| f.name.clone()).collect();
    }

    fn reset(&mut self) {
        self.shared_data.reset();
        self.feusics.drain(..);
        self.set_current_music_index(0);
        self.set_current_feusic_index(0);
        self.timer.stop();
        self.state = PlayerState::Stopped;
        self.musics.drain(..);
    }

    fn play_feusic(&mut self, feusic_index: usize) -> Result<(), Box<dyn Error>> {
        self.musics.drain(..);

        self.set_current_music_index(self.feusics[feusic_index].first_music);
        self.set_current_feusic_index(feusic_index);

        let feusic = &self.feusics[feusic_index];
        let mut tracks = Vec::new();
        let mut feusic_duration = Duration::from_secs(0);
        for music in &self.feusics[feusic_index].musics {
            let mut track = self.audio_manager.add_sub_track(TrackBuilder::default())?;
            let loaded_music = music.loader.read()?;
            let media_source = ReadSeekSource::new(loaded_music.reader);
            let sound_data = StreamingSoundData::from_media_source(media_source)
                .map_err(|e| format!("When getting streaming sound data: {}", e))?;
            feusic_duration = sound_data.duration();

            let mut handle = track.play(sound_data)?;
            match &feusic.looping {
                Looping::Partial { start, end, .. } => {
                    handle.set_loop_region(*start..*end);
                }
                Looping::Whole(_) => {
                    handle.set_loop_region(..);
                }
                _ => {}
            }
            handle.pause(INSTANT_TWEEN);

            tracks.push((track, handle));
            println!("Loaded audio file: {}", music.name);
        }

        self.musics = tracks;
        self.timer.reset(
            self.current_music_index,
            feusic.looping.duration(),
            feusic
                .musics
                .iter()
                .map(|m| m.next_choices.clone())
                .collect::<Vec<_>>(),
        );
        *self.shared_data.music_names.write().unwrap() =
            feusic.musics.iter().map(|m| m.name.clone()).collect();
        self.shared_data.feusic_duration_in_secs.store(
            feusic_duration.as_secs() as usize,
            std::sync::atomic::Ordering::Relaxed,
        );
        self.play_internal();

        Ok(())
    }

    fn play_feusic_by_name(&mut self, feusic_name: &str) -> Result<(), Box<dyn Error>> {
        self.play_feusic(
            self.feusics
                .iter()
                .position(|p| p.name == feusic_name)
                .ok_or_else(|| format!("Cannot find feusic with name {}", feusic_name))?,
        )
    }

    fn play(&mut self) -> Result<(), Box<dyn Error>> {
        if self.feusics.is_empty() {
            println!("Attempted to play with empty playlist");
        } else if self.musics.is_empty() {
            self.play_feusic_by_name(&self.feusics[0].name.clone())?;
        } else {
            self.play_internal();
        }

        Ok(())
    }

    fn play_internal(&mut self) {
        for (i, (_, handle)) in self.musics.iter_mut().enumerate() {
            if self.current_music_index == i {
                println!(
                    "Play audio {} at volume 1",
                    self.feusics[self.current_feusic_index].musics[i].name
                );
                handle.set_volume(Decibels::IDENTITY, INSTANT_TWEEN);
            } else {
                println!(
                    "Play audio {} at volume 0",
                    self.feusics[self.current_feusic_index].musics[i].name
                );
                handle.set_volume(Decibels::SILENCE, INSTANT_TWEEN);
            }
            handle.resume(INSTANT_TWEEN);
        }
        self.state = PlayerState::Playing;
    }

    fn seek(&mut self, duration: Duration) {
        for (_, handle) in self.musics.iter_mut() {
            handle.seek_to(duration.as_secs_f64());
        }
        println!("Seeked to {:?}", duration);
    }

    fn remove_loop(&mut self) {
        for (_, handle) in self.musics.iter_mut() {
            handle.set_loop_region(None);
        }
        println!("Removed loop");
    }

    fn pause(&mut self) {
        for (_, handle) in self.musics.iter_mut() {
            handle.pause(INSTANT_TWEEN);
        }
        println!("Paused audio.");
        self.state = PlayerState::Paused;
    }

    fn resume(&mut self) -> Result<(), Box<dyn Error>> {
        self.play()
    }

    fn stop(&mut self) {
        for (_, handle) in self.musics.iter_mut() {
            handle.stop(INSTANT_TWEEN);
        }
        println!("Stopped audio.");
        self.state = PlayerState::Stopped;
    }

    fn next(&mut self) -> Result<(), Box<dyn Error>> {
        if self.feusics.is_empty() {
            println!("Attempted to go next with empty playlist");
            Ok(())
        } else {
            self.play_feusic_by_name(
                self.feusics[(self.current_feusic_index + 1) % self.feusics.len()]
                    .name
                    .clone()
                    .as_str(),
            )
        }
    }

    fn crossfade_next(&mut self, duration: Duration) -> Result<(), Box<dyn Error>> {
        if self.musics.is_empty() {
            println!("Attempted to crossfade with no feusic playing");
            Ok(())
        } else {
            self.crossfade_with(duration, (self.current_music_index + 1) % self.musics.len())
        }
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
                if next_music_index >= self.musics.len() {
                    eprintln!("Target music index {} does not exists.", next_music_index);
                    return Ok(());
                }

                if self.musics.len() < 2 {
                    println!("Crossfade requires at least two audio files in a feusic.");
                    return Ok(());
                }

                self.musics
                    .get_mut(next_music_index)
                    .map(|(_, next_handle)| {
                        next_handle.set_volume(
                            Decibels::IDENTITY,
                            Tween {
                                duration,
                                easing: Easing::InPowf(0.15),
                                ..Default::default()
                            },
                        )
                    });
                self.musics
                    .get_mut(self.current_music_index)
                    .map(|(_, current_handle)| {
                        current_handle.set_volume(
                            Decibels::SILENCE,
                            Tween {
                                duration,
                                easing: Easing::OutPowf(0.15),
                                ..Default::default()
                            },
                        )
                    });

                println!("Crossfade");

                self.set_current_music_index(next_music_index);
            }
        }

        Ok(())
    }

    fn set_current_music_index(&mut self, index: usize) {
        self.current_music_index = index;
        self.shared_data
            .music_index
            .store(index, std::sync::atomic::Ordering::Relaxed);
    }

    fn set_current_feusic_index(&mut self, index: usize) {
        self.current_feusic_index = index;
        self.shared_data
            .feusic_index
            .store(index, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn tick(&mut self) {
        self.shared_data.feusic_position_in_secs.store(
            self.music_position().as_secs() as usize,
            std::sync::atomic::Ordering::Relaxed,
        );

        let is_paused = self.paused();
        self.shared_data
            .is_paused
            .store(is_paused, std::sync::atomic::Ordering::Relaxed);

        if !is_paused {
            self.timer.tick();
        }

        if self
            .musics
            .get(0)
            .map(|(_, handle)| matches!(handle.state(), PlaybackState::Stopped))
            .unwrap_or(false)
        {
            self.action_sender.send(PlayerAction::Next).ok();
        }

        for action in self
            .action_receiver
            .try_iter()
            .collect::<Vec<PlayerAction<M>>>()
        {
            match action {
                PlayerAction::Play => {
                    if let Err(e) = self.play() {
                        eprintln!("Error playing: {}", e);
                    }
                }
                PlayerAction::PlayIndex(index) => {
                    if let Err(e) = self.play_feusic(index) {
                        eprintln!("Error playing index {}: {}", index, e);
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
                PlayerAction::SetPlaylist(playlist) => {
                    self.set_playlist(playlist);
                }
            }
        }
    }

    pub fn music_position(&self) -> Duration {
        self.musics
            .get(0)
            .map(|(_, handle)| Duration::from_secs_f64(handle.position()))
            .unwrap_or(Duration::from_secs(0))
    }

    pub fn paused(&self) -> bool {
        matches!(self.state, PlayerState::Paused | PlayerState::Stopped)
    }

    pub fn shared_data(&self) -> Arc<PlayerSharedData> {
        self.shared_data.clone()
    }
}
