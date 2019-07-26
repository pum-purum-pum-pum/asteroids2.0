use sdl2::rwops::RWops;
use std::path::Path;
use std::io::Read;
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
use crate::gfx::{Canvas, GlyphVertex, TextVertexBuffer, TextData};
use crate::physics::{safe_maintain, CollisionId, PHYSICS_SIMULATION_TIME};
use crate::sound::init_sound;
use crate::systems::{
    AISystem, CollisionSystem, ControlSystem, GamePlaySystem, InsertSystem,
    KinematicSystem, PhysicsSystem, RenderingSystem, SoundSystem, MenuRenderingSystem,
    GUISystem,
};
use glyph_brush::{rusttype::*, *};
use crate::gfx::{ParticlesData, MovementParticles};
use crate::gui::{IngameUI, Primitive};
const NEBULAS_NUM: usize = 3usize;
pub const FINGER_NUMBER: usize = 20;

pub fn run() -> Result<(), String> {
    let dejavu: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");
    // let path_str = format!("assets/{}.png", );
    // let dejavu = RWops::from_file(Path::new(&"assets/fonts/DejaVuSans.ttf"), "r").unwrap();
    // let dejavu: Vec<u8> = dejavu.bytes().map(|x| x.unwrap() ).collect();
    // let dejavu = dejavu.as_slice();
    let mut glyph_brush: GlyphBrush<GlyphVertex, _> = GlyphBrushBuilder::using_font_bytes(dejavu).build();
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
        gl_attr.set_context_version(3, 2);
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
    let text_buffer = TextVertexBuffer::empty_new(&context).unwrap();
    let glyph_texture = red::shader::Texture::new(&context, glyph_brush.texture_dimensions());
    let text_data = ThreadPin::new(TextData{
        vertex_buffer: text_buffer,
        vertex_num: 0,
        glyph_texture: glyph_texture.clone(),
        glyph_brush
    });

    let mut text_vb = ThreadPin::new(TextVertexBuffer::empty_new(&context).unwrap());
    let canvas = Canvas::new(&context, &glsl_version).unwrap();
    let mut keys_channel: EventChannel<Keycode> = EventChannel::with_capacity(100);
    let mut sounds_channel: EventChannel<Sound> = EventChannel::with_capacity(20);
    let mut insert_channel: EventChannel<InsertEvent> = EventChannel::with_capacity(100);
    let mut primitives_channel: EventChannel<Primitive> = EventChannel::with_capacity(100);
    // ------------------- SPECS SETUP
    let mut specs_world = SpecsWorld::new();
    let touches: Touches = [None; FINGER_NUMBER];
    let spawned_upgrades: SpawnedUpgrades = vec![];
    specs_world.add_resource(text_data);
    // specs_world.add_resource(glyph_brush);
    specs_world.add_resource(glyph_texture);
    specs_world.add_resource(spawned_upgrades);
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
    specs_world.register::<Blaster>();
    specs_world.register::<ShotGun>();
    specs_world.register::<Lazer>();
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
    let rotation_speed_image_data = ThreadPin::new(
        ImageData::new(&context, "rotation_speed").unwrap()
    );
    let shield_regen_image_data = ThreadPin::new(
        ImageData::new(&context, "shield_regen").unwrap()
    );
    let health_regen_image_data = ThreadPin::new(
        ImageData::new(&context, "health_regen").unwrap()
    );
    let health_size_image_data = ThreadPin::new(
        ImageData::new(&context, "health_size").unwrap()
    );
    let shield_size_image_data = ThreadPin::new(
        ImageData::new(&context, "shield_size").unwrap()
    );
    let lazer_gun_image_data = ThreadPin::new(
        ImageData::new(&context, "lazer_gun").unwrap()
    );
    let blaster_image_data = ThreadPin::new(
        ImageData::new(&context, "blaster_gun").unwrap()
    );
    let shotgun_image_data = ThreadPin::new(
        ImageData::new(&context, "shotgun").unwrap()
    );
    let enemy3_image_data = ThreadPin::new(
        ImageData::new(&context, "enemy3").unwrap()
    );
    let enemy4_image_data = ThreadPin::new(
        ImageData::new(&context, "enemy4").unwrap()
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
    let rotation_speed_image = specs_world
        .create_entity()
        .with(rotation_speed_image_data)
        .build();
    let shield_regen_image = specs_world
        .create_entity()
        .with(shield_regen_image_data)
        .build();
    let health_regen_image = specs_world
        .create_entity()
        .with(health_regen_image_data)
        .build();
    let shield_size_image = specs_world
        .create_entity()
        .with(shield_size_image_data)
        .build();
    let health_size_image = specs_world
        .create_entity()
        .with(health_size_image_data)
        .build();
    let lazer_gun_image = specs_world
        .create_entity()
        .with(lazer_gun_image_data)
        .build();
    let blaster_image = specs_world
        .create_entity()
        .with(blaster_image_data)
        .build();
    let shotgun_image = specs_world
        .create_entity()
        .with(shotgun_image_data)
        .build();
    let enemy3_image = specs_world
        .create_entity()
        .with(enemy3_image_data)
        .build();
    let enemy4_image = specs_world
        .create_entity()
        .with(enemy4_image_data)
        .build();
    let preloaded_images = PreloadedImages {
        character: character_image,
        projectile: projectile_image,
        enemy_projectile: enemy_projectile_image,
        asteroid: asteroid_image,
        enemy: enemy_image,
        enemy2: enemy2_image,
        enemy3: enemy3_image,
        enemy4: enemy4_image,
        background: background_image,
        nebulas: nebula_images,
        ship_speed_upgrade: ship_speed_image,
        bullet_speed_upgrade: bullet_speed_image,
        attack_speed_upgrade: attack_speed_image,
        light_white: light_image,
        light_sea: light_sea_image,
        direction: direction_image,
        circle: circle_image,
        lazer: lazer_gun_image,
        blaster: blaster_image,
        shotgun: shotgun_image,
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

    let insert_system = InsertSystem::new(insert_channel.register_reader());
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
    specs_world.add_resource(MenuChosedGun::default());
    specs_world.add_resource(preloaded_sounds);
    specs_world.add_resource(preloaded_particles);
    specs_world.add_resource(ThreadPin::new(timer));
    let mut avaliable_upgrades = vec![
        UpgradeCard {
            upgrade_type: UpgradeType::AttackSpeed,
            image: Image(preloaded_images.attack_speed_upgrade),
            name: "Attack speed".to_string(),
            description: "+ X% attack speed".to_string()
        },
        UpgradeCard {
            upgrade_type: UpgradeType::BulletSpeed,
            image: Image(preloaded_images.bullet_speed_upgrade),
            name: "Bullet speed".to_string(),
            description: "+ X% bullet speed. Also by law of physics bullets go futher".to_string()
        },
        UpgradeCard {
            upgrade_type: UpgradeType::ShipSpeed,
            image: Image(preloaded_images.ship_speed_upgrade),
            name: "Ship speed".to_string(),
            description: "+ X% attack speed".to_string()
        },
        UpgradeCard {
            upgrade_type: UpgradeType::ShipRotationSpeed,
            image: Image(rotation_speed_image),
            name: "Ship rotation speed".to_string(),
            description: "Improves rotation speed by X%".to_string() 
        },
        UpgradeCard {
            upgrade_type: UpgradeType::ShieldRegen,
            image: Image(shield_regen_image),
            name: "Shield reneration".to_string(),
            description: "+ 60 hp per sec".to_string()
        },
        UpgradeCard {
            upgrade_type: UpgradeType::HealthRegen,
            image: Image(health_regen_image),
            name: "Health regeneration".to_string(),
            description: "+ 60 hp per sec".to_string()
        },
        UpgradeCard {
            upgrade_type: UpgradeType::ShieldSize,
            image: Image(shield_size_image),
            name: "Shield size".to_string(),
            description: "More shield".to_string()
        },
        UpgradeCard {
            upgrade_type: UpgradeType::HealthSize,
            image: Image(health_size_image),
            name: "Health size".to_string(),
            description: "More health".to_string()
        }
    ];
    specs_world.add_resource(avaliable_upgrades);
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
        // .with_thread_local(insert_system)
        .with_thread_local(rendering_system)
        .with_thread_local(sound_system)
        .build();
    let mut insert_dispatcher = DispatcherBuilder::new()
        .with_thread_local(insert_system)
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
    insert_dispatcher.dispatch(&specs_world.res);
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
        insert_dispatcher.dispatch(&specs_world.res);
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