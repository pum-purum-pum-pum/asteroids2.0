#[cfg(any(target_os = "android"))]
use backtrace::Backtrace;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::rwops::RWops;
use shrev::EventChannel;
use specs::prelude::*;
use specs::World as SpecsWorld;

#[cfg(any(target_os = "android"))]
use std::panic;
use std::sync::{Arc, Mutex};
// use rand::prelude::*;
use crate::gui::{Primitive, UI};
use crate::setup::*;
use crate::systems::{
    AISystem, CollisionSystem, CommonRespawn, ControlSystem, DeadScreen,
    DestroySync, GUISystem, GamePlaySystem, InsertSystem, KinematicSystem,
    MenuRenderingSystem, RenderingSystem, ScoreTableRendering, SoundSystem,
    UpgradeGUI,
};
use common::*;
use components::*;
use gfx_h::{effects::MenuParticles, Canvas, MovementParticles, ParticlesData};
// #[cfg(not(any(target_os = "ios", target_os = "android", target_os = "emscripten"))]
use log::info;
use physics::safe_maintain;
use physics_system::PhysicsSystem;
use red::glow::RenderLoop;
use sound::init_sound;
use std::fs::File;
use std::path::Path;
use telemetry::TimeSpans;

pub fn run() -> Result<(), String> {
    let mut specs_world = SpecsWorld::new();
    data_setup(&mut specs_world);
    let _guard = setup_logging();
    let telegraph = setup_telegraph();
    let time_spans = TimeSpans::new();
    setup_android();
    setup_physics(&mut specs_world);

    // We need to own _gl_context to avoid RAII crazyness
    let (context, sdl_context, render_loop, _gl_context, hdpi, canvas) =
        setup_gfx(&mut specs_world)?;
    // Hide the cursor
    sdl_context.mouse().show_cursor(false);
    setup_text(&context, &mut specs_world);
    let atlas = read_atlas("assets/out.ron");
    let name_to_atlas = setup_images(&atlas);
    let mut asteroids_spawn_channel: EventChannel<InsertEvent> =
        EventChannel::with_capacity(100);
    let mut keys_channel: EventChannel<Keycode> =
        EventChannel::with_capacity(100);
    let mut sounds_channel: EventChannel<Sound> =
        EventChannel::with_capacity(30);
    let mut insert_channel: EventChannel<InsertEvent> =
        EventChannel::with_capacity(100);
    let mut primitives_channel: EventChannel<Primitive> =
        EventChannel::with_capacity(100);
    let name_to_animation = load_animations(&atlas);
    load_description(&mut specs_world, &name_to_atlas);
    let preloaded_images = preloaded_images(&name_to_atlas, &name_to_animation);
    let size = 10f32;
    let movement_particles = ThreadPin::new(ParticlesData::MovementParticles(
        MovementParticles::new_quad(&context, -size, -size, size, size, 100),
    ));
    // let engine_particles = ThreadPin::new(ParticlesData::Engine(
    //     Engine::new(&display, )
    // ))
    let movement_particles_entity =
        specs_world.create_entity().with(movement_particles).build();
    let preloaded_particles = PreloadedParticles {
        movement: movement_particles_entity,
    };
    let destroy_sync =
        DestroySync::new(asteroids_spawn_channel.register_reader());
    specs_world.add_resource(Arc::new(Mutex::new(asteroids_spawn_channel)));
    let physics_system = PhysicsSystem::default();
    let insert_system = InsertSystem::new(insert_channel.register_reader());
    let rendering_system =
        RenderingSystem::new(primitives_channel.register_reader());
    let rendering_system2 =
        RenderingSystem::new(primitives_channel.register_reader());
    let menu_rendering_system = MenuRenderingSystem;
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
    let score_table_system =
        ScoreTableRendering::new(primitives_channel.register_reader());
    let mut score_table_dispatcher = DispatcherBuilder::new()
        .with_thread_local(score_table_system)
        .build();
    let sound_system = SoundSystem::new(sounds_channel.register_reader());
    let control_system = ControlSystem::new(keys_channel.register_reader());
    let gameplay_sytem = GamePlaySystem::default();
    let collision_system = CollisionSystem::default();
    let ai_system = AISystem::default();
    let gui_system = GUISystem::default();
    let (preloaded_sounds, music_data, _audio, _mixer, timer) =
        init_sound(&sdl_context, &mut specs_world)?;
    specs_world.add_resource(NebulaGrid::new(1, 100f32, 100f32, 50f32, 50f32));
    specs_world.add_resource(PlanetGrid::new(1, 60f32, 60f32, 30f32, 30f32));
    specs_world.add_resource(StarsGrid::new(3, 40f32, 40f32, 4f32, 4f32));
    specs_world.add_resource(FogGrid::new(2, 50f32, 50f32, 5f32, 5f32));

    // specs_world.add_resource(MacroGame{coins: 0, score_table: 0});
    // specs_world.add_resource(TimeTracker::new());
    specs_world.add_resource(name_to_atlas);
    specs_world.add_resource(ThreadPin::new(music_data));
    specs_world.add_resource(Music::default());
    specs_world.add_resource(LoopSound::default());
    specs_world.add_resource(preloaded_sounds);
    specs_world.add_resource(preloaded_particles);
    specs_world.add_resource(ThreadPin::new(timer));
    specs_world.add_resource(ThreadPin::new(MenuParticles::new_quad(
        &context,
        (-size, size),
        (-size, size),
        (-20.0, 20.0),
        200,
    )));
    specs_world.add_resource(GlobalParams::default());
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
        .with(destroy_sync, "destroy_sync", &[])
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
            let dims =
                specs_world.read_resource::<red::Viewport>().dimensions();
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
                    } else {
                        None
                    };
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
                                            .read_resource::<ThreadPin<Canvas>>(
                                            )
                                            .observer(),
                                        finger.pressure,
                                        dims.0 as u32,
                                        dims.1 as u32,
                                        specs_world
                                            .read_resource::<ThreadPin<Canvas>>(
                                            )
                                            .z_far
                                    ));
                                }
                                None => (),
                            }
                        }
                    }
                }
            }
        }
        flame::end("control crazyness");
        let app_state = *specs_world.read_resource::<AppState>();
        match app_state {
            AppState::Menu => menu_dispatcher.dispatch(&specs_world.res),
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
                    let s = to_string_pretty(
                        &*specs_world.write_resource::<MacroGame>(),
                        pretty,
                    )
                    .expect("Serialization failed");
                    let file = "rons/macro_game.ron";
                    // let mut rw = RWops::from_file(Path::new(&file), "r+").expect("failed to load macro game");
                    eprintln!("{}", s);
                    if let Ok(mut rw) = RWops::from_file(Path::new(&file), "w+")
                    {
                        rw.write(s.as_bytes())
                            .expect("failed to load macro game");
                    } else {
                        let mut rw = RWops::from_file(Path::new(&file), "w")
                            .expect("failed to load macro game");
                        rw.write(s.as_bytes()).expect("failed to write");
                    }
                    flame::dump_html(
                        &mut File::create("flame-graph.html").unwrap(),
                    )
                    .unwrap();
                }
                sdl2::event::Event::Window {
                    win_event: sdl2::event::WindowEvent::Resized(w, h),
                    ..
                } => {
                    let mut viewport =
                        specs_world.write_resource::<red::Viewport>();
                    viewport.update_size(w, h);
                    let context =
                        specs_world.read_resource::<ThreadPin<red::GL>>();
                    viewport.set_used(&*context);
                }
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
