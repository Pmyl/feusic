use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        RwLock, RwLockReadGuard,
    },
    time::Duration,
};

pub struct PlayerSharedData {
    pub(super) feusic_duration_in_secs: AtomicUsize,
    pub(super) feusic_position_in_secs: AtomicUsize,
    pub(super) is_paused: AtomicBool,
    pub(super) music_names: RwLock<Vec<String>>,
    pub(super) music_index: AtomicUsize,
}

impl Default for PlayerSharedData {
    fn default() -> Self {
        Self {
            is_paused: AtomicBool::new(true),
            feusic_duration_in_secs: Default::default(),
            feusic_position_in_secs: Default::default(),
            music_names: Default::default(),
            music_index: Default::default(),
        }
    }
}

impl PlayerSharedData {
    pub fn music_duration(&self) -> Duration {
        Duration::from_secs(self.feusic_duration_in_secs.load(Ordering::Relaxed) as u64)
    }

    pub fn music_position(&self) -> Duration {
        Duration::from_secs(self.feusic_position_in_secs.load(Ordering::Relaxed) as u64)
    }

    pub fn paused(&self) -> bool {
        self.is_paused.load(Ordering::Relaxed)
    }

    pub fn music_names<'a>(&'a self) -> SharedDataRef<'a, Vec<String>> {
        SharedDataRef {
            guard: self.music_names.read().unwrap(),
        }
    }

    pub fn music_index(&self) -> usize {
        self.music_index.load(Ordering::Relaxed)
    }

    pub(super) fn reset(&self) {
        self.feusic_duration_in_secs.store(0, Ordering::Relaxed);
        self.feusic_position_in_secs.store(0, Ordering::Relaxed);
        self.is_paused.store(true, Ordering::Relaxed);
        self.music_names.write().unwrap().clear();
        self.music_index.store(0, Ordering::Relaxed);
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
