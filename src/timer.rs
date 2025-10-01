use std::time::Instant;

pub struct Timer {
    last_frame_time: Instant,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            last_frame_time: Instant::now(),
        }
    }
}

impl Timer {
    pub fn new() -> Self {
        Timer::default()
    }

    pub fn tick(&mut self) -> f32 {
        let current_time = Instant::now();

        let delta_time = current_time
            .duration_since(self.last_frame_time)
            .as_secs_f32();

        self.last_frame_time = current_time;

        delta_time
    }
}
