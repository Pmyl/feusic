use std::{
    sync::mpsc::Sender,
    time::{Duration, Instant},
};

use rand::{thread_rng, Rng};

use crate::core::{feusic::Next, player::PlayerAction};

pub struct FeusicTimer {
    timings: Vec<Vec<Next>>,
    timing_index: usize,
    case_index: usize,
    sender: Sender<PlayerAction>,
    change_time: Instant,
    running: bool,
}

const CROSSFADE_TIME_FIRE_EMBLEM: Duration = Duration::from_millis(1000);

impl FeusicTimer {
    pub fn new(sender: Sender<PlayerAction>, start: usize, timing: Vec<Vec<Next>>) -> Self {
        println!("Start timer");

        let running = !timing.is_empty();

        if running {
            let mut timer = Self {
                sender,
                timing_index: start,
                case_index: Self::find_next_case_index(&timing[start]),
                running,
                timings: timing,
                change_time: Instant::now(),
            };

            timer.wait_intil_next_change();
            timer
        } else {
            Self {
                sender,
                timing_index: 0,
                case_index: 0,
                running: false,
                timings: vec![],
                change_time: Instant::now(),
            }
        }
    }

    pub fn tick(&mut self) {
        if !self.running {
            return;
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

        self.wait_intil_next_change();
    }

    fn wait_intil_next_change(&mut self) {
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
