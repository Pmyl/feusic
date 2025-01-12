use std::{
    sync::{mpsc::Sender, Arc, Mutex},
    thread,
    time::Duration,
};

use crate::core::feusic::loader::MusicLoader;

use super::{FeusicPlayer, PlayerAction};

pub struct FeusicPlayerController<M> {
    action_sender: Sender<PlayerAction>,
    player: Arc<Mutex<FeusicPlayer<M>>>,
}

impl<M: MusicLoader> FeusicPlayerController<M> {
    pub fn new(player: FeusicPlayer<M>) -> Self {
        let action_sender = player.action_sender.clone();
        let player = Arc::new(Mutex::new(player));

        let controller = Self {
            player,
            action_sender,
        };

        controller.run();

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

    fn run(&self) {
        let player = self.player.clone();

        thread::spawn(move || loop {
            let mut player = player.lock().unwrap();
            player.tick();
            drop(player);

            thread::sleep(Duration::from_millis(50));
        });
    }
}
