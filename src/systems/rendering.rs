use crate::gfx::{TextData, RenderMode};
use std::collections::{HashMap};
use std::cmp::Ordering::Equal;
use super::*;
use crate::gui::VecController;
use glyph_brush::{Section, rusttype::Scale};
use crate::geometry::{shadow_geometry};
const LIGHT_RECTANGLE_SIZE: f32 = 10f32;

pub struct MenuRenderingSystem {
    reader: ReaderId<Primitive>,
}

impl MenuRenderingSystem {
    pub fn new(reader: ReaderId<Primitive>) -> Self {
        MenuRenderingSystem{
            reader: reader
        }
    }
}

impl<'a> System<'a> for MenuRenderingSystem {
    type SystemData = (
        ReadStorage<'a, ThreadPin<ImageData>>,
        ReadExpect<'a, ThreadPin<red::GL>>,
        WriteExpect<'a, Canvas>,
        ReadExpect<'a, red::Viewport>,
        Write<'a, EventChannel<Primitive>>,
        Write<'a, IngameUI>,
        Write<'a, EventChannel<InsertEvent>>,
        WriteExpect<'a, PreloadedImages>,
        Read<'a, Mouse>,
        Write<'a, AppState>,
        Write<'a, MenuChosedGun>,
        WriteExpect<'a, ThreadPin<TextData<'static>>>,
        ReadExpect<'a, Description>,
        Read<'a, Vec<UpgradeCardRaw>>,
        Write<'a, Vec<UpgradeCard>>,
        Read<'a, HashMap<String, specs::Entity>>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            image_datas,
            gl,
            mut canvas,
            viewport,
            mut primitives_channel,
            mut ui,
            mut insert_channel,
            preloaded_images,
            mouse,
            mut app_state,
            // text_data
            mut chosed_gun,
            mut text_data,
            description,
            upgrade_cards_raw,
            mut avaliable_upgrades,
            name_to_image,
        ) = data;
        let mut frame = red::Frame::new(&gl);
        frame.set_clear_color(0.0, 0.0, 0.0, 1.0);
        frame.clear_color();
        let dims = viewport.dimensions();
        let (w, h) = (dims.0 as f32, dims.1 as f32);
        // return;

        let (button_w, button_h) = (w/4f32, h/4f32);
        let lazer_button = Button::new(
            Point2::new(0f32, button_h / 2f32),
            button_w,
            button_h,
            Point3::new(0f32, 0f32, 0f32),
            false,
            Some(Image(preloaded_images.lazer)),
            "Lazer gun".to_string()
        );
        let blaster_button = Button::new(
            Point2::new(button_w + 0.1, button_h / 2f32),
            button_w,
            button_h,
            Point3::new(0f32, 0f32, 0f32),
            false,
            Some(Image(preloaded_images.blaster)),
            "Lazer gun".to_string()
        );

        let shotgun_button = Button::new(
            Point2::new(2.0 * button_w + 0.1, button_h / 2f32),
            button_w,
            button_h,
            Point3::new(0f32, 0f32, 0f32),
            false,
            Some(Image(preloaded_images.shotgun)),
            "Lazer gun".to_string()
        );
        if lazer_button.place_and_check(&mut ui, &*mouse) {
            chosed_gun.0 = Some(description.player_guns[0])
        }
        if blaster_button.place_and_check(&mut ui, &*mouse) {
            chosed_gun.0 = Some(description.player_guns[1])
        }
        if shotgun_button.place_and_check(&mut ui, &*mouse) {
            chosed_gun.0 = Some(description.player_guns[2])
        }
        let button = Button::new(
            Point2::new(w/2.0 - button_w / 2.0, h - button_h), 
            // Point2::new(0f32, 0f32),
            button_w, 
            button_h, 
            Point3::new(0.1f32, 0.4f32, 1f32), 
            false, 
            None, 
            "Play".to_string()
        );
        if let Some(gun) = chosed_gun.0 {
            if button.place_and_check(&mut ui, &*mouse) {
                *app_state = AppState::Play(PlayState::Action);
                insert_channel.single_write(InsertEvent::Character{ 
                    gun_kind: gun, 
                    ship_stats: description.player_ships_stats[0]
                });
                chosed_gun.0 = None;
                *avaliable_upgrades = get_avaliable_cards(
                    &upgrade_cards_raw,
                    &gun,
                    &name_to_image
                );
            }
        }
        primitives_channel.iter_write(ui.primitives.drain(..));
        render_primitives(
            &mouse,
            &mut self.reader,
            &mut frame,
            &image_datas,
            &gl,
            &mut canvas,
            &viewport,
            &mut primitives_channel,
            &mut text_data,
        );
    }
}

pub fn render_primitives<'a>(
    mouse: &Read<'a, Mouse>,
    reader: &mut ReaderId<Primitive>,
    frame: &mut red::Frame,
    image_datas: &ReadStorage<'a, ThreadPin<ImageData>>,
    gl: &ReadExpect<'a, ThreadPin<red::GL>>,
    canvas: &mut WriteExpect<'a, Canvas>,
    viewport: &ReadExpect<'a, red::Viewport>,
    primitives_channel: &mut Write<'a, EventChannel<Primitive>>,
    text_data: &mut WriteExpect<'a, ThreadPin<TextData<'static>>>,
) {
    let dims = viewport.dimensions();
    let (w, h) = (dims.0 as f32, dims.1 as f32);
    let scale = Scale::uniform((w / 6000.0 * mouse.hdpi as f32).round());
    for primitive in primitives_channel.read(reader) {
        match primitive {
            Primitive {
                kind: PrimitiveKind::Rectangle(rectangle),
                with_projection,
                image
            } => {
                let (model, points, indicies) = rectangle.get_geometry();
                let geom_data =
                    GeometryData::new(&gl, &points, &indicies).unwrap();
                match image {
                    Some(image) => {
                        canvas
                            .render_primitive_texture(
                                &gl, 
                                &viewport,
                                frame, 
                                image_datas.get(image.0).unwrap(),
                                &model, 
                                *with_projection, 
                                rectangle.height
                            );
                    }
                    None => {
                        let fill_color = rectangle.color;
                        canvas.render_primitive(
                            &gl,
                            &viewport,
                            frame,
                            &geom_data,
                            &model,
                            (fill_color.x, fill_color.y, fill_color.z),
                            *with_projection,
                            RenderMode::Draw
                        );
                        // canvas
                        //     .render_primitive(&display, &mut target, &geom_data, &model, rectangle.color, *with_projection)
                        //     .unwrap();

                    }
                }
            }
            Primitive {
                kind: PrimitiveKind::Text(text),
                with_projection: _,
                image: _
            } => {
                text_data.glyph_brush.queue(Section {
                    text: &text.text,
                    scale,
                    screen_position: (text.position.x, text.position.y),
                    bounds: (w /3.15, h),
                    color: [1.0, 1.0, 1.0, 1.0],
                    ..Section::default()
                });
            }
        }
    }
    canvas.render_text(
        text_data,
        &viewport,
        frame
    );
}

#[derive(Default)]
pub struct GUISystem;

impl<'a> System<'a> for GUISystem {
    type SystemData = (
        (
            Entities<'a>,
            ReadStorage<'a, Isometry>,
            WriteStorage<'a, Velocity>,
            ReadStorage<'a, PhysicsComponent>,
            ReadStorage<'a, CharacterMarker>,
            ReadStorage<'a, ShipMarker>,
            ReadStorage<'a, Lifes>,
            ReadStorage<'a, Shield>,
            WriteStorage<'a, Spin>,
            WriteStorage<'a, Blaster>,
            WriteStorage<'a, ShipStats>,
            ReadExpect<'a, red::Viewport>,
        ),
        Write<'a, World<f32>>,
        Write<'a, EventChannel<Primitive>>,
        Write<'a, IngameUI>,
        Read<'a, Progress>,
        Write<'a, AppState>,
        Read<'a, Mouse>,
        WriteExpect<'a, PreloadedImages>,
        ReadExpect<'a, PreloadedSounds>,
        Write<'a, Touches>,
        Write<'a, EventChannel<Sound>>,
        Write<'a, EventChannel<InsertEvent>>,
        Read<'a, AvaliableUpgrades>,
        Write<'a, SpawnedUpgrades>,
        WriteExpect<'a, ChoosedUpgrade>,
        Read<'a, CurrentWave>,
    );

    fn run(&mut self, data: Self::SystemData) {
          let (
            (
                entities,
                isometries,
                mut velocities,
                physics,
                character_markers,
                ship_markers,
                lifes,
                shields,
                mut spins,
                mut blasters,
                mut ships_stats,
                viewport,
            ),
            // preloaded_particles,
            mut world,
            _primitives_channel,
            mut ingame_ui,
            progress,
            mut app_state,
            mouse,
            preloaded_images,
            preloaded_sounds,
            touches,
            mut sounds_channel,
            mut insert_channel,
            avaliable_upgrades,
            mut spawned_upgrades,
            mut choosed_upgrade,
            current_wave
        ) = data;
        let dims = viewport.dimensions();
        let (w, h) = (dims.0 as f32, dims.1 as f32);
        let life_color = Point3::new(0.0, 0.6, 0.1); // TODO move in consts?
        let shield_color = Point3::new(0.0, 0.1, 0.6); 
        let experience_color = Point3::new(0.8, 0.8, 0.8);
        let white_color = Point3::new(1.0, 1.0, 1.0);
        let grey_color = Point3::new(0.5, 0.5, 0.5);

        //contorls
        let stick_size = w / 80.0;
        let ctrl_size = stick_size * 10.0;
        let move_controller = VecController::new(
            Point2::new(ctrl_size, ctrl_size),
            ctrl_size,
            stick_size,
            Image(preloaded_images.circle)
        );
        let attack_controller = VecController::new(
            Point2::new(w - ctrl_size, ctrl_size),
            ctrl_size,
            stick_size,
            Image(preloaded_images.circle)
        );

            // UNCOMMENT TO ADD LIFE BARS
        // lifes and shields bars
        // for (entity, isometry, life, ship_stats, _ship) in (&entities, &isometries, &lifes, &ships_stats, &ship_markers).join() {
        //     let shield = shields.get(entity);
        //     // dbg!("draw lifes and shields");
        //     let position = isometry.0.translation.vector;
        //     let ship_lifes_bar = Rectangle {
        //         position: Point2::new(position.x, position.y),
        //         width: (life.0 as f32/ ship_stats.max_health as f32) * 1.5,
        //         height: 0.1,
        //         color: life_color.clone()
        //     };
            // if let Some(shield) = shield  {
            //     let ship_shield_bar = Rectangle {
            //         position: Point2::new(position.x, position.y - 1.0),
            //         width: (shield.0 as f32/ ship_stats.max_shield as f32) * 1.5,
            //         height: 0.1,
            //         color: shield_color.clone()
            //     };
            //     ingame_ui.primitives.push(
            //         Primitive {
            //             kind: PrimitiveKind::Rectangle(ship_shield_bar),
            //             with_projection: true,
            //             image: None
            //         }
            //     )
            // }
            // ingame_ui.primitives.push(
            //     Primitive {
            //         kind: PrimitiveKind::Rectangle(ship_lifes_bar),
            //         with_projection: true,
            //         image: None
            //     }
            // );
        // }
        
        let (character, ship_stats, _) = (&entities, &mut ships_stats, &character_markers).join().next().unwrap();
        // move controller
        match move_controller.set(
            0,
            &mut ingame_ui,
            &touches
        ) {
            Some(dir) => {
                let (character, _) = (&entities, &character_markers).join().next().unwrap();
                let (_character_isometry, mut character_velocity) = {
                    let character_body = world
                        .rigid_body(physics.get(character).unwrap().body_handle)
                        .unwrap();
                    (*character_body.position(), *character_body.velocity())
                };

                for (iso, _vel, spin, _char_marker) in (
                    &isometries,
                    &mut velocities,
                    &mut spins,
                    &character_markers,
                ).join()
                {
                    let player_torque = DT
                        * calculate_player_ship_spin_for_aim(
                            dir,
                            iso.rotation(),
                            spin.0,
                        );
                    spin.0 += player_torque.max(-MAX_TORQUE).min(MAX_TORQUE);
                }


                // let rotation = isometries.get(character).unwrap().0.rotation;
                // let thrust = player_stats.thrust_force * (rotation * Vector3::new(0.0, -1.0, 0.0));
                let thrust = ship_stats.thrust_force * Vector3::new(dir.x, dir.y, 0.0);
                *character_velocity.as_vector_mut() += thrust;
                let character_body = world
                    .rigid_body_mut(physics.get(character).unwrap().body_handle)
                    .unwrap();
                character_body.set_velocity(character_velocity);
            }
            None => ()
        }

        match attack_controller.set(
            1,
            &mut ingame_ui,
            &touches
        ) {
            Some(dir) => {
                let dir = dir.normalize();
                let blaster = blasters.get_mut(character);
                if let Some(blaster) = blaster {
                    if blaster.shoot() {
                        let isometry = *isometries.get(character).unwrap();
                        let position = isometry.0.translation.vector;
                        // let direction = isometry.0 * Vector3::new(0f32, -1f32, 0f32);
                        let velocity_rel = blaster.bullet_speed * dir;
                        let char_velocity = velocities.get(character).unwrap();
                        let projectile_velocity = Velocity::new(
                            char_velocity.0.x + velocity_rel.x,
                            char_velocity.0.y + velocity_rel.y,
                        ) ;
                        sounds_channel.single_write(Sound(preloaded_sounds.shot));
                        let rotation = Rotation2::rotation_between(&Vector2::new(0.0, 1.0), &dir);
                        insert_channel.single_write(InsertEvent::Bullet {
                            kind: EntityType::Player,
                            iso: Point3::new(position.x, position.y, rotation.angle()),
                            velocity: Point2::new(projectile_velocity.0.x, projectile_velocity.0.y),
                            damage: blaster.bullets_damage,
                            owner: character,
                        });
                    }
                }

            }
            None => ()
        }

        // stats
        ingame_ui.primitives.push(
            Primitive {
                kind: PrimitiveKind::Text(Text {
                    position: Point2::new(w - w/7.0, h / 20.0), 
                    text: format!(
                        "Score: {}", 
                        progress.score
                    ).to_string()
                }),
                with_projection: false,
                image: None
            }
        );

        ingame_ui.primitives.push(
            Primitive {
                kind: PrimitiveKind::Text(Text {
                    position: Point2::new(w - w/7.0, h / 7.0 + h / 20.0), 
                    text: format!(
                        "Wave: {}", 
                        current_wave.id
                    ).to_string()
                }),
                with_projection: false,
                image: None
            }
        );


        ingame_ui.primitives.push(
            Primitive {
                kind: PrimitiveKind::Text(Text {
                    position: Point2::new(w/20.0, h / 20.0), 
                    text: format!(
                        "Experience: {} %, \n Level {}",
                        100 * progress.experience / progress.current_max_experience() as usize, 
                        progress.level
                    ).to_string()
                }),
                with_projection: false,
                image: None
            }
        );

        let (character, _) = (&entities, &character_markers).join().next().unwrap();
        // "UI" things
        // experience and level bars
        let experiencebar_w = w / 5.0;
        let experiencebar_h = h / 100.0;
        let experience_position = Point2::new(w/2.0 - experiencebar_w / 2.0, h - h / 20.0);
        let experience_bar = Rectangle {
            position: experience_position,
            width: (progress.experience as f32 / progress.current_max_experience() as f32) * experiencebar_w,
            height: experiencebar_h,
            color: experience_color.clone()
        };
        let experience_bar_back = Rectangle {
            position: experience_position,
            width: experiencebar_w,
            height: experiencebar_h,
            color: white_color.clone()
        };
        ingame_ui.primitives.push(
            Primitive {
                kind: PrimitiveKind::Rectangle(experience_bar_back),
                with_projection: false,
                image: None
            }
        );
        ingame_ui.primitives.push(
            Primitive {
                kind: PrimitiveKind::Rectangle(experience_bar),
                with_projection: false,
                image: None
            }
        );
        // let ship_lifes_bar = Rectangle {
        //     position: Point2::new(position.x, position.y),
        //     width: (life.0 as f32/ MAX_SHIELDS as f32) * 1.5,
        //     height: 0.1,
        //     color: white_color
        // };

        // upgrade UI
        let mut current_upgrade = None;
        let (upgrade_button_w, upgrade_button_h) = ((w/4f32).min(h/2f32), (w/4f32).min(h/2f32));
        let (choose_button_w, choose_button_h) = (w/6f32, h/12f32);
        let shift = upgrade_button_h / 10f32;
        match *app_state {
            AppState::Play(PlayState::Upgrade) => {
                let upgrades = spawned_upgrades.last();
                if let Some(upgrades) = upgrades {
                    for (i, upg_id) in upgrades.iter().enumerate() {
                        let upg = &avaliable_upgrades[*upg_id];
                        let current_point = 
                            Point2::new(
                                w / 2.0 - upgrade_button_w - shift 
                                + i as f32 * (upgrade_button_w + shift), 
                                shift
                            );
                        let upgrade_button = Button::new(
                            current_point,
                            upgrade_button_w, upgrade_button_h, 
                            white_color.clone(), 
                            false,
                            Some(upg.image),
                            upg.name.clone()
                        );
                        if upgrade_button.place_and_check(&mut ingame_ui, &*mouse) {
                            // choosed_upgrade = Some(upg.upgrade_type);
                            choosed_upgrade.0 = *upg_id;
                            // *app_state = AppState::Play(PlayState::Action);
                            // with multytouch things it's not cheatable
                        }
                    }
                }
                let select_upgrade = Button::new(
                    Point2::new(w / 2.0 - choose_button_w - shift, h - 1.0 * choose_button_h),
                    choose_button_w, choose_button_h, 
                    grey_color.clone(), 
                    false,
                    None,
                    "Upgrade!".to_string()
                );

                if spawned_upgrades.len() > 0 {
                    ingame_ui.primitives.push(
                        Primitive {
                            kind: PrimitiveKind::Text(Text {
                                position: Point2::new(w/4.0, upgrade_button_h + shift),
                                text: avaliable_upgrades[choosed_upgrade.0].description.clone()
                            }),
                            with_projection: false,
                            image: None
                        }
                    );                    
                    if select_upgrade.place_and_check(&mut ingame_ui, &*mouse) {
                        current_upgrade = Some(avaliable_upgrades[choosed_upgrade.0].upgrade_type);
                        spawned_upgrades.pop();
                    }
                }
                let done_button = Button::new(
                    Point2::new(w / 2.0 + shift, h - 1.0 * choose_button_h),
                    choose_button_w, choose_button_h, 
                    grey_color.clone(), 
                    false,
                    None,
                    "Done".to_string()
                );
                if done_button.place_and_check(&mut ingame_ui, &*mouse) {
                    *app_state = AppState::Play(PlayState::Action);
                }
            }
            _ => ()
        }


        match current_upgrade {
            Some(choosed_upgrade) => {
                match choosed_upgrade {
                    UpgradeType::AttackSpeed => {
                        match blasters.get_mut(character) {
                            Some(gun) => {
                                gun.recharge_time = (gun.recharge_time as f32 * 0.9) as usize;
                            }
                            None => ()
                        }
                    }
                    UpgradeType::ShipSpeed => {
                        ship_stats.thrust_force += 0.1 * THRUST_FORCE_INIT;
                    }
                    UpgradeType::ShipRotationSpeed => {
                        ship_stats.torque += 0.1 * SHIP_ROTATION_SPEED_INIT;
                    }
                    UpgradeType::BulletSpeed => {
                        match blasters.get_mut(character) {
                            Some(gun) => {
                                gun.bullet_speed += 0.1 * BULLET_SPEED_INIT;
                            }
                            None => ()
                        }
                    }
                    UpgradeType::HealthRegen => {
                        ship_stats.health_regen += 1;
                    }
                    UpgradeType::ShieldRegen => {
                        ship_stats.shield_regen += 1;
                    }
                    UpgradeType::HealthSize => {
                        ship_stats.max_health += 20;
                    }
                    UpgradeType::ShieldSize => {
                        ship_stats.max_shield += 20;
                    }
                }
            }
            None => ()
        }


        for (life, shield, _character) in (&lifes, &shields, &character_markers).join() {
            let (lifebar_w, lifebar_h) = (w/4f32, h/50.0);
            let lifes_bar = Rectangle {
                position: Point2::new(w/2.0 - lifebar_w / 2.0, h/20.0),
                width: (life.0 as f32 / ship_stats.max_health as f32) * lifebar_w,
                height: lifebar_h,
                color: life_color.clone()
            };
            let shields_bar = Rectangle {
                position: Point2::new(w/2.0 - lifebar_w / 2.0, h/40.0),
                width: (shield.0 as f32 / ship_stats.max_shield as f32) * lifebar_w,
                height: lifebar_h,
                color: Point3::new(0.0, 0.1, 0.6)
            };
            let border = 0f32;
            let lifes_bar_back = Rectangle {
                position: Point2::new(w/2.0 - lifebar_w / 2.0 - border, h/40.0 - border + h/40.0 - border),
                width: lifebar_w + border * 2.0,
                height: lifebar_h + border * 2.0,
                color: Point3::new(1.0, 1.0, 1.0)
            };
            ingame_ui.primitives.push(
                Primitive {
                    kind: PrimitiveKind::Rectangle(shields_bar),
                    with_projection: false,
                    image: None
                }
            );
            ingame_ui.primitives.push(
                Primitive {
                    kind: PrimitiveKind::Rectangle(lifes_bar_back),
                    with_projection: false,
                    image: None
                }
            );
            ingame_ui.primitives.push(
                Primitive {
                    kind: PrimitiveKind::Rectangle(lifes_bar),
                    with_projection: false,
                    image: None
                }
            );
        }
    }
}

pub struct RenderingSystem {
    reader: ReaderId<Primitive>,
}

impl RenderingSystem {
    pub fn new(reader: ReaderId<Primitive>) -> Self{
        RenderingSystem {
            reader: reader
        }
    }
}

impl<'a> System<'a> for RenderingSystem {
    type SystemData = (
        (
            Entities<'a>,
            ReadStorage<'a, Isometry>,
            ReadStorage<'a, Velocity>,
            ReadStorage<'a, PhysicsComponent>,
            ReadStorage<'a, CharacterMarker>,
            ReadStorage<'a, ShipMarker>,
            ReadStorage<'a, AsteroidMarker>,
            ReadStorage<'a, LightMarker>,
            ReadStorage<'a, NebulaMarker>,
            ReadStorage<'a, Projectile>,
            ReadStorage<'a, ThreadPin<ImageData>>,
            ReadStorage<'a, Image>,
            WriteStorage<'a, Animation>,
            ReadStorage<'a, Size>,
            ReadStorage<'a, Polygon>,
            ReadStorage<'a, Lazer>,
            ReadStorage<'a, Geometry>,
            WriteStorage<'a, ThreadPin<ParticlesData>>,
        ),
        Read<'a, Mouse>,
        ReadExpect<'a, ThreadPin<red::GL>>,
        ReadExpect<'a, red::Viewport>,
        WriteExpect<'a, Canvas>,
        ReadExpect<'a, PreloadedParticles>,
        Read<'a, World<f32>>,
        Write<'a, EventChannel<Primitive>>,
        Write<'a, IngameUI>,
        WriteExpect<'a, ThreadPin<TextData<'static>>>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            (
                entities,
                isometries,
                velocities,
                physics,
                character_markers,
                ship_markers,
                asteroid_markers,
                light_markers,
                nebulas,
                projectiles,
                image_datas,
                image_ids,
                mut animations,
                sizes,
                polygons,
                lazers,
                geometries,
                mut particles_datas,
            ),
            mouse,
            gl,
            viewport,
            mut canvas,
            preloaded_particles,
            world,
            mut primitives_channel,
            mut ingame_ui,
            mut text_data
        ) = data;
        let mut frame = red::Frame::new(&gl);
        frame.set_clear_color(0.015, 0.004, 0.0, 1.0);
        frame.set_clear_stencil(0);
        frame.clear_color_and_stencil();
        let (char_iso, char_pos, char_vel) = {
            let mut opt_iso = None;
            let mut opt_vel = None;
            for (iso, vel, _) in (&isometries, &velocities, &character_markers).join() {
                canvas.update_observer(
                    Point2::new(iso.0.translation.vector.x, iso.0.translation.vector.y),
                    vel.0.norm() / VELOCITY_MAX,
                    Vector2::new(mouse.x01, mouse.y01).normalize()
                );
                opt_iso = Some(iso);
                opt_vel = Some(vel);
            }
            (
                opt_iso.unwrap().0,
                opt_iso.unwrap().0.translation.vector,
                opt_vel.unwrap().0
            )
        };
        for (_entity, iso, geom, _) in (&entities, &isometries, &geometries, &asteroid_markers).join() {
            let pos = Point2::new(iso.0.translation.vector.x, iso.0.translation.vector.y);
            // light_poly.clip_one((*geom).clone(), pos);
            let rotation = iso.0.rotation.euler_angles().2;
            let rotation = Rotation2::new(rotation);
            let shadow_triangulation = shadow_geometry(
                Point2::new(char_pos.x, char_pos.y),
                (*geom).clone(),
                pos,
                rotation
            );
            if let Some(shadow_triangulation) = shadow_triangulation {
                let geometry_data =  GeometryData::new(
                    &gl, 
                    &shadow_triangulation.points, 
                    &shadow_triangulation.indicies)
                .unwrap();
                let iso = Isometry3::new(iso.0.translation.vector, Vector3::new(0f32, 0f32, 0f32));
                canvas
                    .render_geometry(
                        &gl, &viewport,
                        &mut frame,
                        &geometry_data,
                        &iso,
                        RenderMode::StencilWrite
                    );
            }
        }
        for (_entity, iso, image, size, _nebula) in
            (&entities, &isometries, &image_ids, &sizes, &nebulas).join() {
            let image_data = image_datas.get(image.0).unwrap();
            canvas
                .render(
                        &gl,
                        &viewport,
                        &mut frame,
                        &image_data,
                        &iso.0,
                        size.0,
                        false
                );
        };
        {
            for (entity, particles_data) in (&entities, &mut particles_datas).join() {
                match **particles_data {
                    ParticlesData::Engine(ref mut particles) => {
                        let mut direction = Vector3::new(0f32, 1f32, 0f32);
                        direction = char_iso * direction;
                        if particles.update(
                            Vector2::new(char_pos.x, char_pos.y),
                            Vector2::new(char_vel.x, char_vel.y),
                            Vector2::new(direction.x, direction.y)
                        ) {
                            canvas
                                .render_instancing(
                                    &gl,
                                    &viewport,
                                    &mut frame,
                                    &particles.instancing_data,
                                    &Isometry3::new(
                                        Vector3::new(0f32, 0f32, 0f32),
                                        Vector3::new(0f32, 0f32, 0f32),
                                    ),
                                );
                        } else {
                            entities.delete(entity).unwrap();
                        }
                    }
                    ParticlesData::Explosion(ref mut particles) => {
                        if particles.update() {
                            canvas
                                .render_instancing(
                                    &gl,
                                    &viewport,
                                    &mut frame,
                                    &particles.instancing_data,
                                    &Isometry3::new(
                                        Vector3::new(0f32, 0f32, 0f32),
                                        Vector3::new(0f32, 0f32, 0f32),
                                    )
                                );
                        } else {
                            entities.delete(entity).unwrap();
                        }
                }
                    _ => ()
                };
            }
        }

        for (iso, vel, _char_marker) in (&isometries, &velocities, &character_markers).join() {
            let translation_vec = iso.0.translation.vector;
            let mut isometry = Isometry3::new(translation_vec, Vector3::new(0f32, 0f32, 0f32));
            let pure_isometry = isometry.clone();
            isometry.translation.vector.z = canvas.get_z_shift();
            match **particles_datas
                .get_mut(preloaded_particles.movement)
                .unwrap()
            {
                ParticlesData::MovementParticles(ref mut particles) => {
                    particles.update(1.0 * Vector2::new(-vel.0.x, -vel.0.y));
                     canvas
                        .render_instancing(
                            &gl,
                            &viewport,
                            &mut frame,
                            &particles.instancing_data,
                            &pure_isometry,
                        );
                }
                _ => panic!(),
            };
        }



        for (_entity, iso, image, size, _projectile) in
            (&entities, &isometries, &image_ids, &sizes, &projectiles).join()
        {
            canvas
                .render(
                    &gl,
                    &viewport,
                    &mut frame,
                    &image_datas.get(image.0).unwrap(),
                    &iso.0,
                    size.0,
                    true
                );
        }
        for (_entity, iso, image, size, _light) in
            (&entities, &isometries, &image_ids, &sizes, &light_markers).join()
        {
            let translation_vec = iso.0.translation.vector;
            let isometry = Isometry3::new(translation_vec, Vector3::new(0f32, 0f32, 0f32));
            canvas
                .render(
                    &gl,
                    &viewport,
                    &mut frame,
                    &image_datas.get(image.0).unwrap(),
                    &isometry,
                    size.0,
                    true,
                );
        }
        for (_entity, physics_component, image, size, _ship) in
                    (&entities, &physics, &image_ids, &sizes, &ship_markers).join() {
                let iso2 = world
                    .rigid_body(physics_component.body_handle)
                    .unwrap()
                    .position();
                let iso = iso2_iso3(iso2);
                let image_data = &image_datas.get(image.0).unwrap();
                canvas
                    .render(
                        &gl,
                        &viewport,
                        &mut frame,
                        &image_data,
                        &iso,
                        size.0,
                        true,
                    )
        }
        for (_entity, iso, _image, _size, polygon, _asteroid) in (
            &entities,
            &isometries,
            &image_ids,
            &sizes,
            &polygons,
            &asteroid_markers,
        )
            .join()
        {
            let triangulation = polygon.triangulate();
            let geom_data =
                GeometryData::new(&gl, &triangulation.points, &triangulation.indicies).unwrap();
            canvas
                .render_geometry(
                    &gl, &viewport, 
                    &mut frame, 
                    &geom_data, 
                    &iso.0,
                    RenderMode::Draw
                )
        }
        for (iso, lazer) in (&isometries, &lazers).join() {
            if lazer.active {
                let h = lazer.current_distance;
                let w = 0.05f32;
                let positions = [
                    Point2::new(-w / 2.0, 0f32),
                    Point2::new(w / 2.0, 0f32),
                    Point2::new(0.0, -h) // hmmmmm, don't know why minus
                ];
                // let indices = [0u16, 1, 2, 2, 3, 0];
                let indices = [0u16, 1, 2];
                let geometry_data = GeometryData::new(
                    &gl, &positions, &indices
                ).unwrap();
                canvas.render_geometry(
                    &gl,
                    &viewport,
                    &mut frame,
                    &geometry_data,
                    &iso.0,
                    RenderMode::Draw
                );
            }
        };
        for (iso, size, animation) in (&isometries, &sizes, &mut animations).join() {
            let animation_frame = animation.next_frame();
            if let Some(animation_frame) = animation_frame {
                let image_data = image_datas.get(animation_frame.image.0).unwrap();
                canvas
                    .render(
                        &gl,
                        &viewport,
                        &mut frame,
                        &image_data,
                        &iso.0,
                        size.0,
                        false
                    )
            };
        };
        primitives_channel.iter_write(ingame_ui.primitives.drain(..));
        render_primitives(
            &mouse,
            &mut self.reader,
            &mut frame,
            &image_datas,
            &gl,
            &mut canvas,
            &viewport,
            &mut primitives_channel,
            &mut text_data,
        );
    }
}