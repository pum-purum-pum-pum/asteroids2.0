use astro_lib as al;
use al::prelude::*;

use specs::{Join};
use specs::prelude::*;
use shrev::EventChannel;
use sdl2::keyboard::Keycode;
use glium::Surface;
use glium;
use nalgebra::{Isometry3, Vector3};

use crate::components::{*};
use crate::gfx::{ImageData};

const DAMPING_FACTOR: f32 = 0.95f32;
const THRUST_FORCE: f32 = 0.8f32;
const VELOCITY_MAX: f32 = 20f32;

#[derive(Default)]
pub struct RenderingSystem;

type Image = ThreadPin<ImageData>;

impl<'a> System<'a> for RenderingSystem {
    type SystemData = (
        ReadStorage<'a, Position>,
        ReadStorage<'a, CharacterMarker>,
        ReadStorage<'a, Image>,

        WriteExpect<'a, SDLDisplay>,
        WriteExpect<'a, Canvas>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            positions,
            character_markers,
            image_data,
            display,
            canvas,
        ) = data;
        let mut target = display.draw();
            target.clear_color(0.0, 0.0, 0.0, 1.0);
        for (pos, _, image) in (&positions, &character_markers, &image_data).join() {
            canvas.render(
                &display, 
                &mut target, 
                image, 
                &Isometry3::new(
                    Vector3::new(pos.0.x, pos.0.y, 0f32),
                    Vector3::new(0f32, 0f32, 3f32),
                )
            ).unwrap();
        }
        target.finish().unwrap();
    }
}

pub struct KinematicSystem;

impl<'a> System<'a> for KinematicSystem {
    type SystemData = (
        WriteStorage<'a, Position>,
        WriteStorage<'a, Velocity>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut positions,
            mut velocities,
        ) = data;
        for (position, velocity) in (&mut positions, &mut velocities).join() {
            position.0 += velocity.0;
            velocity.0 *= DAMPING_FACTOR;
        }
    }
}

pub struct ControlSystem {
    reader: ReaderId<Keycode>,
    character: specs::Entity,
}

impl ControlSystem {
    pub fn new(reader: ReaderId<Keycode>, character: specs::Entity) -> Self {
        ControlSystem{ reader: reader, character: character }
    }
}

impl<'a> System<'a> for ControlSystem {
    type SystemData = (
        WriteStorage<'a, Position>,
        WriteStorage<'a, Velocity>,
        Read<'a, EventChannel<Keycode>>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (_positions, mut velocities, keys_channel) = data;
        for key in keys_channel.read(&mut self.reader) {
            let character_velocity = velocities.get_mut(self.character).unwrap();
            match key {
                Keycode::Left | Keycode::A => {
                    character_velocity.0.x = (character_velocity.0.x - THRUST_FORCE).max(-VELOCITY_MAX);
                }
                Keycode::Right | Keycode::D => {
                    character_velocity.0.x = (character_velocity.0.x + THRUST_FORCE).min(VELOCITY_MAX);
                }
                Keycode::Up | Keycode::W =>  {
                    character_velocity.0.y = (character_velocity.0.y - THRUST_FORCE).max(-VELOCITY_MAX);
                }
                Keycode::Down | Keycode::S => {
                    character_velocity.0.y = (character_velocity.0.y + THRUST_FORCE).min(VELOCITY_MAX);
                }
                _ => ()
            }
        }
    }
}