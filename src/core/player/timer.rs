use std::{
    sync::mpsc::Sender,
    time::{Duration, Instant},
};

use rand::{thread_rng, Rng};

use crate::core::{
    feusic::{loader::MusicLoader, Next},
    player::PlayerAction,
};

pub struct FeusicTimer<M: MusicLoader> {
    timings: Vec<Vec<Next>>,
    timing_index: usize,
    case_index: usize,
    sender: Sender<PlayerAction<M>>,
    change_time: Instant,
    running: bool,
    last_tick: Instant,
    time_left_secs: Option<f32>,
}

const CROSSFADE_TIME_FIRE_EMBLEM: Duration = Duration::from_millis(1000);

impl<M: MusicLoader> FeusicTimer<M> {
    pub fn new(
        sender: Sender<PlayerAction<M>>,
        start: usize,
        duration: Option<Duration>,
        timings: Vec<Vec<Next>>,
    ) -> Self {
        println!("New timer");

        let running = has_timings(&timings);
        println!("Running timer");

        if running {
            let mut timer = Self {
                sender,
                timing_index: start,
                case_index: Self::find_next_case_index(&timings[start]),
                running,
                timings,
                change_time: Instant::now(),
                last_tick: Instant::now(),
                time_left_secs: duration.map(|duration| duration.as_secs_f32()),
            };

            timer.wait_until_next_change();
            timer
        } else {
            Self {
                sender,
                timing_index: 0,
                case_index: 0,
                running: false,
                timings: vec![],
                change_time: Instant::now(),
                last_tick: Instant::now(),
                time_left_secs: None,
            }
        }
    }

    pub fn reset(&mut self, start: usize, duration: Option<Duration>, timings: Vec<Vec<Next>>) {
        println!("Reset timer");

        self.running = has_timings(&timings);

        if self.running {
            self.case_index = Self::find_next_case_index(&timings[start]);
            self.timing_index = start;
            self.timings = timings;
            self.change_time = Instant::now();
            self.last_tick = Instant::now();
            self.time_left_secs = duration.map(|duration| duration.as_secs_f32());

            self.wait_until_next_change();
        }
    }

    pub fn tick(&mut self) {
        if !self.running {
            return;
        }

        let new_tick = Instant::now();
        let delta_as_secs = (new_tick - self.last_tick).as_secs_f32();

        self.time_left_secs = self
            .time_left_secs
            .map(|time_left| time_left - delta_as_secs);
        self.last_tick = new_tick;

        match self.time_left_secs {
            Some(time_left_secs) if time_left_secs <= 0.0 => {
                println!("TIMING:remove_loop");
                self.sender.send(PlayerAction::RemoveLoop).unwrap();
                self.running = false;
                return;
            }
            _ => {}
        }

        if Instant::now() < self.change_time {
            return;
        }

        let current = &self.timings[self.timing_index];
        let case = &current[self.case_index];

        println!("TIMING:goto:{}", case.target_music);
        self.sender
            .send(PlayerAction::CrossfadeWith(
                CROSSFADE_TIME_FIRE_EMBLEM,
                case.target_music,
            ))
            .unwrap();

        self.timing_index = case.target_music;

        self.wait_until_next_change();
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    fn wait_until_next_change(&mut self) {
        let current = &self.timings[self.timing_index];
        self.case_index = Self::find_next_case_index(current);
        let case = &current[self.case_index];
        let time_to_wait = thread_rng().gen_range(case.wait.0..=case.wait.1);
        println!("TIMING:wait:{}", time_to_wait);
        self.change_time = Instant::now()
            .checked_add(Duration::from_millis(time_to_wait as u64))
            .unwrap();
    }

    fn find_next_case_index(timing: &Vec<Next>) -> usize {
        let probability_total = timing.iter().map(|c| c.probability_weight).sum();
        let mut random_probability = thread_rng().gen_range(0..probability_total);

        timing
            .iter()
            .position(|c| {
                if random_probability < c.probability_weight {
                    true
                } else {
                    random_probability -= c.probability_weight;
                    false
                }
            })
            .unwrap()
    }
}

fn has_timings(timings: &Vec<Vec<Next>>) -> bool {
    timings.iter().any(|c| !c.is_empty())
}
