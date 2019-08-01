// use crate::types::{*};
use crate::components::*;
// use rand::prelude::*;
// use noise::{NoiseFn, Perlin, Seedable};

// use std::io::{BufReader, Error as IOError, Read};


// use nalgebra::geometry::Orthographic3;

// use red;
// use red::VertexAttribPointers;
// use red::glow::Context;
// use red::glow;

// use image;

// use sdl2::rwops::RWops;
// use std::path::Path;
// use glyph_brush::{
//     BrushAction, BrushError, rusttype::{Rect, point}, GlyphBrush,
//     DefaultSectionHasher
// };
// use red::shader::Texture;
// use red::data::{*};
// use red::{DrawParams, DrawType, Stencil, StencilTest, Operation};
use specs::prelude::*;
use specs_derive::Component;

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
	frame_ticks: usize
}

impl Animation {
	pub fn new(
		frames: Vec<AnimationFrame>,
		iterations: usize,
		current_frame: usize,
	) -> Self {
		assert!(frames.len() > 0);
		Animation {
			frames: frames,
			iterations: iterations,
			current_iteration: 0,
			current_frame: current_frame,
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
			return None
		}
		Some(res)
	}
}