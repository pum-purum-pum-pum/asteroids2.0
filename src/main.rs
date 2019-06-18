use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use shrev::EventChannel;
use specs::prelude::*;
use specs::World as SpecsWorld;

mod components;
mod geometry;
mod gfx;
mod sound;
mod gfx_backend;
mod systems;
#[cfg(test)]
mod test;
use astro_lib::prelude::*;


use components::*;
use sound::{init_sound};
use gfx::{Canvas, ImageData, ParticlesData};
use gfx_backend::DisplayBuild;
use systems::{ControlSystem, KinematicSystem, RenderingSystem, 
              GamePlaySystem, CollisionSystem, AISystem, SoundSystem};

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
    let mut sounds_channel: EventChannel<Sound> = EventChannel::with_capacity(20);
    // ------------------- SPECS SETUP
    let mut specs_world = SpecsWorld::new();
    let images: Collector<ImageData, Image> = Collector::new_empty();
    let mut images = ThreadPin::new(images);
    let particles: Collector<ParticlesData, Particles> = Collector::new_empty();
    let size = 10f32;
    let mut particles = ThreadPin::new(particles);
    let movement_particles = particles.add_item(
        "movement".to_string(), ParticlesData::new_quad(
            &display, -size, -size, size, size, 100)
    );
    let preloaded_particles = PreloadedParticles{ movement: movement_particles };
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
    specs_world.register::<Sound>();
    specs_world.register::<Geometry>();
    specs_world.register::<Lifetime>();
    specs_world.register::<Size>();
    specs_world.register::<EnemyMarker>();
    specs_world.register::<LightMarker>();
    specs_world.register::<ShipMarker>();
    let background_image_data = ImageData::new(&display, "back").unwrap();
    let background_image = images.add_item("back".to_string(), background_image_data);
    let character_image_data = ImageData::new(&display, "player").unwrap();
    let character_image = images.add_item("player".to_string(), character_image_data);
    let enemy_image_data = ImageData::new(&display, "enemy").unwrap();
    let enemy_image = images.add_item("enemy".to_string(), enemy_image_data);
    let asteroid_image_data = ImageData::new(&display, "asteroid").unwrap();
    let asteroid_image = images.add_item("asteroid".to_string(), asteroid_image_data);
    let light_image_data = ImageData::new(&display, "light").unwrap();
    let light_image = images.add_item("light".to_string(), light_image_data);
    let projectile_image_data = ImageData::new(&display, "projectile").unwrap();
    let projectile_image = images.add_item("projectile".to_string(), projectile_image_data);
    let preloaded_images = PreloadedImages{
        projectile: projectile_image,
        asteroid: asteroid_image,
        background: background_image
    };
    let char_size = 0.7f32;
    let character_shape = Geometry::Circle{
        radius: char_size,
    };
    let enemy_size = 0.7f32;
    let enemy_shape = Geometry::Circle{
        radius: enemy_size,
    };
    let character = specs_world
        .create_entity()
        .with(Isometry::new(0f32, 0f32, 0f32))
        .with(Velocity::new(0f32, 0f32))
        .with(CharacterMarker::default())
        .with(ShipMarker::default())
        .with(character_image)
        .with(Gun::new(12u8))
        .with(Spin::default())
        .with(character_shape)
        .with(Size(char_size))
        .build();
    let _enemy = specs_world
        .create_entity()
        .with(Isometry::new(3f32, 3f32, 0f32))
        .with(Velocity::new(0f32, 0f32))
        .with(EnemyMarker::default())
        .with(ShipMarker::default())
        .with(enemy_image)
        .with(Gun::new(20u8))
        .with(Spin::default())
        .with(enemy_shape)
        .with(Size(enemy_size))
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
            .with(LightMarker)
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
    let rendering_system = RenderingSystem::default();
    let sound_system = SoundSystem::new(sounds_channel.register_reader());
    let control_system = ControlSystem::new(keys_channel.register_reader());
    let gameplay_sytem = GamePlaySystem::default();
    let collision_system = CollisionSystem::default();
    let ai_system = AISystem::default();
    let (sounds, preloaded_sounds, _audio, _mixer, timer) = init_sound(&sdl_context)?;
    let sounds = ThreadPin::new(sounds);
    specs_world.add_resource(sounds);
    specs_world.add_resource(preloaded_sounds);
    specs_world.add_resource(particles);
    specs_world.add_resource(preloaded_particles);
    specs_world.add_resource(ThreadPin::new(timer));
    let mut dispatcher = DispatcherBuilder::new()
        .with(KinematicSystem {}, "kinematic_system", &[])
        .with(control_system, "control_system", &[])
        .with(gameplay_sytem, "gameplay_system", &[])
        .with(ai_system, "ai_system", &[])
        .with(collision_system, "collision_system", &["ai_system"])
        .with_thread_local(rendering_system)
        .with_thread_local(sound_system)
        .build();
    specs_world.add_resource(keys_channel);
    specs_world.add_resource(sounds_channel);
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
