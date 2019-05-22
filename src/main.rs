use std::time::Duration;
use std::path::Path;

use astro_lib as al;
use astro_lib::prelude::*;
use al::gfx::{draw_image};
use al::types::*;

use specs::prelude::*;
use specs::World as SpecsWorld;
use shrev::EventChannel;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::{TextureCreator};
use sdl2::image::{LoadTexture};

mod components;
mod systems;

use systems::{KinematicSystem, ControlSystem};
use components::*;

pub fn main() -> Result<(), String> {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
 
    let window = video_subsystem.window("rust-sdl2 demo", 800, 600)
        .opengl()
        .position_centered()
        .build()
        .unwrap();
    let _gl_context = window.gl_create_context().unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    
    canvas.set_draw_color(Color::RGB(0, 255, 255));
    canvas.clear();
    canvas.present();
    let image_path = Path::new("assets/player.png");
    
    let texture_creator : TextureCreator<_> = canvas.texture_creator();
    let texture = texture_creator.load_texture(image_path)?;
    let mut event_pump = sdl_context.event_pump()?;
    let mut i = 0;

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
    // ------------------------------
    'running: loop {
        i = (i + 1) % 255;
        canvas.set_draw_color(Color::RGB(i, 64, 255 - i));
        canvas.clear();
        {
            let positions = specs_world.write_storage::<Position>();
            for position in positions.join() {
                draw_image(&mut canvas, position.0, Vector2::new(100f32, 100f32), &texture)?;
            }
        }
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running;
                },
                _ => {}
            }
        }
        let keys_iter: Vec<Keycode> = event_pump
            .keyboard_state()
            .pressed_scancodes()
            .filter_map(Keycode::from_scancode)
            .collect();
        specs_world.write_resource::<EventChannel<Keycode>>().iter_write(keys_iter);
        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
        dispatcher.dispatch(&mut specs_world.res);
        specs_world.maintain();
    }
    Ok(())
}