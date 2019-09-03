use common::*;
use std::collections::{VecDeque, HashMap};
use std::time::{Duration, Instant};
use red::{GL};
use gfx_h::{Canvas};


#[derive(Debug, Clone)]
pub struct Plot<T: Copy> {
	plot_data: PlotData,
	duration: Duration,
	queue: VecDeque<(Instant, T)>
}

#[derive(Debug, Clone)]
pub struct PlotData {
	pub color: Point3
}

impl PlotData {
	pub fn new(color: Point3) -> Self {
		Self {
			color: color
		}
	}
}

impl<T> Plot<T> where T: Copy {
	pub fn new(plot_data: PlotData, duration: Duration) -> Self {
		Plot { plot_data: plot_data, duration: duration, queue: VecDeque::new() }
	}
	pub fn update(&mut self) {
		loop {
			if let Some(value) = self.queue.front() {
				if Instant::now() - value.0 > self.duration {
					self.queue.pop_front();
				} else {
					break
				}
			} else {
				break
			}

		}
	}

	pub fn insert(&mut self, value: T) {
		self.queue.push_back((Instant::now(), value));
	}

	pub fn iter(&self) -> impl Iterator<Item = (f32, T)> + '_ {
		self.queue.iter().map(move |x| 
			(
				((Instant::now() - x.0).as_millis() as f32)
						/ (self.duration.as_millis() as f32), 
				x.1
			)
		)
	}
}

pub struct TeleGraph {
	duration: Duration,
	plot: HashMap<String, Plot<f32>>,
}

impl TeleGraph {
	pub fn new(duration: Duration) -> Self {
		Self {
			duration: duration,
			plot: HashMap::new(),
		}
	}

	pub fn set_color(&mut self, name: String, color: Point3) {
		let plot_data = PlotData::new(color);
		let this_plot = self.plot.entry(name).or_insert(Plot::new(plot_data, self.duration));
		this_plot.plot_data.color = color;
	}

	pub fn update(&mut self) {
		for p in self.plot.iter_mut() {
			p.1.update()
		}
	}

	pub fn insert(&mut self, name: String, value: f32) {
		let plot_data = PlotData::new(Point3::new(1f32, 1f32, 1f32));
		let this_plot = self.plot.entry(name).or_insert(Plot::new(plot_data, self.duration));
		this_plot.insert(value);
	}

	pub fn iter(&self, name: String) -> Option<(PlotData, impl Iterator<Item = (f32, f32)> + '_)> {
		if let Some(plot) = self.plot.get(&name) {
			Some((plot.plot_data.clone(), plot.iter()))
		} else {
			None
		}
	}

	pub fn iter_names(&self) -> impl Iterator<Item = &String> + '_ {
		self.plot.keys()
	}
}


pub fn render_plot<T>(
	plot_data: PlotData,
	iter_plot: T, 
	w: f32, 
	h: f32,
	context: &GL,
	viewport: &red::Viewport,
	canvas: &Canvas,
	frame: &mut red::Frame,
) where T: Iterator<Item = (f32, f32)> {
    let render_line = move |
        a: Point2,
        b: Point2,
        frame: &mut red::Frame,
	| {
	        canvas.draw_line(
	        	a,
	        	b,
	        	&context,
	        	frame,
	        	&viewport,
	        	plot_data.color,
	        	0.1f32
        );	
	};
	let mut prev = None;
	for (x_fract, value) in iter_plot {
		let x =  w - w * x_fract - w / 2.0;
		let y = h - h * value - h / 2.0;
		let current = Point2::new(x, y);
		if let Some(prev) = prev {
    		render_line(prev, current, frame);
		}
		prev = Some(current);
	}	
}