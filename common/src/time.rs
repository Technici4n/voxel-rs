use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Helper struct to calculate the average time of an operation
pub struct AverageTimeCounter {
    times: VecDeque<(Instant, Duration)>,
    total_micros: u128,
}

impl AverageTimeCounter {
    pub fn new() -> Self {
        Self {
            times: Default::default(),
            total_micros: 0,
        }
    }

    fn remove_old(&mut self) {
        let t = Instant::now();
        while let Some(&(prev_time, duration)) = self.times.front() {
            let d = t - prev_time;
            if d.as_secs() >= 10 {
                self.times.pop_front();
                self.total_micros -= duration.as_micros();
            } else {
                break;
            }
        }
    }

    pub fn add_time(&mut self, duration: Duration) {
        self.total_micros += duration.as_micros();
        self.times.push_back((Instant::now(), duration));
        self.remove_old();
    }

    pub fn average_time_micros(&mut self) -> u64 {
        self.remove_old();
        if self.times.len() == 0 {
            0
        } else {
            (self.total_micros / self.times.len() as u128) as u64
        }
    }
}
