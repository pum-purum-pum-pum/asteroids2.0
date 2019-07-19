use backtrace::Backtrace;
use ncollide2d::shape::ShapeHandle;
use ncollide2d::world::CollisionGroups;
use nphysics2d::object::BodyStatus;
use nphysics2d::world::World;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use shrev::EventChannel;
use specs::prelude::*;
use specs::World as SpecsWorld;
use red::{self, GL, Frame, DrawType, glow};
use red::glow::RenderLoop;
use std::panic;
use crate::types::{*};
use crate::components::*;
use crate::gfx::{Canvas};
use crate::physics::{safe_maintain, CollisionId, PHYSICS_SIMULATION_TIME};
use crate::sound::init_sound;
use crate::systems::{
    AISystem, CollisionSystem, ControlSystem, GamePlaySystem, InsertEvent, InsertSystem,
    KinematicSystem, PhysicsSystem, RenderingSystem, SoundSystem, MenuRenderingSystem,
    GUISystem,
};
use crate::gfx::{ParticlesData, MovementParticles};
use crate::gui::{IngameUI, Primitive};
const NEBULAS_NUM: usize = 3usize;
pub const FINGER_NUMBER: usize = 20;


pub fn run() -> Result<(), String> {
    #[cfg(any(target_os = "android"))]
    panic::set_hook(Box::new(|panic_info| {
        trace!("AAA PANIC");
        trace!("{}", panic_info);
        let bt = Backtrace::new();
        trace!("{:?}", bt);
    }));
    #[cfg(any(target_os = "android"))]
    android_log::init("MyApp").unwrap();
    let (window_w, window_h) = (1024u32, 769);
    let mut viewport = red::Viewport::for_window(window_w as i32, window_h as i32);
    let mut phys_world: World<f32> = World::new();
    phys_world.set_timestep(PHYSICS_SIMULATION_TIME);
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();
    let (_ddpi, hdpi, _vdpi) = video.display_dpi(0i32)?;
        let gl_attr = video.gl_attr();
    let mut glsl_version = "#version 130";
    #[cfg(any(target_os = "ios", target_os = "android", target_os = "emscripten"))]
    {
        gl_attr.set_context_profile(sdl2::video::GLProfile::GLES);
        gl_attr.set_context_version(2, 0);
        glsl_version = "#version 100"
    }
    #[cfg(not(any(target_os = "ios", target_os = "android", target_os = "emscripten")))]
    {
        gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
        gl_attr.set_context_version(3, 0);
    }
    let window = video
        .window("Asteroids 2.0", window_w, window_h)
        // .fullscreen()
        .opengl()
        .resizable()
        .build()
        .unwrap();
    let _gl_context = window.gl_create_context().unwrap();
    let render_loop =
        glow::native::RenderLoop::<sdl2::video::Window>::from_sdl_window(window);
    let context = glow::native::Context::from_loader_function(|s| {
        video.gl_get_proc_address(s) as *const _
    });
    let context = GL::new(context);
    let canvas = Canvas::new(&context, &glsl_version).unwrap();
    let mut keys_channel: EventChannel<Keycode> = EventChannel::with_capacity(100);
    let mut sounds_channel: EventChannel<Sound> = EventChannel::with_capacity(20);
    let mut insert_channel: EventChannel<InsertEvent> = EventChannel::with_capacity(100);
    let mut primitives_channel: EventChannel<Primitive> = EventChannel::with_capacity(100);
    // ------------------- SPECS SETUP
    let mut specs_world = SpecsWorld::new();
    let touches: Touches = [None; FINGER_NUMBER];
    specs_world.add_resource(touches);
    specs_world.add_resource(viewport);
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
    specs_world.register::<ThreadPin<sdl2::mixer::Chunk>>();
    specs_world.register::<ThreadPin<SoundData>>();
    specs_world.register::<Image>();
    specs_world.register::<Lifes>();
    specs_world.register::<Shield>();
    specs_world.register::<NebulaMarker>();
    specs_world.register::<Damage>();
    specs_world.register::<AIType>();
    specs_world.register::<ThreadPin<ParticlesData>>();
    {
        // Create menu widges
        
    }
    let background_image_data = ThreadPin::new(
        ImageData::new(&context, "back").unwrap()
    );
    let character_image_data = ThreadPin::new(
        ImageData::new(&context, "player_new").unwrap()
    );
    let asteroid_image_data = ThreadPin::new(
        ImageData::new(&context, "asteroid").unwrap()
    );
    let light_image_data = ThreadPin::new(
        ImageData::new(&context, "light").unwrap()
    );
    let light_sea_image_data = ThreadPin::new(
        ImageData::new(&context, "light_sea").unwrap()
    );
    let projectile_image_data = ThreadPin::new(
        ImageData::new(&context, "projectile").unwrap()
    );
    let enemy_projectile_image_data = ThreadPin::new(
        ImageData::new(&context, "enemy_projectile").unwrap()
    );
    let enemy_image_data = ThreadPin::new(
        ImageData::new(&context, "enemy_new").unwrap()
    );
    let enemy2_image_data = ThreadPin::new(
        ImageData::new(&context, "enemy2").unwrap()
    );
    let bullet_speed_image_data = ThreadPin::new(
        ImageData::new(&context, "bullet_speed").unwrap()
    );
    let ship_speed_image_data = ThreadPin::new(
        ImageData::new(&context, "ship_speed").unwrap()
    );
    let attack_speed_image_data = ThreadPin::new(
        ImageData::new(&context, "attack_speed").unwrap()
    );
    let direction_image_data = ThreadPin::new(
        ImageData::new(&context, "direction").unwrap()
    );
    let circle_image_data = ThreadPin::new(
        ImageData::new(&context, "circle").unwrap()
    );
    let mut nebula_images = vec![];
    for i in 1..=NEBULAS_NUM {
        let nebula_image_data = ThreadPin::new(
            ImageData::new(&context, &format!("nebula{}", i)).unwrap()
        );
        let nebula_image = specs_world
            .create_entity()
            .with(nebula_image_data)
            .build();
        nebula_images.push(nebula_image);
    }
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
    let light_sea_image = specs_world
        .create_entity()
        .with(light_sea_image_data)
        .build();
    let direction_image = specs_world
        .create_entity()
        .with(direction_image_data)
        .build();
    let enemy_projectile_image = specs_world
        .create_entity()
        .with(enemy_projectile_image_data)
        .build();
    let projectile_image = specs_world
        .create_entity()
        .with(projectile_image_data)
        .build();
    let enemy_image = specs_world
        .create_entity()
        .with(enemy_image_data)
        .build();
    let ship_speed_image = specs_world
        .create_entity()
        .with(ship_speed_image_data)
        .build();
    let bullet_speed_image = specs_world
        .create_entity()
        .with(bullet_speed_image_data)
        .build();
    let attack_speed_image = specs_world
        .create_entity()
        .with(attack_speed_image_data)
        .build();
    let enemy2_image = specs_world
        .create_entity()
        .with(enemy2_image_data)
        .build();
    let circle_image = specs_world
        .create_entity()
        .with(circle_image_data)
        .build();
    let preloaded_images = PreloadedImages {
        projectile: projectile_image,
        enemy_projectile: enemy_projectile_image,
        asteroid: asteroid_image,
        enemy: enemy_image,
        enemy2: enemy2_image,
        background: background_image,
        nebulas: nebula_images,
        ship_speed_upgrade: ship_speed_image,
        bullet_speed_upgrade: bullet_speed_image,
        attack_speed_upgrade: attack_speed_image,
        light_white: light_image,
        light_sea: light_sea_image,
        direction: direction_image,
        circle: circle_image
    };


    let movement_particles = ThreadPin::new(ParticlesData::MovementParticles(
        MovementParticles::new_quad(&context, -size, -size, size, size, 100),
    ));
    // let engine_particles = ThreadPin::new(ParticlesData::Engine(
    //     Engine::new(&display, )
    // ))
    let movement_particles_entity = specs_world.create_entity().with(movement_particles).build();
    let preloaded_particles = PreloadedParticles {
        movement: movement_particles_entity,
    };

    let char_size = 0.4f32;
    let character_shape = Geometry::Circle { radius: char_size };
    let enemy_size = 0.4f32;
    let _enemy_shape = Geometry::Circle { radius: enemy_size };
    let lifes = Lifes(MAX_LIFES);
    let shields = Shield(MAX_SHIELDS);
    let character = specs_world
        .create_entity()
        .with(lifes)
        .with(shields)
        .with(Isometry::new(0f32, 0f32, 0f32))
        .with(Velocity::new(0f32, 0f32))
        .with(CharacterMarker::default())
        .with(ShipMarker::default())
        .with(Image(character_image))
        .with(Gun::new(12usize, 10usize))
        .with(Spin::default())
        .with(character_shape)
        .with(Size(char_size))
        .build();
    let character_physics_shape = ncollide2d::shape::Ball::new(char_size);

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
        Velocity2::new(Vector2::new(0f32, 0f32), 0f32),
        BodyStatus::Dynamic,
        &mut specs_world.write_resource(),
        &mut specs_world.write_resource(),
        character_collision_groups,
        0.5f32,
    );
    let insert_system = InsertSystem::new(insert_channel.register_reader());
    insert_channel.single_write(InsertEvent::Engine {
        position: Point2::new(0f32, 0f32),
        num: 4usize,
        attached: AttachPosition(character)
    });
    // insert_channel.single_write(InsertEvent::)
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
    let rendering_system = RenderingSystem::new(primitives_channel.register_reader());
    let menu_rendering_system = MenuRenderingSystem::new(primitives_channel.register_reader());
    let mut menu_dispatcher = DispatcherBuilder::new()
        .with_thread_local(menu_rendering_system)
        .build();
    let phyiscs_system = PhysicsSystem::default();
    let sound_system = SoundSystem::new(sounds_channel.register_reader());
    let control_system = ControlSystem::new(keys_channel.register_reader());
    let gameplay_sytem = GamePlaySystem::default();
    let collision_system = CollisionSystem::default();
    let ai_system = AISystem::default();
    let gui_system = GUISystem::default();
    let (preloaded_sounds, _audio, _mixer, timer) = init_sound(&sdl_context, &mut specs_world)?;
    specs_world.add_resource(preloaded_sounds);
    specs_world.add_resource(preloaded_particles);
    specs_world.add_resource(ThreadPin::new(timer));
    let mut dispatcher = DispatcherBuilder::new()
        .with(control_system, "control_system", &[])
        .with(gameplay_sytem, "gameplay_system", &[])
        .with(ai_system, "ai_system", &[])
        .with(collision_system, "collision_system", &["ai_system"])
        .with(
            phyiscs_system,
            "physics_system",
            &[
                // "kinematic_system",
                "control_system",
                "gameplay_system",
                "collision_system",
            ],
        )
        .with(KinematicSystem {}, "kinematic_system", &["physics_system"])
        .with_thread_local(gui_system)
        .with_thread_local(insert_system)
        .with_thread_local(rendering_system)
        .with_thread_local(sound_system)
        .build();
    specs_world.add_resource(keys_channel);
    specs_world.add_resource(sounds_channel);
    specs_world.add_resource(insert_channel);
    specs_world.add_resource(ThreadPin::new(context));
    specs_world.add_resource(Mouse {
        wdpi: hdpi,
        hdpi: hdpi,
        ..Mouse::default()
    });
    specs_world.add_resource(ThreadPin::new(canvas));
    specs_world.add_resource(preloaded_images);
    specs_world.add_resource(Stat::default());
    specs_world.add_resource(AppState::Menu);
    specs_world.add_resource(IngameUI::default());
    specs_world.add_resource(primitives_channel);
    specs_world.add_resource(Progress::default());
    specs_world.add_resource(PlayerStats::default());
    // ------------------------------

    let mut events_loop = sdl_context.event_pump().unwrap();
    safe_maintain(&mut specs_world);

    render_loop.run(move |running: &mut bool| {
        let keys_iter: Vec<Keycode> = events_loop
            .keyboard_state()
            .pressed_scancodes()
            .filter_map(Keycode::from_scancode)
            .collect();
        specs_world
            .write_resource::<EventChannel<Keycode>>()
            .iter_write(keys_iter);
        // Create a set of pressed Keys.
        {
            let state = events_loop.mouse_state();
            let buttons: Vec<_> = state.pressed_mouse_buttons().collect();
            let mut mouse_state = specs_world.write_resource::<Mouse>();
            mouse_state.set_left(buttons.contains(&MouseButton::Left));
            mouse_state.set_right(buttons.contains(&MouseButton::Right));
            let dims = specs_world
                .read_resource::<red::Viewport>()
                .dimensions();
            mouse_state.set_position(
                state.x(),
                state.y(),
                specs_world.read_resource::<ThreadPin<Canvas>>().observer(),
                dims.0 as u32,
                dims.1 as u32,
            );
            // fingers
            {
                #[cfg(not(target_os = "android"))]
                {
                    let mut touches = specs_world.write_resource::<Touches>();
                    
                    touches[0] = if mouse_state.left {
                        Some(Finger::new(
                            0,
                            state.x() as f32,
                            state.y() as f32,
                            specs_world
                                .read_resource::<ThreadPin<Canvas>>()
                                .observer(),
                            0f32, 
                            dims.0 as u32,
                            dims.1 as u32
                        ))
                    } else {None};
                }
                #[cfg(target_os = "android")]
                {
                    let mut touches = specs_world.write_resource::<Touches>();
                    // TODO add multy touch here
                    if sdl2::touch::num_touch_devices() > 0 {
                        let device = sdl2::touch::touch_device(0);
                        for i in 0..sdl2::touch::num_touch_fingers(device) {
                            match sdl2::touch::touch_finger(device, i) {
                                Some(finger) => {
                                    touches[i as usize] = Some(Finger::new(
                                        finger.id as usize,
                                        finger.x * dims.0 as f32,
                                        finger.y * dims.1 as f32,
                                        specs_world
                                            .read_resource::<ThreadPin<Canvas>>()
                                            .observer(),
                                        finger.pressure,
                                        dims.0 as u32,
                                        dims.1 as u32
                                    ));
                                }
                                None => ()
                            }
                        }
                    }
                }
            }
        }
        let app_state = *specs_world.read_resource::<AppState>();
        match app_state {
            AppState::Menu => {
                menu_dispatcher.dispatch(&specs_world.res)
            }
            AppState::Play(_) => {
                dispatcher.dispatch(&specs_world.res);
            }
        }
        safe_maintain(&mut specs_world);
        for event in events_loop.poll_iter() {
            use sdl2::event::Event;
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => *running = false,
                sdl2::event::Event::Window {
                    win_event: sdl2::event::WindowEvent::Resized(w, h),
                    ..
                } => {
                    let mut viewport = specs_world.write_resource::<red::Viewport>();
                    viewport.update_size(w, h);
                    let context = specs_world.read_resource::<ThreadPin<red::GL>>();
                    viewport.set_used(&*context);
                },
                _ => (),
            }

        }
        // ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    });
        
        
    // 'running: loop {
    //     for event in event_pump.poll_iter() {
    //     }
    // }
    Ok(())
}