use specs::prelude::*;
use specs_derive::Component;

use crate::Image;

#[derive(Debug, Clone, Copy)]
pub struct AnimationFrame {
    pub image: Image,
    pub ticks: usize,
}

#[derive(Component, Debug, Clone)]
pub struct Animation {
    frames: Vec<AnimationFrame>,
    iterations: usize,
    current_iteration: usize,
    pub current_frame: usize,
    frame_ticks: usize,
}

impl Animation {
    pub fn new(frames: Vec<AnimationFrame>, iterations: usize, current_frame: usize) -> Self {
        assert!(frames.len() > 0);
        Animation {
            frames,
            iterations,
            current_iteration: 0,
            current_frame,
            frame_ticks: 0,
        }
    }

    pub fn next_frame(&mut self) -> Option<AnimationFrame> {
        let res = self.frames[self.current_frame];
        self.frame_ticks += 1;
        if self.frame_ticks >= res.ticks {
            self.current_frame = (self.current_frame + 1) % self.frames.len();
            if self.current_frame == 0 {
                self.current_iteration += 1;
            }
            self.frame_ticks = 0;
        }
        if self.current_iteration >= self.iterations {
            return None;
        }
        Some(res)
    }
}
