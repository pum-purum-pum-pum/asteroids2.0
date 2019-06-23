use ncollide2d::shape::ShapeHandle;
use ncollide2d::world::CollisionGroups;
use nphysics2d::object::BodyStatus;
use nphysics2d::world::World;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use shrev::EventChannel;
use specs::prelude::*;
use specs::World as SpecsWorld;

mod components;
mod geometry;
mod gfx;
mod gfx_backend;
mod physics;
mod sound;
mod systems;
#[cfg(test)]
mod test;
use astro_lib::prelude::*;

use components::*;
use gfx::{Canvas, ImageData, MovementParticles, ParticlesData};
use gfx_backend::DisplayBuild;
use physics::{safe_maintain, CollisionId, PHYSICS_SIMULATION_TIME};
use sound::init_sound;
use systems::{
    AISystem, CollisionSystem, ControlSystem, GamePlaySystem, InsertEvent, InsertSystem,
    KinematicSystem, PhysicsSystem, RenderingSystem, SoundSystem,
};

pub fn main() -> Result<(), String> {
    let mut phys_world: World<f32> = World::new();
    phys_world.set_timestep(PHYSICS_SIMULATION_TIME);
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
    let mut insert_channel: EventChannel<InsertEvent> = EventChannel::with_capacity(100);
    // ------------------- SPECS SETUP
    let mut specs_world = SpecsWorld::new();
    specs_world.add_resource(phys_world);
    specs_world.add_resource(BodiesMap::new());
    let size = 10f32;
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
    specs_world.register::<PhysicsComponent>();
    specs_world.register::<Polygon>();
    specs_world.register::<ThreadPin<ParticlesData>>();
    specs_world.register::<ThreadPin<sdl2::mixer::Chunk>>();
    specs_world.register::<ThreadPin<SoundData>>();
    specs_world.register::<Image>();
    let background_image_data = ThreadPin::new(
        ImageData::new(&display, "back").unwrap()
    );
    let character_image_data = ThreadPin::new(
        ImageData::new(&display, "player").unwrap()
    );
    let asteroid_image_data = ThreadPin::new(
        ImageData::new(&display, "asteroid").unwrap()
    );
    let light_image_data = ThreadPin::new(
        ImageData::new(&display, "light").unwrap()
    );
    let projectile_image_data = ThreadPin::new(
        ImageData::new(&display, "projectile").unwrap()
    );
    let enemy_image_data = ThreadPin::new(
        ImageData::new(&display, "enemy").unwrap()
    );
    let background_image = specs_world
        .create_entity()
        .with(background_image_data)
        .build();
    let character_image = specs_world
        .create_entity()
        .with(character_image_data)
        .build();
    let asteroid_image = specs_world
        .create_entity()
        .with(asteroid_image_data)
        .build();
    let light_image = specs_world
        .create_entity()
        .with(light_image_data)
        .build();
    let projectile_image = specs_world
        .create_entity()
        .with(projectile_image_data)
        .build();
    let enemy_image = specs_world
        .create_entity()
        .with(enemy_image_data)
        .build();
    let preloaded_images = PreloadedImages {
        projectile: projectile_image,
        asteroid: asteroid_image,
        enemy: enemy_image,
        background: background_image,
    };
    let movement_particles = ThreadPin::new(ParticlesData::MovementParticles(
        MovementParticles::new_quad(&display, -size, -size, size, size, 100),
    ));
    let movement_particles_entity = specs_world.create_entity().with(movement_particles).build();
    let preloaded_particles = PreloadedParticles {
        movement: movement_particles_entity,
    };
    let char_size = 0.7f32;
    let character_shape = Geometry::Circle { radius: char_size };
    let enemy_size = 0.7f32;
    let _enemy_shape = Geometry::Circle { radius: enemy_size };
    let character = specs_world
        .create_entity()
        .with(Isometry::new(0f32, 0f32, 0f32))
        .with(Velocity::new(0f32, 0f32))
        .with(CharacterMarker::default())
        .with(ShipMarker::default())
        .with(Image(character_image))
        .with(Gun::new(12u8))
        .with(Spin::default())
        .with(character_shape)
        .with(Size(char_size))
        .build();
    let r = 1f32;
    let character_physics_shape = ncollide2d::shape::Ball::new(r);

    let mut character_collision_groups = CollisionGroups::new();
    character_collision_groups.set_membership(&[CollisionId::PlayerShip as usize]);
    character_collision_groups.set_whitelist(&[
        CollisionId::Asteroid as usize,
        CollisionId::EnemyBullet as usize,
        CollisionId::EnemyShip as usize,
    ]);
    character_collision_groups.set_blacklist(&[CollisionId::PlayerBullet as usize]);

    PhysicsComponent::safe_insert(
        &mut specs_world.write_storage(),
        character,
        ShapeHandle::new(character_physics_shape),
        Isometry2::new(Vector2::new(0f32, 0f32), 0f32),
        BodyStatus::Dynamic,
        &mut specs_world.write_resource(),
        &mut specs_world.write_resource(),
        character_collision_groups,
        0.5f32,
    );
    {
        let _light = specs_world
            .create_entity()
            .with(Isometry::new(0f32, 0f32, 0f32))
            .with(AttachPosition(character))
            .with(Velocity::new(0f32, 0f32))
            .with(Image(light_image))
            .with(Spin::default())
            .with(Size(15f32))
            .with(LightMarker)
            .build();
    }
    let rendering_system = RenderingSystem::default();
    let phyiscs_system = PhysicsSystem::default();
    let sound_system = SoundSystem::new(sounds_channel.register_reader());
    let control_system = ControlSystem::new(keys_channel.register_reader());
    let gameplay_sytem = GamePlaySystem::default();
    let collision_system = CollisionSystem::default();
    let ai_system = AISystem::default();
    let insert_system = InsertSystem::new(insert_channel.register_reader());
    let (preloaded_sounds, _audio, _mixer, timer) = init_sound(&sdl_context, &mut specs_world)?;
    specs_world.add_resource(preloaded_sounds);
    specs_world.add_resource(preloaded_particles);
    specs_world.add_resource(ThreadPin::new(timer));
    let mut dispatcher = DispatcherBuilder::new()
        .with(KinematicSystem {}, "kinematic_system", &[])
        .with(control_system, "control_system", &[])
        .with(gameplay_sytem, "gameplay_system", &[])
        .with(ai_system, "ai_system", &[])
        .with(collision_system, "collision_system", &["ai_system"])
        .with(
            phyiscs_system,
            "physics_system",
            &[
                "kinematic_system",
                "control_system",
                "gameplay_system",
                "collision_system",
            ],
        )
        .with_thread_local(insert_system)
        .with_thread_local(rendering_system)
        .with_thread_local(sound_system)
        .build();
    specs_world.add_resource(keys_channel);
    specs_world.add_resource(sounds_channel);
    specs_world.add_resource(insert_channel);
    specs_world.add_resource(ThreadPin::new(display));
    specs_world.add_resource(Mouse {
        wdpi: hdpi,
        hdpi: hdpi,
        ..Mouse::default()
    });
    specs_world.add_resource(ThreadPin::new(canvas));
    specs_world.add_resource(preloaded_images);
    specs_world.add_resource(Stat::default());
    // let poly = LightningPolygon::new_rectangle(0f32, 0f32, 1f32, 1f32);
    // specs_world.add_resource(poly);
    // ------------------------------

    let mut event_pump = sdl_context.event_pump().unwrap();
    safe_maintain(&mut specs_world);
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
        }
        dispatcher.dispatch(&specs_world.res);
        safe_maintain(&mut specs_world);
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
