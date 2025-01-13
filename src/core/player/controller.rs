use std::{
    sync::{mpsc::Sender, Arc},
    thread,
    time::Duration,
};

use crate::core::feusic::loader::MusicLoader;

use super::{shared_data::SharedDataRef, FeusicPlayer, PlayerAction, PlayerSharedData};

pub struct FeusicPlayerController {
    action_sender: Sender<PlayerAction>,
    shared_data: Arc<PlayerSharedData>,
}

impl FeusicPlayerController {
    pub fn new<M: MusicLoader>(player: FeusicPlayer<M>) -> Self {
        let action_sender = player.action_sender.clone();
        let shared_data = player.shared_data();

        let controller = Self {
            shared_data,
            action_sender,
        };

        controller.run(player);

        controller
    }

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

    pub fn crossfade(&self, duration: Duration) {
        self.action_sender
            .send(PlayerAction::CrossfadeNext(duration))
            .ok();
    }

    pub fn seek(&self, duration: Duration) {
        self.action_sender.send(PlayerAction::Seek(duration)).ok();
    }

    pub fn remove_loop(&self) {
        self.action_sender.send(PlayerAction::RemoveLoop).ok();
    }

    pub fn music_position(&self) -> Duration {
        self.shared_data.music_position()
    }

    pub fn music_duration(&self) -> Duration {
        self.shared_data.music_duration()
    }

    pub fn paused(&self) -> bool {
        self.shared_data.paused()
    }

    pub fn music_names<'a>(&'a self) -> SharedDataRef<'a, Vec<String>> {
        self.shared_data.music_names()
    }

    pub fn music_index(&self) -> usize {
        self.shared_data.music_index()
    }

    fn run<M: MusicLoader>(&self, mut player: FeusicPlayer<M>) {
        thread::spawn(move || loop {
            player.tick();
            thread::sleep(Duration::from_millis(50));
        });
    }
}
