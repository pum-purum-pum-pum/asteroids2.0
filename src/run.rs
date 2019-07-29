use nphysics2d::world::World;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use shrev::EventChannel;
use specs::prelude::*;
use specs::World as SpecsWorld;
use red::{self, GL, glow};
use red::glow::RenderLoop;
#[cfg(any(target_os = "android"))]
use backtrace::Backtrace;
#[cfg(any(target_os = "android"))]
use std::panic;
use crate::types::{*};
use crate::components::*;
use crate::gfx::{Canvas, GlyphVertex, TextVertexBuffer, TextData};
use crate::physics::{safe_maintain, PHYSICS_SIMULATION_TIME};
use crate::sound::{init_sound, };
use crate::systems::{
    AISystem, CollisionSystem, ControlSystem, GamePlaySystem, InsertSystem,
    KinematicSystem, PhysicsSystem, RenderingSystem, SoundSystem, MenuRenderingSystem,
    GUISystem,
};
use glyph_brush::{*};
use crate::gfx::{ParticlesData, MovementParticles};
use crate::gui::{IngameUI, Primitive};
use std::collections::{HashMap};

const NEBULAS_NUM: usize = 3usize;
pub const FINGER_NUMBER: usize = 20;

pub fn run() -> Result<(), String> {
    let dejavu: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");
    // let path_str = format!("assets/{}.png", );
    // let dejavu = RWops::from_file(Path::new(&"assets/fonts/DejaVuSans.ttf"), "r").unwrap();
    // let dejavu: Vec<u8> = dejavu.bytes().map(|x| x.unwrap() ).collect();
    // let dejavu = dejavu.as_slice();
    let glyph_brush: GlyphBrush<GlyphVertex, _> = GlyphBrushBuilder::using_font_bytes(dejavu).build();
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
    let viewport = red::Viewport::for_window(window_w as i32, window_h as i32);
    let mut phys_world: World<f32> = World::new();
    phys_world.set_timestep(PHYSICS_SIMULATION_TIME);
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();
    let (_ddpi, hdpi, _vdpi) = video.display_dpi(0i32)?;
        let gl_attr = video.gl_attr();
    let glsl_version = "#version 130";
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

    let canvas = Canvas::new(&context, &glsl_version).unwrap();
    let mut keys_channel: EventChannel<Keycode> = EventChannel::with_capacity(100);
    let mut sounds_channel: EventChannel<Sound> = EventChannel::with_capacity(20);
    let mut insert_channel: EventChannel<InsertEvent> = EventChannel::with_capacity(100);
    let mut primitives_channel: EventChannel<Primitive> = EventChannel::with_capacity(100);
    // ------------------- SPECS SETUP
    let mut specs_world = SpecsWorld::new();
    let touches: Touches = [None; FINGER_NUMBER];
    let spawned_upgrades: SpawnedUpgrades = vec![];
    specs_world.add_resource(ChoosedUpgrade(0usize));
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
    specs_world.register::<ShipStats>();
    let images = [
        "back",
        "player_new", 
        "asteroid",
        "light",
        "light_sea",
        "projectile",
        "enemy_projectile",
        "enemy1",
        "enemy2",
        "enemy3",
        "enemy4",
        "bullet_speed",
        "ship_speed",
        "attack_speed",
        "direction",
        "circle",
        "rotation_speed",
        "shield_regen",
        "health_regen",
        "health_size",
        "shield_size",
        "lazer_gun",
        "blaster_gun",
        "shotgun",

    ];
    let mut name_to_image = HashMap::new();
    for image_name in images.iter() {
        let image_data = ThreadPin::new(
            ImageData::new(&context, image_name).unwrap()
        );
        let image = specs_world
            .create_entity()
            .with(image_data)
            .build();        
        name_to_image.insert(image_name.to_string(), image);
    }
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

    {   // load .ron files with tweaks 
        use ron::de::from_reader;
        use serde::{Serialize, Deserialize};
        use std::fs::File;

        #[derive(Debug, Serialize, Deserialize)]
        pub struct DescriptionSave {
            player_ships_stats: Vec<ShipStats>,
            player_guns: Vec<GunKind>,
            enemies: Vec<EnemyKindSave>
        }
  
        fn process_description(
            description_save: DescriptionSave, 
            name_to_image: &HashMap<String, specs::Entity>
        ) -> Description {
            Description {
                player_ships_stats: description_save.player_ships_stats,
                player_guns: description_save.player_guns,
                enemies: description_save.enemies.iter().map(
                    |enemy| {
                        load_enemy(enemy, name_to_image)
                    })
                .collect()
            }
        }

        fn load_enemy(enemy_save: &EnemyKindSave, name_to_image: &HashMap<String, specs::Entity>) -> EnemyKind {
            EnemyKind {
                ai_kind: enemy_save.ai_kind,
                gun_kind: enemy_save.gun_kind,
                ship_stats: enemy_save.ship_stats,
                image: Image(name_to_image[&enemy_save.image_name])
            }
        }
        #[derive(Debug, Serialize, Deserialize)]
        pub struct EnemyKindSave {
            pub ai_kind: AIType,
            pub gun_kind: GunKind,
            pub ship_stats: ShipStats,
            pub image_name: String,
        };
        let file = File::open("desc.ron").unwrap();
        let desc: DescriptionSave = match from_reader(file) {
            Ok(x) => x,
            Err(e) => {
                println!("Failed to load config: {}", e);

                std::process::exit(1);
            }
        };
        let desc = process_description(desc, &name_to_image);
        specs_world.add_resource(desc);
        #[derive(Debug, Serialize, Deserialize)]
        pub struct UpgradeCardSave {
            upgrade_type: UpgradeType,
            image: String,
            name: String,
            description: String            
        }
        let file = File::open("upgrades.ron").unwrap();
        let mut upgrades: Vec<UpgradeCardSave> = match from_reader(file) {
            Ok(x) => x,
            Err(e) => {
                println!("Failed to load config: {}", e);

                std::process::exit(1);
            }
        };
        let upgrades: Vec<UpgradeCard> = upgrades.drain(..).map(
            |upgrade| {
                UpgradeCard {
                    upgrade_type: upgrade.upgrade_type,
                    image: Image(name_to_image[&upgrade.image]),
                    name: upgrade.name,
                    description: upgrade.description
                }
            }
        ).collect();
        let avaliable_upgrades = upgrades;
        specs_world.add_resource(avaliable_upgrades);
    }

    let preloaded_images = PreloadedImages {
        character: name_to_image["player_new"],
        projectile: name_to_image["projectile"],
        enemy_projectile: name_to_image["enemy_projectile"],
        asteroid: name_to_image["asteroid"],
        enemy: name_to_image["enemy1"],
        enemy2: name_to_image["enemy2"],
        enemy3: name_to_image["enemy3"],
        enemy4: name_to_image["enemy4"],
        background: name_to_image["back"],
        nebulas: nebula_images,
        ship_speed_upgrade: name_to_image["ship_speed"],
        bullet_speed_upgrade: name_to_image["bullet_speed"],
        attack_speed_upgrade: name_to_image["attack_speed"],
        light_white: name_to_image["light"],
        light_sea: name_to_image["light_sea"],
        direction: name_to_image["direction"],
        circle: name_to_image["circle"],
        lazer: name_to_image["lazer_gun"],
        blaster: name_to_image["blaster_gun"],
        shotgun: name_to_image["shotgun"],
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
    let (preloaded_sounds, music_data, _audio, _mixer, timer) = init_sound(&sdl_context, &mut specs_world)?;
    specs_world.add_resource(ThreadPin::new(music_data));
    specs_world.add_resource(Music::default());
    specs_world.add_resource(LoopSound::default());
    specs_world.add_resource(MenuChosedGun::default());
    specs_world.add_resource(preloaded_sounds);
    specs_world.add_resource(preloaded_particles);
    specs_world.add_resource(ThreadPin::new(timer));
    let mut sound_dispatcher = DispatcherBuilder::new()
        .with_thread_local(sound_system)
        .build();
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
        sound_dispatcher.dispatch(&specs_world.res);
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