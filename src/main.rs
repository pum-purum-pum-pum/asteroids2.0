use specs::prelude::*;
use specs::World as SpecsWorld;
use shrev::EventChannel;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;

mod components;
mod systems;
mod resources;
mod gfx_backend;
mod gfx;
mod geometry;
#[cfg(test)]
mod test;
use astro_lib::prelude::*;

use systems::{KinematicSystem, ControlSystem, RenderingSystem};
use components::*;
use gfx_backend::DisplayBuild;
use gfx::{Canvas, ImageData};

pub fn main() -> Result<(), String> {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let (_ddpi, hdpi, _vdpi) = video_subsystem.display_dpi(0i32)?;
 
    let display = video_subsystem.window("rust-sdl2 demo", 1700, 1000)
        .resizable()
        .position_centered()
        .build_glium()
        .unwrap();
    // dbg!("hello");
    let canvas = Canvas::new(&display);
    let mut keys_channel: EventChannel<Keycode> = EventChannel::with_capacity(100);
    // ------------------- SPECS SETUP
    let mut specs_world = SpecsWorld::new();
    specs_world.register::<Isometry>();
    specs_world.register::<Velocity>();
    specs_world.register::<CharacterMarker>();
    specs_world.register::<AsteroidMarker>();
    specs_world.register::<ThreadPin<ImageData>>();
    specs_world.register::<Spin>();
    specs_world.register::<AttachPosition>();
    let character_image = ImageData::new(&display, "player", 0.5f32).unwrap();
    let character = specs_world.create_entity()
        .with(Isometry::new(0f32, 0f32, 0f32))
        .with(Velocity::new(0f32, 0f32))
        .with(CharacterMarker::default())
        .with(ThreadPin::new(character_image))
        .with(Spin::default())
        .build();
    let asteroid_image = ImageData::new(&display, "asteroid", 0.5f32).unwrap();
    let _asteroid = specs_world.create_entity()
        .with(Isometry::new(1f32, 1f32, 0f32))
        .with(Velocity::new(0f32, 0f32))
        .with(AsteroidMarker::default())
        .with(ThreadPin::new(asteroid_image))
        .with(Spin::default())
        .build();
    let asteroid_image = ImageData::new(&display, "asteroid", 0.5f32).unwrap();
    let _asteroid = specs_world.create_entity()
        .with(Isometry::new(-5f32, -5f32, 0f32))
        .with(Velocity::new(0f32, 0f32))
        .with(AsteroidMarker::default())
        .with(ThreadPin::new(asteroid_image))
        .with(Spin::default())
        .build();
    {
        let light_image = ImageData::new(&display, "light", 13f32).unwrap();
        let _light = specs_world.create_entity()
            .with(Isometry::new(0f32, 0f32, 0f32))
            .with(AttachPosition(character))
            .with(Velocity::new(0f32, 0f32))
            .with(ThreadPin::new(light_image))
            .with(Spin::default())
            .build();
    }
    let control_system = ControlSystem::new(keys_channel.register_reader()); 
    let rendering_system = RenderingSystem::default();
    let mut dispatcher = DispatcherBuilder::new()
        .with(KinematicSystem{}, "kinematic_system", &[])
        .with(control_system, "control_system", &[])
        .with_thread_local(rendering_system)
        .build();
    specs_world.add_resource(keys_channel);
    specs_world.add_resource(ThreadPin::new(display));
    specs_world.add_resource(Mouse{wdpi: hdpi, hdpi: hdpi,..Mouse::default()});
    specs_world.add_resource(ThreadPin::new(canvas));
    // let poly = LightningPolygon::new_rectangle(0f32, 0f32, 1f32, 1f32);
    // specs_world.add_resource(poly);
    // ------------------------------

    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        let keys_iter: Vec<Keycode> = event_pump
            .keyboard_state()
            .pressed_scancodes()
            .filter_map(Keycode::from_scancode)
            .collect();
        specs_world.write_resource::<EventChannel<Keycode>>().iter_write(keys_iter);
        // Create a set of pressed Keys.
        {
            let state = event_pump.mouse_state();
            let buttons: Vec<_> = state.pressed_mouse_buttons().collect();
            let mut mouse_state = specs_world.write_resource::<Mouse>();
            mouse_state.set_left(buttons.contains(&MouseButton::Left));
            mouse_state.set_right(buttons.contains(&MouseButton::Right));
            let dims = specs_world.read_resource::<SDLDisplay>().get_framebuffer_dimensions();
            mouse_state.set_position(
                state.x(), 
                state.y(), 
                specs_world.read_resource::<ThreadPin<Canvas>>().observer(),
                dims.0,
                dims.1
            );
            // dbg!((dims.0, dims.1));
            // dbg!((mouse_state.x, mouse_state.y));
        }
        dispatcher.dispatch(&specs_world.res);
        for event in event_pump.poll_iter() {
            use sdl2::event::Event;

            match event {
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'running,
                _ => ()
            }
            // ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
        }
    }
    Ok(())
}
