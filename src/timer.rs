use std::time::Instant;

pub struct Timer {
    last_frame_time: Instant,
    delta_time: f32,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            last_frame_time: Instant::now(),
            delta_time: 0.0,
        }
    }
}

impl Timer {
    pub fn new() -> Self {
        Timer::default()
    }

    pub fn update(&mut self) {
        let current_time = Instant::now();

        self.delta_time = current_time
            .duration_since(self.last_frame_time)
            .as_secs_f32();

        self.last_frame_time = current_time;
    }

    pub fn delta_time(&self) -> f32 {
        self.delta_time
    }
}
