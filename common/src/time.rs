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

    pub fn average_iter_per_sec(&mut self) -> f32 {
        self.remove_old();
        self.times.len() as f32 / 10.0
    }
}

/// Helper struct to calculate what parts of an operation are taking time
pub struct BreakdownCounter {
    times: VecDeque<(Instant, Vec<Duration>)>,
    total_micros: Vec<u128>,
    part_names: Vec<String>,
    last_time: Instant,
}

impl BreakdownCounter {
    pub fn new() -> Self {
        Self {
            times: Default::default(),
            total_micros: Default::default(),
            part_names: Default::default(),
            last_time: Instant::now(),
        }
    }

    /// Start a new frame
    pub fn start_frame(&mut self) {
        self.remove_old();
        self.part_names.clear();
        self.last_time = Instant::now();
    }

    fn remove_old(&mut self) {
        let t = Instant::now();
        while let Some((prev_time, durations)) = self.times.front() {
            let d = t - *prev_time;
            if d.as_secs() >= 10 {
                for (i, stored_d) in durations.iter().enumerate() {
                    self.total_micros[i] -= stored_d.as_micros();
                }
                self.times.pop_front();
            } else {
                break;
            }
        }
    }

    /// Record the previous part under the given `part_name`
    pub fn record_part(&mut self, part_name: impl ToString) {
        let i = self.part_names.len();
        self.part_names.push(part_name.to_string());
        self.remove_old();

        let now = Instant::now();
        let d = now - self.last_time;
        self.last_time = now;

        if i == 0 {
            self.times.push_back((now, Vec::with_capacity(self.times.back().map(|x| x.1.len()).unwrap_or_default())));
        }
        self.times.back_mut().unwrap().1.push(d);


        if i < self.total_micros.len() {
            self.total_micros[i] += d.as_micros();
        } else {
            self.total_micros.push(d.as_micros());
        }
    }

    /// Extract part averages
    pub fn extract_part_averages(&mut self) -> Vec<(String, f64)> {
        let total_micros = self.total_micros.iter().sum::<u128>() as f64;
        self.part_names.drain(..).zip(self.total_micros.iter()).map(|(s, m)| (s, *m as f64 / total_micros)).collect()
    }
}