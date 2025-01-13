use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize},
        RwLock, RwLockReadGuard,
    },
    time::Duration,
};

#[derive(Default)]
pub struct PlayerSharedData {
    pub(super) feusic_duration_in_secs: AtomicUsize,
    pub(super) feusic_position_in_secs: AtomicUsize,
    pub(super) is_paused: AtomicBool,
    pub(super) music_names: RwLock<Vec<String>>,
    pub(super) music_index: AtomicUsize,
}

impl PlayerSharedData {
    pub fn music_duration(&self) -> Duration {
        Duration::from_secs(
            self.feusic_duration_in_secs
                .load(std::sync::atomic::Ordering::Relaxed) as u64,
        )
    }

    pub fn music_position(&self) -> Duration {
        Duration::from_secs(
            self.feusic_position_in_secs
                .load(std::sync::atomic::Ordering::Relaxed) as u64,
        )
    }

    pub fn paused(&self) -> bool {
        self.is_paused.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn music_names<'a>(&'a self) -> SharedDataRef<'a, Vec<String>> {
        SharedDataRef {
            guard: self.music_names.read().unwrap(),
        }
    }

    pub fn music_index(&self) -> usize {
        self.music_index.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl<'a, T> SharedDataRef<'a, T> {
    pub fn get(&'a self) -> &'a T {
        &self.guard
    }
}

pub struct SharedDataRef<'a, T> {
    guard: RwLockReadGuard<'a, T>,
}
