use common::*;
use components::*;
use gfx_h::{
    load_atlas_image, Canvas, GeometryData, GlyphVertex, ParticlesData,
    TextData, TextVertexBuffer, WorldTextData,
};
use glyph_brush::*;
use packer::SerializedSpriteSheet;
#[cfg(any(target_os = "android"))]
use std::panic;
#[cfg(any(target_os = "android"))]
use backtrace::Backtrace;
#[cfg(any(target_os = "android"))]
use log::trace;
use nphysics2d::world::World;
use physics::PHYSICS_SIMULATION_TIME;
use red::{self, glow, GL};
use ron::de::from_str;
use sdl2::rwops::RWops;
use serde::{Deserialize, Serialize};
use slog::o;
use specs::World as SpecsWorld;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::time::Duration;
use telemetry::TeleGraph;

const NEBULAS_NUM: usize = 2usize;

pub fn preloaded_images(
    name_to_atlas: &HashMap<String, AtlasImage>,
    name_to_animation: &HashMap<String, Animation>,
) -> PreloadedImages {
    let mut nebula_images = vec![];
    for i in 1..=NEBULAS_NUM {
        let nebula_image = name_to_atlas[&format!("nebula{}", i)];
        nebula_images.push(nebula_image);
    }
    let mut stars_images = vec![];
    for i in 1..=4 {
        let stars_image = name_to_atlas[&format!("stars{}", i)];
        stars_images.push(stars_image);
    }
    let mut planet_images = vec![];
    for planet_name in vec!["planet", "jupyterish", "halfmoon"].iter() {
        let planet_image = name_to_atlas[&planet_name.to_string()];
        planet_images.push(planet_image);
    }
    PreloadedImages {
        nebulas: nebula_images,
        stars: stars_images,
        fog: name_to_atlas["fog"],
        planets: planet_images,
        ship_speed_upgrade: name_to_atlas["speed_upgrade"],
        bullet_speed_upgrade: name_to_atlas["bullet_speed"],
        attack_speed_upgrade: name_to_atlas["fire_rate"],
        light_white: name_to_atlas["light"],
        direction: name_to_atlas["direction"],
        circle: name_to_atlas["circle"],
        lazer: name_to_atlas["lazer_gun"],
        blaster: name_to_atlas["blaster_gun"],
        coin: name_to_atlas["coin"],
        health: name_to_atlas["life"],
        side_bullet_ability: name_to_atlas["side_bullets_ability"],
        exp: name_to_atlas["exp"],
        bar: name_to_atlas["bar"],
        upg_bar: name_to_atlas["upg_bar"],
        transparent_sqr: name_to_atlas["transparent_sqr"],
        explosion: name_to_animation["explosion_anim"].clone(),
        blast: name_to_animation["blast2_anim"].clone(),
        bullet_contact: name_to_animation["bullet_contact_anim"].clone(),
        double_coin: name_to_atlas["double_coin_ability"],
        double_exp: name_to_atlas["double_exp_ability"],
        basic_ship: name_to_atlas["basic"],
        heavy_ship: name_to_atlas["heavy"],
        super_ship: name_to_atlas["basic"],
        locked: name_to_atlas["locked"],
        cursor: name_to_atlas["cursor"],
    }
}

pub fn load_animations(
    atlas: &SerializedSpriteSheet,
) -> HashMap<String, Animation> {
    let mut name_to_animation = HashMap::new();
    // load animations
    let animations = [
        ("explosion_anim", 7),
        ("blast2_anim", 7),
        ("bullet_contact_anim", 1),
    ]; // (name, ticks)
    for (animation_name, ticks) in animations.iter() {
        let mut frames = vec![];
        for i in 1..100 {
            let animation_name = format!("{}_{}", animation_name, i);
            if let Some(image) = load_atlas_image(&animation_name, &atlas, 1.0)
            {
                let animation_frame = AnimationFrame {
                    image,
                    ticks: *ticks,
                };
                frames.push(animation_frame);
            } else {
                break;
            };
        }
        let animation = Animation::new(frames, 1, 0);
        name_to_animation.insert(animation_name.to_string(), animation);
    }
    name_to_animation
}

pub fn just_read(file: &str) -> Result<String, String> {
    let mut rw = RWops::from_file(Path::new(&file), "r")?;
    let mut desc_str = String::new();
    if let Ok(_) = rw.read_to_string(&mut desc_str) {
        Ok(desc_str)
    } else {
        Err("failed to read file".to_string())
    }
}

pub fn setup_logging() -> slog_scope::GlobalLoggerGuard {
    use slog::Drain;
    use std::fs::OpenOptions;
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
    let guard = slog_scope::set_global_logger(logger);
    // register slog_stdlog as the log handler with the log crate
    slog_stdlog::init().unwrap();
    guard
}

pub fn setup_text(context: &red::GL, specs_world: &mut SpecsWorld) {
    let dejavu: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");
    {
        let text_buffer = TextVertexBuffer::empty_new(&context).unwrap();
        let glyph_brush: GlyphBrush<GlyphVertex, _> =
            GlyphBrushBuilder::using_font_bytes(dejavu).build();
        let glyph_texture = red::shader::Texture::new(
            context,
            glyph_brush.texture_dimensions(),
        );
        let text_data = ThreadPin::new(TextData {
            vertex_buffer: text_buffer,
            vertex_num: 0,
            glyph_texture: glyph_texture.clone(),
            glyph_brush,
        });
        specs_world.add_resource(text_data);
        specs_world.add_resource(glyph_texture);
    }
    {
        // copy paste to store world text data separetly
        // (needed because we need different transformation uniform in shader)
        let text_buffer = TextVertexBuffer::empty_new(&context).unwrap();
        let glyph_brush: GlyphBrush<GlyphVertex, _> =
            GlyphBrushBuilder::using_font_bytes(dejavu).build();
        let glyph_texture = red::shader::Texture::new(
            context,
            glyph_brush.texture_dimensions(),
        );
        let text_data = ThreadPin::new(WorldTextData {
            vertex_buffer: text_buffer,
            vertex_num: 0,
            glyph_texture: glyph_texture.clone(),
            glyph_brush,
        });
        specs_world.add_resource(text_data);
        specs_world.add_resource(glyph_texture);
    }
}

pub fn setup_telegraph() -> TeleGraph {
    let mut telegraph = TeleGraph::new(Duration::from_secs(10));
    telegraph.set_color("rendering".to_string(), Point3::new(1.0, 0.0, 0.0));
    telegraph.set_color("dispatch".to_string(), Point3::new(0.0, 1.0, 0.0));
    telegraph.set_color("insert".to_string(), Point3::new(0.0, 0.0, 1.0));
    telegraph.set_color("asteroids".to_string(), Point3::new(1.0, 0.0, 1.0));
    telegraph.set_color("fps".to_string(), Point3::new(1.0, 1.0, 0.0));
    telegraph.set_color(
        "asteroids rendering".to_string(),
        Point3::new(0.1, 1.0, 1.0),
    );
    telegraph.set_color(
        "foreground rendering".to_string(),
        Point3::new(0.8, 0.8, 0.1),
    );
    telegraph.set_color(
        "background rendering".to_string(),
        Point3::new(0.8, 0.1, 0.1),
    );
    telegraph
        .set_color("shadow rendering".to_string(), Point3::new(0.1, 0.1, 1.0));
    telegraph.set_color(
        "sprite batch rendering".to_string(),
        Point3::new(0.5, 0.1, 0.1),
    );
    telegraph.set_color("clear".to_string(), Point3::new(0.0, 0.6, 0.0));
    telegraph
}
#[cfg(any(target_os = "android"))]
pub fn setup_android() {
    panic::set_hook(Box::new(|panic_info| {
        trace!("AAA PANIC");
        trace!("{}", panic_info);
        let bt = Backtrace::new();
        trace!("{:?}", bt);
    }));
    android_log::init("MyApp").unwrap();
}

pub fn setup_physics(specs_world: &mut SpecsWorld) {
    let mut phys_world: World<f32> = World::new();
    phys_world.set_timestep(PHYSICS_SIMULATION_TIME);
    {
        // nphysics whatever parameters tuning
        phys_world.integration_parameters_mut().erp = 0.01;
        phys_world
            .integration_parameters_mut()
            .max_linear_correction = 10.0;
    }
    specs_world.add_resource(phys_world);
}

pub fn setup_gfx(
    specs_world: &mut SpecsWorld,
) -> Result<
    (
        red::GL,
        sdl2::Sdl,
        glow::native::RenderLoop<sdl2::video::Window>,
        sdl2::video::GLContext,
        f32,
        Canvas,
    ),
    String,
> {
    let (window_w, window_h) = (1920u32, 1080);
    let viewport = red::Viewport::for_window(window_w as i32, window_h as i32);
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();
    let (_ddpi, hdpi, _vdpi) = video.display_dpi(0i32)?;
    let gl_attr = video.gl_attr();
    #[cfg(not(any(target_os = "android")))]
    let glsl_version = "#version 330";
    #[cfg(any(target_os = "android"))]
    let glsl_version = "#version 300 es";
    #[cfg(any(target_os = "android"))]
    {
        gl_attr.set_context_profile(sdl2::video::GLProfile::GLES);
        gl_attr.set_context_version(3, 0);
    }

    #[cfg(not(any(target_os = "android")))]
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
    let gl_context = window.gl_create_context().unwrap();
    let render_loop =
        glow::native::RenderLoop::<sdl2::video::Window>::from_sdl_window(
            window,
        );
    let context = glow::native::Context::from_loader_function(|s| {
        video.gl_get_proc_address(s) as *const _
    });
    let context = GL::new(context);
    let canvas = Canvas::new(&context, "", "atlas", &glsl_version).unwrap();
    specs_world.add_resource(viewport);
    Ok((context, sdl_context, render_loop, gl_context, hdpi, canvas))
}

pub fn read_atlas(path: &str) -> SerializedSpriteSheet {
    let content = just_read(path).unwrap();
    let parsed: SerializedSpriteSheet = match from_str(&content) {
        Ok(x) => x,
        Err(e) => {
            println!("Failed to load atlas: {}", e);

            std::process::exit(1);
        }
    };
    parsed
}

pub fn setup_images(
    atlas: &SerializedSpriteSheet,
) -> HashMap<String, AtlasImage> {
    // dbg!(&atlas.sprites["chains_dark"]);
    let mut name_to_image = HashMap::new();
    for (name, _sprite) in atlas.sprites.iter() {
        let image = load_atlas_image(&name, &atlas, 1.0).unwrap();
        name_to_image.insert(name.clone(), image);
    }
    name_to_image
}

pub fn data_setup(specs_world: &mut SpecsWorld) {
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
    specs_world.register::<AtlasImage>();
    specs_world.register::<ThreadPin<GeometryData>>();
    specs_world.register::<Spin>();
    specs_world.register::<AttachPosition>();
    specs_world.register::<ShotGun>();
    specs_world.register::<Cannon>();
    specs_world.register::<MultyLazer>();
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
    specs_world.register::<DamageFlash>();
    specs_world.register::<TextComponent>();
    specs_world.register::<Position2D>();
    specs_world.register::<ReflectBulletCollectable>();
    specs_world.register::<ReflectBulletAbility>();
    

    specs_world.add_resource(UpgradesStats::default());
    specs_world.add_resource(DevInfo::new());
    specs_world.add_resource(Pallete::new());
    specs_world.add_resource(UIState::default());
    specs_world.add_resource(BodiesMap::new());
    let spawned_upgrades: SpawnedUpgrades = vec![];
    specs_world.add_resource(spawned_upgrades);
    let touches: Touches = [None; FINGER_NUMBER];
    specs_world.add_resource(touches);
}

pub fn load_description(
    specs_world: &mut SpecsWorld,
    name_to_atlas: &HashMap<String, AtlasImage>,
) {
    // load .ron files with tweaks
    #[derive(Debug, Serialize, Deserialize)]
    pub struct DescriptionSave {
        ship_costs: Vec<usize>,
        gun_costs: Vec<usize>,
        player_ships: Vec<ShipKindSave>,
        player_guns: Vec<GunKindSave>,
        enemies: Vec<EnemyKindSave>,
    }

    fn process_description(
        description_save: DescriptionSave,
        name_to_atlas: &HashMap<String, AtlasImage>,
    ) -> Description {
        Description {
            gun_costs: description_save.gun_costs,
            ship_costs: description_save.ship_costs,
            player_ships: description_save
                .player_ships
                .iter()
                .map(|x| x.clone().load(name_to_atlas))
                .collect(),
            player_guns: description_save
                .player_guns
                .iter()
                .map(|gun| gun.convert(name_to_atlas))
                .collect(),
            enemies: description_save
                .enemies
                .iter()
                .map(|enemy| load_enemy(enemy, name_to_atlas))
                .collect(),
        }
    }

    fn load_enemy(
        enemy_save: &EnemyKindSave,
        name_to_atlas: &HashMap<String, AtlasImage>,
    ) -> EnemyKind {
        dbg!(&enemy_save.image_name);
        EnemyKind {
            ai_kind: enemy_save.ai_kind.clone(),
            gun_kind: enemy_save.gun_kind.convert(name_to_atlas),
            ship_stats: enemy_save.ship_stats,
            size: enemy_save.size,
            image: name_to_atlas[&enemy_save.image_name],
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
    #[cfg(target_os = "android")]
    let file = include_str!("../rons/desc.ron");
    let file = &file;
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
    let desc = process_description(desc, &name_to_atlas);
    specs_world.add_resource(desc);
    let file = include_str!("../rons/upgrades.ron");
    let upgrades_all: Vec<UpgradeCardRaw> = match from_str(file) {
        Ok(x) => x,
        Err(e) => {
            println!("Failed to load config: {}", e);

            std::process::exit(1);
        }
    };
    let upgrades: Vec<UpgradeCard> = upgrades_all
        .iter()
        .map(|upgrade| {
            dbg!(&upgrade.image);
            UpgradeCard {
                upgrade_type: upgrade.upgrade_type,
                image: name_to_atlas[&upgrade.image],
                name: upgrade.name.clone(),
                description: upgrade.description.clone(),
            }
        })
        .collect();
    let avaliable_upgrades = upgrades;
    specs_world.add_resource(avaliable_upgrades);
    pub fn wave_load(
        wave: &WaveSave,
        enemy_name_to_id: &HashMap<String, usize>,
    ) -> Wave {
        let distribution: Vec<(usize, f32)> = wave
            .distribution
            .iter()
            .map(|p| {
                dbg!(&p.0);
                (enemy_name_to_id[&p.0], p.1)
            })
            .collect();
        let const_distribution: Vec<(usize, usize)> = wave
            .const_distribution
            .iter()
            .map(|p| (enemy_name_to_id[&p.0], p.1))
            .collect();
        Wave {
            distribution: distribution,
            ships_number: wave.ships_number,
            const_distribution: const_distribution,
            iterations: wave.iterations,
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
    let waves: Waves = Waves(
        waves
            .0
            .iter()
            .map(|p| wave_load(p, &enemy_name_to_id))
            .collect(),
    );
    specs_world.add_resource(waves);
    specs_world.add_resource(upgrades_all);
    specs_world.add_resource(CurrentWave::default());

    {
        // load macro game
        let file = "rons/macro_game.ron";
        let macro_game =
            if let Ok(mut rw) = RWops::from_file(Path::new(&file), "r") {
                let mut macro_game_str = String::new();
                let macro_game =
                    if let Ok(_) = rw.read_to_string(&mut macro_game_str) {
                        let macro_game: MacroGame =
                            match from_str(&macro_game_str) {
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
}
