use astro_lib::prelude::*;

use specs::prelude::*;
use specs::World as SpecsWorld;
use shrev::EventChannel;
use sdl2::keyboard::Keycode;
use nalgebra::{Isometry3, Vector3};

use glium::Surface;
use glium;

mod components;
mod systems;
mod resources;
mod gfx_backend;
mod gfx;

use systems::{KinematicSystem, ControlSystem};
use components::*;
use resources::*;
use gfx_backend::DisplayBuild;
use gfx::{Canvas, ImageData, load_texture};

pub fn main() -> Result<(), String> {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
 
    let display = video_subsystem.window("rust-sdl2 demo", 1000, 1000)
        .resizable()
        .position_centered()
        .build_glium()
        .unwrap();
    let mut keys_channel: EventChannel<Keycode> = EventChannel::with_capacity(100);
    // ------------------- SPECS SETUP
    let mut specs_world = SpecsWorld::new();
    specs_world.register::<Position>();
    specs_world.register::<Velocity>();
    let character = specs_world.create_entity()
        .with(Position::new(0f32, 0f32))
        .with(Velocity::new(10f32, 10f32))
        .build();
    let control_system = ControlSystem::new(keys_channel.register_reader(), character); 
    let mut dispatcher = DispatcherBuilder::new()
        .with(KinematicSystem{}, "kinematic_system", &[])
        .with(control_system, "control_system", &[])
        .build();
    specs_world.add_resource(keys_channel);
    specs_world.add_resource(MouseState::default());
    // ------------------------------

    let canvas = Canvas::new(&display);
    let image_data = ImageData::new(&display, "player").unwrap();
    let mut running = true;
    let mut event_pump = sdl_context.event_pump().unwrap();

    while running {
        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        canvas.render(
            &display, 
            &mut target, 
            &image_data, 
            &Isometry3::new(
                Vector3::new(0f32, 0f32, 0f32),
                Vector3::new(0f32, 0f32, 0f32),
            )
        ).unwrap();
        target.finish().unwrap();

        for event in event_pump.poll_iter() {
            use sdl2::event::Event;

            match event {
                Event::Quit { .. } => running = false,
                _ => ()
            }
        }
    }
    Ok(())
}