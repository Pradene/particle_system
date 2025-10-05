#![allow(unused)]

use std::time::{Duration, Instant};

pub struct Timer {
    start: Instant,
    last_frame: Instant,
}

impl Default for Timer {
    fn default() -> Self {
        let now = Instant::now();

        Self {
            start: now,
            last_frame: now,
        }
    }
}

impl Timer {
    pub fn new() -> Self {
        Timer::default()
    }

    pub fn tick(&mut self) -> f32 {
        let current_time = Instant::now();

        let delta_time = current_time.duration_since(self.last_frame).as_secs_f32();

        self.last_frame = current_time;

        delta_time
    }

    pub fn elapsed(&self) -> Duration {
        Instant::now() - self.start
    }
}
