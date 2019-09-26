use std::io::{Read};
use nphysics2d::world::World;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::rwops::RWops;
use shrev::EventChannel;
use specs::prelude::*;
use specs::World as SpecsWorld;
use red::{self, GL, glow};
use red::glow::RenderLoop;
#[cfg(any(target_os = "android"))]
use backtrace::Backtrace;
#[cfg(any(target_os = "android"))]
use std::panic;
use std::collections::{HashMap};
use std::time::Duration;
use ron::de::{from_str};
use serde::{Serialize, Deserialize};
// use rand::prelude::*;
use slog::o;
use log::info;

use std::path::Path;
use std::fs::File;
use telemetry::{TeleGraph, TimeSpans};
use common::*;
use components::*;
use gfx_h::{
    Canvas, GlyphVertex, TextVertexBuffer, 
    TextData, ParticlesData, MovementParticles,
    effects::MenuParticles, GeometryData
};
use physics::{safe_maintain, PHYSICS_SIMULATION_TIME};
use physics_system::{PhysicsSystem};
use sound::{init_sound, };
use crate::systems::{
    AISystem, CollisionSystem, ControlSystem, GamePlaySystem, CommonRespawn, InsertSystem,
    KinematicSystem, RenderingSystem, SoundSystem, MenuRenderingSystem,
    ScoreTableRendering, UpgradeGUI, GUISystem, DeadScreen
};
use glyph_brush::{*};
use crate::gui::{UI, Primitive};


const NEBULAS_NUM: usize = 3usize;

pub fn just_read(file: &str) -> Result<String, String> {
    let mut rw = RWops::from_file(Path::new(&file), "r")?;
    let mut desc_str = String::new();
    if let Ok(_) = rw.read_to_string(&mut desc_str) {
        Ok(desc_str)
    } else {
        Err("failed to read file".to_string())
    }
}

pub fn run() -> Result<(), String> {
    // LOGGING

        use std::fs::OpenOptions;
        use slog::Drain;
        let log_path = "game.log";
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(log_path)
            .unwrap();

        // create logger
        let decorator = slog_term::PlainSyncDecorator::new(file);
        let drain = slog_term::FullFormat::new(decorator).build().fuse();
        let logger = slog::Logger::root(drain, o!());

        // slog_stdlog uses the logger from slog_scope, so set a logger there
        let _guard = slog_scope::set_global_logger(logger);

        // register slog_stdlog as the log handler with the log crate
        // slog_stdlog::init().unwrap();
    info!("asteroids: logging crazyness");
    let dejavu: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");
    let mut telegraph = TeleGraph::new(Duration::from_secs(10));
    telegraph.set_color("rendering".to_string(), Point3::new(1.0, 0.0, 0.0));
    telegraph.set_color("dispatch".to_string(), Point3::new(0.0, 1.0, 0.0));
    telegraph.set_color("insert".to_string(), Point3::new(0.0, 0.0, 1.0));
    telegraph.set_color("asteroids".to_string(), Point3::new(1.0, 0.0, 1.0));
    telegraph.set_color("fps".to_string(), Point3::new(1.0, 1.0, 0.0));

    let time_spans = TimeSpans::new();
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
    // let (window_w, window_h) = (1024u32, 769);
    let (window_w, window_h) = (1920u32, 1080);
    let viewport = red::Viewport::for_window(window_w as i32, window_h as i32);
    let mut phys_world: World<f32> = World::new();
    phys_world.set_timestep(PHYSICS_SIMULATION_TIME);
    {   // nphysics whatever parameters tuning
        phys_world.integration_parameters_mut().erp = 0.01;
        phys_world.integration_parameters_mut().max_linear_correction = 10.0;
    }
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();
    let (_ddpi, hdpi, _vdpi) = video.display_dpi(0i32)?;
    let gl_attr = video.gl_attr();
    #[cfg(not(any(target_os = "ios", target_os = "android", target_os = "emscripten")))]        
    let glsl_version = "#version 330";
    #[cfg(any(target_os = "ios", target_os = "android", target_os = "emscripten"))]
    let glsl_version = "#version 300 es";
    #[cfg(any(target_os = "ios", target_os = "android", target_os = "emscripten"))]
    {
        gl_attr.set_context_profile(sdl2::video::GLProfile::GLES);
        gl_attr.set_context_version(3, 0);
    }

    #[cfg(not(any(target_os = "ios", target_os = "android", target_os = "emscripten")))]
    {
        gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
        gl_attr.set_context_version(3, 3);
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

    let canvas = Canvas::new(&context, "", &glsl_version).unwrap();
    let mut keys_channel: EventChannel<Keycode> = EventChannel::with_capacity(100);
    let mut sounds_channel: EventChannel<Sound> = EventChannel::with_capacity(20);
    let mut insert_channel: EventChannel<InsertEvent> = EventChannel::with_capacity(100);
    let mut primitives_channel: EventChannel<Primitive> = EventChannel::with_capacity(100);
    // ------------------- SPECS SETUP
    let mut specs_world = SpecsWorld::new();
    let touches: Touches = [None; FINGER_NUMBER];
    let spawned_upgrades: SpawnedUpgrades = vec![];
    specs_world.add_resource(DevInfo::new());
    specs_world.add_resource(Pallete::new());
    specs_world.add_resource(UIState::default());
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
    specs_world.register::<Rocket>();
    specs_world.register::<RocketGun>();
    specs_world.register::<Projectile>();
    specs_world.register::<Reflection>();
    specs_world.register::<Blast>();
    specs_world.register::<ThreadPin<ImageData>>();
    specs_world.register::<ThreadPin<GeometryData>>();
    specs_world.register::<Spin>();
    specs_world.register::<AttachPosition>();
    specs_world.register::<ShotGun>();
    specs_world.register::<Cannon>();
    specs_world.register::<MultyLazer>();
    specs_world.register::<Image>();
    specs_world.register::<Sound>();
    specs_world.register::<Geometry>();
    specs_world.register::<Lifetime>();
    specs_world.register::<Size>();
    specs_world.register::<EnemyMarker>();
    specs_world.register::<LightMarker>();
    specs_world.register::<ShipMarker>();
    specs_world.register::<Coin>();
    specs_world.register::<SideBulletCollectable>();
    specs_world.register::<SideBulletAbility>();
    specs_world.register::<DoubleCoinsCollectable>();
    specs_world.register::<DoubleCoinsAbility>();
    specs_world.register::<DoubleExpCollectable>();
    specs_world.register::<DoubleExpAbility>();
    specs_world.register::<Exp>();
    specs_world.register::<Health>();
    specs_world.register::<CollectableMarker>();
    specs_world.register::<PhysicsComponent>();
    specs_world.register::<Polygon>();
    specs_world.register::<ThreadPin<sdl2::mixer::Chunk>>();
    specs_world.register::<ThreadPin<SoundData>>();
    specs_world.register::<Image>();
    specs_world.register::<Lifes>();
    specs_world.register::<Shield>();
    specs_world.register::<NebulaMarker>();
    specs_world.register::<StarsMarker>();
    specs_world.register::<FogMarker>();
    specs_world.register::<PlanetMarker>();
    specs_world.register::<Damage>();
    specs_world.register::<AI>();
    specs_world.register::<ThreadPin<ParticlesData>>();
    specs_world.register::<ShipStats>();
    specs_world.register::<Animation>();
    specs_world.register::<Charge>();
    specs_world.register::<Chain>();
    specs_world.register::<LazerConnect>();
    specs_world.register::<SoundPlacement>();
    specs_world.register::<Rift>();

    // TODO: load all this images automagicly (and with assets pack)
    let images = [
        "back",
        "player_ship1",
        "basic",
        "basic_select",
        "heavy",
        "heavy_select",
        "super_ship",
        "asteroid",
        "light",
        "light_sea",
        "projectile",
        "reflect_bullet",
        "enemy_shotgun_projectile",
        "bomb",
        "enemy_projectile",
        "player_projectile",
        "enemy1",
        "kamikadze",
        "buckshot",
        "reflect_bullet_enemy",
        "lazer",
        "play",
        "bullet_speed",
        "ship_speed",
        "fire_rate",
        "bullet_damage",
        "bullet_reflection",
        "direction",
        "circle",
        "circle2",
        "chain_standart",
        "chain_standart_rift",
        "chain_lazer",
        "ship_rotation",
        "shield_regen",
        "health_regen",
        "health_size",
        "shield_size",
        "lazer_gun",
        "blaster_gun",
        "shotgun",
        "coin",
        "side_bullets_ability",
        "rift",
        "double_coin",
        "double_exp",
        "health",
        "exp",
        "lazer_boss",
        "rotship",
        "random_ship",
        "bomber",
        "bomberman",
        "charging",
        "bar",
        "upg_bar",
        "fog",
        "rocket",
        "fish",
        "player",
        "enemy_projectile_old",
        "maneuverability",
        "transparent_sqr",
        "locked",
    ];
    let mut name_to_animation = HashMap::new();
    { // load animations
        let animations = [
            ("explosion", 7),
            ("explosion2", 7),
            ("blast2", 7),
            ("bullet_contact", 1),
        ];
        for (animation_name, ticks) in animations.iter() {
            // let animation_full = &format!("assets/{}", animation_name);
            let mut frames = vec![];
            for i in 1..100 {
                let animation_file = format!("assets/{}/{}.png", animation_name, i);
                if let Ok(_rw) = RWops::from_file(Path::new(&animation_file), "r") { 
                    // TODO: Rewrite -- Hacky, what if it's different error?...
                    let animation_file_relative = format!("{}/{}", animation_name, i);
                    let image_data = ThreadPin::new(
                        ImageData::new(&context, &animation_file_relative).unwrap()
                    );
                    let image = specs_world
                        .create_entity()
                        .with(image_data)
                        .build();        
                    let animation_frame = AnimationFrame {
                        image: Image(image),
                        ticks: *ticks
                    };
                    frames.push(animation_frame);
                } else {break};
            }
            let animation = Animation::new(
                frames,
                1,
                0
            );
            name_to_animation.insert(animation_name.to_string(), animation);
        }
    };
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
    let mut stars_images = vec![];
    for i in 1..=5 {
        let stars_image_data = ThreadPin::new(
            ImageData::new(&context, &format!("stars{}", i)).unwrap()
        );
        let stars_image = specs_world
            .create_entity()
            .with(stars_image_data)
            .build();
        stars_images.push(stars_image);
    }
    let mut planet_images = vec![];
    for planet_name in vec!["planet1", "jupyterish", "halfmoon"].iter() {
        let planet_image_data = ThreadPin::new(
            ImageData::new(&context, &planet_name).unwrap()
        );
        let planet_image = specs_world
            .create_entity()
            .with(planet_image_data)
            .build();
        planet_images.push(planet_image);
    }

    {   // load .ron files with tweaks
        #[derive(Debug, Serialize, Deserialize)]
        pub struct DescriptionSave {
            ship_costs: Vec<usize>,
            gun_costs: Vec<usize>,
            player_ships: Vec<ShipKindSave>,
            player_guns: Vec<GunKindSave>,
            enemies: Vec<EnemyKindSave>
        }
  
        fn process_description(
            description_save: DescriptionSave, 
            name_to_image: &HashMap<String, specs::Entity>
        ) -> Description {
            Description {
                gun_costs: description_save.gun_costs,
                ship_costs: description_save.ship_costs,
                player_ships: description_save.player_ships.iter().map(|x| x.clone().load(name_to_image)).collect(),
                player_guns: description_save.player_guns
                    .iter()
                    .map(|gun| gun.convert(name_to_image))
                    .collect(),
                enemies: description_save.enemies.iter().map(
                    |enemy| {
                        load_enemy(enemy, name_to_image)
                    })
                .collect()
            }
        }

        fn load_enemy(enemy_save: &EnemyKindSave, name_to_image: &HashMap<String, specs::Entity>) -> EnemyKind {

            EnemyKind {
                ai_kind: enemy_save.ai_kind.clone(),
                gun_kind: enemy_save.gun_kind.convert(name_to_image),
                ship_stats: enemy_save.ship_stats,
                size: enemy_save.size,
                image: Image(name_to_image[&enemy_save.image_name]),
                snake: enemy_save.snake,
                rift: enemy_save.rift.clone(),
            }
        }
        #[derive(Debug, Serialize, Deserialize)]
        pub struct EnemyKindSave {
            pub ai_kind: AI,
            pub gun_kind: GunKindSave,
            pub ship_stats: ShipStats,
            pub size: f32,
            pub image_name: String,
            pub snake: Option<usize>,
            #[serde(default)] 
            pub rift: Option<Rift>,
        };
        #[cfg(not(target_os = "android"))]
        let file = just_read("rons/desc.ron").unwrap();
        let file = &file;
        #[cfg(target_os = "android")]
        let file = include_str!("../rons/desc.ron");
        let desc: DescriptionSave = match from_str(file) {
            Ok(x) => x,
            Err(e) => {
                println!("Failed to load config: {}", e);

                std::process::exit(1);
            }
        };
        let mut enemy_name_to_id = HashMap::new();
        for (id, enemy) in desc.enemies.iter().enumerate() {
            enemy_name_to_id.insert(enemy.image_name.clone(), id);
        }
        let desc = process_description(desc, &name_to_image);
        specs_world.add_resource(desc);
        let file = include_str!("../rons/upgrades.ron");
        let upgrades_all: Vec<UpgradeCardRaw> = match from_str(file) {
            Ok(x) => x,
            Err(e) => {
                println!("Failed to load config: {}", e);

                std::process::exit(1);
            }
        };
        let upgrades: Vec<UpgradeCard> = upgrades_all.iter().map(
            |upgrade| {
                UpgradeCard {
                    upgrade_type: upgrade.upgrade_type,
                    image: Image(name_to_image[&upgrade.image]),
                    name: upgrade.name.clone(),
                    description: upgrade.description.clone()
                }
            }
        ).collect();
        let avaliable_upgrades = upgrades;
        specs_world.add_resource(avaliable_upgrades);
        pub fn wave_load(wave: &WaveSave, enemy_name_to_id: &HashMap<String, usize>) -> Wave {
            let distribution: Vec<(usize, f32)> = 
                wave.distribution
                    .iter()
                    .map(|p| (enemy_name_to_id[&p.0], p.1))
                    .collect();
            let const_distribution: Vec<(usize, usize)> =
                wave.const_distribution
                    .iter()
                    .map(|p| (enemy_name_to_id[&p.0], p.1))
                    .collect();
            Wave {
                distribution: distribution,
                ships_number: wave.ships_number,
                const_distribution: const_distribution,
                iterations: wave.iterations
            }
        }
        #[cfg(target_os = "android")]
        let file = include_str!("../rons/waves.ron");
        #[cfg(not(target_os = "android"))]
        let file = &just_read("rons/waves.ron").unwrap();
        let waves: WavesSave = match from_str(file) {
            Ok(x) => x,
            Err(e) => {
                println!("Failed to load config: {}", e);
                std::process::exit(1);
            }
        };
        let waves: Waves = Waves(waves.0
            .iter()
            .map(|p| wave_load(p, &enemy_name_to_id))
            .collect());
        specs_world.add_resource(waves);
        specs_world.add_resource(upgrades_all);
        specs_world.add_resource(CurrentWave::default());
    }

    let preloaded_images = PreloadedImages {
        character: name_to_image["basic"],
        projectile: name_to_image["projectile"],
        enemy_projectile: name_to_image["enemy_projectile"],
        asteroid: name_to_image["asteroid"],
        enemy: name_to_image["enemy1"],
        enemy2: name_to_image["kamikadze"],
        enemy3: name_to_image["buckshot"],
        enemy4: name_to_image["lazer"],
        background: name_to_image["back"],
        nebulas: nebula_images,
        stars: stars_images,
        fog: name_to_image["fog"],
        planets: planet_images,
        ship_speed_upgrade: name_to_image["ship_speed"],
        bullet_speed_upgrade: name_to_image["bullet_speed"],
        attack_speed_upgrade: name_to_image["fire_rate"],
        light_white: name_to_image["light"],
        light_sea: name_to_image["light_sea"],
        direction: name_to_image["direction"],
        circle: name_to_image["circle"],
        lazer: name_to_image["lazer_gun"],
        play: name_to_image["play"],
        blaster: name_to_image["blaster_gun"],
        shotgun: name_to_image["shotgun"],
        coin: name_to_image["coin"],
        health: name_to_image["health"],
        side_bullet_ability: name_to_image["side_bullets_ability"],
        exp: name_to_image["exp"],
        bar: name_to_image["bar"],
        upg_bar: name_to_image["upg_bar"],
        transparent_sqr: name_to_image["transparent_sqr"],
        explosion: name_to_animation["explosion"].clone(),
        blast: name_to_animation["blast2"].clone(),
        bullet_contact: name_to_animation["bullet_contact"].clone(),
        double_coin: name_to_image["double_coin"],
        double_exp: name_to_image["double_exp"],
        basic_ship: name_to_image["basic"],
        heavy_ship: name_to_image["heavy"],
        super_ship: name_to_image["super_ship"],
        locked: name_to_image["locked"],
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

    let physics_system = PhysicsSystem::default();
    let insert_system = InsertSystem::new(insert_channel.register_reader());
    let rendering_system = RenderingSystem::new(primitives_channel.register_reader());
    let rendering_system2 = RenderingSystem::new(primitives_channel.register_reader());
    let menu_rendering_system = MenuRenderingSystem::new(primitives_channel.register_reader());
    let dead_screen_system = DeadScreen::default();
    let common_respawn = CommonRespawn::default();
    let mut dead_screen_dispatcher = DispatcherBuilder::new()
        .with(common_respawn.clone(), "common_respawn", &[])
        .with_thread_local(physics_system.clone())
        .with_thread_local(dead_screen_system)
        .build();
    let mut menu_dispatcher = DispatcherBuilder::new()
        .with(common_respawn.clone(), "common_respawn", &[])
        .with_thread_local(menu_rendering_system)
        .with_thread_local(rendering_system2)
        .with_thread_local(physics_system.clone())
        .build();
    let score_table_system = ScoreTableRendering::new(primitives_channel.register_reader());
    let mut score_table_dispatcher = DispatcherBuilder::new()
        .with_thread_local(score_table_system)
        .build();
    let sound_system = SoundSystem::new(sounds_channel.register_reader());
    let control_system = ControlSystem::new(keys_channel.register_reader());
    let gameplay_sytem = GamePlaySystem::default();
    let collision_system = CollisionSystem::default();
    let ai_system = AISystem::default();
    let gui_system = GUISystem::default();
    let (preloaded_sounds, music_data, _audio, _mixer, timer) = init_sound(&sdl_context, &mut specs_world)?;
    specs_world.add_resource(NebulaGrid::new(1, 100f32, 100f32, 50f32, 50f32));
    specs_world.add_resource(PlanetGrid::new(1, 60f32, 60f32, 30f32, 30f32));
    specs_world.add_resource(StarsGrid::new(3, 40f32, 40f32, 4f32, 4f32));
    specs_world.add_resource(FogGrid::new(2, 50f32, 50f32, 5f32, 5f32));
    
    // specs_world.add_resource(MacroGame{coins: 0, score_table: 0});
    specs_world.add_resource(name_to_image);
    specs_world.add_resource(ThreadPin::new(music_data));
    specs_world.add_resource(Music::default());
    specs_world.add_resource(LoopSound::default());
    specs_world.add_resource(preloaded_sounds);
    specs_world.add_resource(preloaded_particles);
    specs_world.add_resource(ThreadPin::new(timer));
    specs_world.add_resource(
        ThreadPin::new(
            MenuParticles::new_quad(&context, -size, -size, size, size, -20.0, 20.0, 200)
        )
    );
    specs_world.add_resource(GlobalParams::default());
    {
        let file = "rons/macro_game.ron";
        let macro_game = if let Ok(mut rw) = RWops::from_file(Path::new(&file), "r") {
            let mut macro_game_str = String::new();
            let macro_game = if let Ok(_) = rw.read_to_string(&mut macro_game_str) {
                let macro_game: MacroGame = match from_str(&macro_game_str) {
                    Ok(x) => x,
                    Err(e) => {
                        println!("Failed to load config: {}", e);

                        std::process::exit(1);
                    }
                };
                macro_game
            } else {
                MacroGame::default()
            };
            macro_game
        } else {
            MacroGame::default()
        };
        specs_world.add_resource(macro_game);
    }
    let mut sound_dispatcher = DispatcherBuilder::new()
        .with_thread_local(sound_system)
        .build();
    let mut rendering_dispatcher = DispatcherBuilder::new()
        .with_thread_local(rendering_system)
        .build();
    let mut dispatcher = DispatcherBuilder::new()
        // .with(control_system, "control_system", &[])
        .with_thread_local(control_system)
        .with(gameplay_sytem, "gameplay_system", &[])
        .with(common_respawn, "common_respawn", &[])
        .with(ai_system, "ai_system", &[])
        .with(collision_system, "collision_system", &["ai_system"])
        .with(
            physics_system,
            "physics_system",
            &[
                // "kinematic_system",
                // "control_system",
                "gameplay_system",
                "collision_system",
            ],
        )
        .with(KinematicSystem {}, "kinematic_system", &["physics_system"])
        // .with_thread_local(insert_system)
        .build();
    let mut insert_dispatcher = DispatcherBuilder::new()
        .with_thread_local(insert_system)
        .build();
    let mut gui_dispatcher = DispatcherBuilder::new()
        .with_thread_local(gui_system)
        .build();
    let upgrade_gui_system = UpgradeGUI::default();
    let mut upgrade_gui_dispatcher = DispatcherBuilder::new()
        .with_thread_local(upgrade_gui_system)
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
    specs_world.add_resource(AppState::Menu);
    specs_world.add_resource(UI::default());
    specs_world.add_resource(primitives_channel);
    specs_world.add_resource(Progress::default());
    specs_world.add_resource(telegraph);
    specs_world.add_resource(time_spans);
    // ------------------------------

    let mut events_loop = sdl_context.event_pump().unwrap();
    insert_dispatcher.dispatch(&specs_world.res);
    safe_maintain(&mut specs_world);

    render_loop.run(move |running: &mut bool| {
        flame::start("loop");
        info!("asteroids: start loop");
        specs_world.write_resource::<DevInfo>().update();
        let keys_iter: Vec<Keycode> = events_loop
            .keyboard_state()
            .pressed_scancodes()
            .filter_map(Keycode::from_scancode)
            .collect();
        specs_world
            .write_resource::<EventChannel<Keycode>>()
            .iter_write(keys_iter);
        // Create a set of pressed Keys.
        flame::start("control crazyness");
        info!("asteroids: control crazyness");
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
                specs_world.read_resource::<ThreadPin<Canvas>>().z_far,
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
                            dims.1 as u32,
                            specs_world
                                .read_resource::<ThreadPin<Canvas>>()
                                .z_far,
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
        flame::end("control crazyness");
        let app_state = *specs_world.read_resource::<AppState>();
        match app_state {
            AppState::Menu => {
                menu_dispatcher.dispatch(&specs_world.res)
            }
            AppState::Play(play_state) => {
                if let PlayState::Action = play_state {
                    flame::start("dispatch");
                    info!("asteroids: main dispatcher");
                    dispatcher.dispatch_seq(&specs_world.res);
                    dispatcher.dispatch_thread_local(&specs_world.res);
                    info!("asteroids: gui dispatcher");
                    gui_dispatcher.dispatch(&specs_world.res);
                    flame::end("dispatch");
                } else {
                    info!("asteroids: upgrade dispatcher");
                    upgrade_gui_dispatcher.dispatch(&specs_world.res);
                }
                // specs_world.write_resource::<TimeSpans>().begin("rendering".to_string());
                info!("asteroids: rendering dispatcher");
                rendering_dispatcher.dispatch(&specs_world.res);
                // specs_world.write_resource::<TimeSpans>().end("rendering".to_string())
            }
            AppState::ScoreTable => {
                score_table_dispatcher.dispatch(&specs_world.res);
            }
            AppState::DeadScreen => {
                info!("dead screen");
                dead_screen_dispatcher.dispatch(&specs_world.res);
                rendering_dispatcher.dispatch(&specs_world.res);
            }
        }
        info!("asteroids: insert dispatcher");
        flame::start("insert");
        insert_dispatcher.dispatch(&specs_world.res);
        flame::end("insert");
        info!("asteroids: sounds dispatcher");
        flame::start("sounds");
        sound_dispatcher.dispatch(&specs_world.res);
        flame::end("sounds");
        flame::start("maintain");
        info!("asteroids: maintain");
        safe_maintain(&mut specs_world);
        flame::end("maintain");
        flame::start("events loop");
        info!("asteroids: events loop");
        for event in events_loop.poll_iter() {
            use sdl2::event::Event;
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    *running = false;
                    use ron::ser::{to_string_pretty, PrettyConfig};
                    use std::io::Write;
                    // use serde::Serialize;
                    let pretty = PrettyConfig {
                        depth_limit: 2,
                        separate_tuple_members: true,
                        enumerate_arrays: true,
                        ..PrettyConfig::default()
                    };
                    let s = to_string_pretty(&*specs_world.write_resource::<MacroGame>(), pretty).expect("Serialization failed");
                    let file = "rons/macro_game.ron";
                    // let mut rw = RWops::from_file(Path::new(&file), "r+").expect("failed to load macro game");
                    eprintln!("{}", s);
                    if let Ok(mut rw) = RWops::from_file(Path::new(&file), "r+") {
                        rw.write(s.as_bytes()).expect("failed to load macro game");
                    } else {
                        let mut rw = RWops::from_file(Path::new(&file), "w").expect("failed to load macro game");
                        rw.write(s.as_bytes()).expect("failed to write");
                    }
                    flame::dump_html(&mut File::create("flame-graph.html").unwrap()).unwrap();
                },
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
        flame::end("events loop");
        // ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
        flame::end("loop");
        if flame::spans().len() > 10 {
            flame::clear();
        }
    });
        
    Ok(())
}