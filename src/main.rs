use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use shrev::EventChannel;
use specs::prelude::*;
use specs::World as SpecsWorld;

mod components;
mod geometry;
mod gfx;
mod gfx_backend;
mod resources;
mod systems;
#[cfg(test)]
mod test;
use astro_lib::prelude::*;

use components::*;
use gfx::{Canvas, ImageData};
use gfx_backend::DisplayBuild;
use systems::{ControlSystem, KinematicSystem, RenderingSystem, GamePlaySystem, CollisionSystem};

pub fn main() -> Result<(), String> {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let (_ddpi, hdpi, _vdpi) = video_subsystem.display_dpi(0i32)?;

    let display = video_subsystem
        .window("rust-sdl2 demo", 1700, 1000)
        .resizable()
        .position_centered()
        .build_glium()
        .unwrap();
    // dbg!("hello");
    let canvas = Canvas::new(&display);
    let mut keys_channel: EventChannel<Keycode> = EventChannel::with_capacity(100);
    // ------------------- SPECS SETUP
    let mut specs_world = SpecsWorld::new();
    let mut images = ThreadPin::new(Images::default());
    specs_world.register::<Isometry>();
    specs_world.register::<Velocity>();
    specs_world.register::<CharacterMarker>();
    specs_world.register::<AsteroidMarker>();
    specs_world.register::<Projectile>();
    specs_world.register::<ThreadPin<ImageData>>();
    specs_world.register::<Spin>();
    specs_world.register::<AttachPosition>();
    specs_world.register::<Gun>();
    specs_world.register::<Image>();
    specs_world.register::<Geometry>();
    specs_world.register::<Lifetime>();
    specs_world.register::<Size>();
    let background_image_data = ImageData::new(&display, "back").unwrap();
    let background_image = images.add_image("back".to_string(), background_image_data);
    let character_image_data = ImageData::new(&display, "player").unwrap();
    let character_image = images.add_image("player".to_string(), character_image_data);
    let asteroid_image_data = ImageData::new(&display, "asteroid").unwrap();
    let asteroid_image = images.add_image("asteroid".to_string(), asteroid_image_data);
    let light_image_data = ImageData::new(&display, "light").unwrap();
    let light_image = images.add_image("light".to_string(), light_image_data);
    let projectile_image_data = ImageData::new(&display, "projectile").unwrap();
    let projectile_image = images.add_image("projectile".to_string(), projectile_image_data);
    let preloaded_images = PreloadedImages{
        projectile: projectile_image,
        asteroid: asteroid_image,
        background: background_image
    };
    let char_size = 0.7f32;
    let character_shape = Geometry::Circle{
        radius: char_size,
    };
    let character = specs_world
        .create_entity()
        .with(Isometry::new(0f32, 0f32, 0f32))
        .with(Velocity::new(0f32, 0f32))
        .with(CharacterMarker::default())
        .with(character_image)
        .with(Gun::new(50u8))
        .with(Spin::default())
        .with(character_shape)
        .with(Size(char_size))
        .build();
    {
        let _light = specs_world
            .create_entity()
            .with(Isometry::new(0f32, 0f32, 0f32))
            .with(AttachPosition(character))
            .with(Velocity::new(0f32, 0f32))
            .with(light_image)
            .with(Spin::default())
            .with(Size(15f32))
            .build();
    }
    // {
    //     let _back = specs_world
    //         .create_entity()
    //         .with(Isometry::new(0f32, 0f32, 0f32))
    //         .with(AttachPosition(character))
    //         .with(Velocity::new(0f32, 0f32))
    //         .with(background_image)
    //         .with(Spin::default())
    //         .with(Size(15f32))
    //         .build();
    // }
    let control_system = ControlSystem::new(keys_channel.register_reader());
    let rendering_system = RenderingSystem::default();
    let gameplay_sytem = GamePlaySystem::default();
    let collision_system = CollisionSystem::default();
    let mut dispatcher = DispatcherBuilder::new()
        .with(KinematicSystem {}, "kinematic_system", &[])
        .with(control_system, "control_system", &[])
        .with(gameplay_sytem, "gameplay_system", &[])
        .with(collision_system, "collision_system", &[])
        .with_thread_local(rendering_system)
        .build();
    specs_world.add_resource(keys_channel);
    specs_world.add_resource(ThreadPin::new(display));
    specs_world.add_resource(Mouse {
        wdpi: hdpi,
        hdpi: hdpi,
        ..Mouse::default()
    });
    specs_world.add_resource(ThreadPin::new(canvas));
    specs_world.add_resource(images);
    specs_world.add_resource(preloaded_images);
    specs_world.add_resource(Stat::default());
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
        specs_world
            .write_resource::<EventChannel<Keycode>>()
            .iter_write(keys_iter);
        // Create a set of pressed Keys.
        {
            let state = event_pump.mouse_state();
            let buttons: Vec<_> = state.pressed_mouse_buttons().collect();
            let mut mouse_state = specs_world.write_resource::<Mouse>();
            mouse_state.set_left(buttons.contains(&MouseButton::Left));
            mouse_state.set_right(buttons.contains(&MouseButton::Right));
            let dims = specs_world
                .read_resource::<SDLDisplay>()
                .get_framebuffer_dimensions();
            mouse_state.set_position(
                state.x(),
                state.y(),
                specs_world.read_resource::<ThreadPin<Canvas>>().observer(),
                dims.0,
                dims.1,
            );
            // dbg!((dims.0, dims.1));
            // dbg!((mouse_state.x, mouse_state.y));
        }
        dispatcher.dispatch(&specs_world.res);
        specs_world.maintain();
        for event in event_pump.poll_iter() {
            use sdl2::event::Event;

            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => (),
            }
            // ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
        }
    }
    Ok(())
}
