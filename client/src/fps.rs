use std::{collections::VecDeque, time::Instant};

const SECONDS_DIFFERENCE: u64 = 2;

pub struct FpsCounter {
    frames: VecDeque<Instant>,
}

impl FpsCounter {
    pub fn new() -> FpsCounter {
        Self {
            frames: VecDeque::new(),
        }
    }

    pub fn add_frame(&mut self) {
        let new_frame = Instant::now();
        while let Some(frame) = self.frames.front() {
            if (new_frame - *frame).as_secs() >= SECONDS_DIFFERENCE {
                self.frames.pop_front();
            } else {
                break;
            }
        }
        self.frames.push_back(new_frame);
    }

    pub fn fps(&self) -> usize {
        self.frames.len() / SECONDS_DIFFERENCE as usize
    }
}
