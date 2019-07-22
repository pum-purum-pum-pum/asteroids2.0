use super::*;
use crate::gui::VecController;

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
        ReadStorage<'a, Geometry>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, Polygon>,
        ReadExpect<'a, ThreadPin<red::GL>>,
        WriteExpect<'a, Canvas>,
        Read<'a, World<f32>>,
        ReadExpect<'a, red::Viewport>,
        Write<'a, EventChannel<Primitive>>,
        Write<'a, IngameUI>,
        Write<'a, EventChannel<InsertEvent>>,
        Read<'a, Mouse>,
        Write<'a, AppState>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
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
            geometries,
            sizes,
            polygons,
            gl,
            mut canvas,
            world,
            viewport,
            mut primitives_channel,
            mut ui,
            mut insert_channel,
            mouse,
            mut app_state,
            // text_data
        ) = data;
        let vao = red::buffer::VertexArray::new(&*gl).unwrap();
        let mut frame = red::Frame::new(&gl);
        frame.set_clear_color(0.0, 0.0, 0.0, 1.0);
        frame.clear_color();
        let dims = viewport.dimensions();
        // canvas.render_geometry(
            // gl: &red::GL,
            // viewport: &red::Viewport,
            // frame: &mut red::Frame,
            // vao: &red::buffer::VertexArray,
            // geometry_data: &GeometryData,
            // model: &Isometry3,
        // );
    //     let mut target = display.draw();
    //     target.clear_color(0.0, 0.0, 0.0, 1.0);
    //     target.clear_stencil(0i32);
    //     let dims = display.get_framebuffer_dimensions();
        let (w, h) = (dims.0 as f32, dims.1 as f32);
        let (button_w, button_h) = (w/4f32, h/4f32);
        let button = Button::new(
            Point2::new(w/2.0 - button_w / 2.0, h/2.0 - button_h / 2.0), 
            // Point2::new(0f32, 0f32),
            button_w, 
            button_h, 
            Point3::new(0.1f32, 0.4f32, 1f32), 
            false, 
            None, 
            "Play".to_string()
        );
        if button.place_and_check(&mut ui, &*mouse) {
            *app_state = AppState::Play(PlayState::Action);
            insert_channel.single_write(InsertEvent::Character);
        }
        primitives_channel.iter_write(ui.primitives.drain(..));
        for primitive in primitives_channel.read(&mut self.reader) {
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
                            // unimplemented!();
                            // canvas
                            //     .render_primitive_texture(
                            //         &display, 
                            //         &mut target, 
                            //         image_datas.get(image.0).unwrap(),
                            //         &model, 
                            //         *with_projection, 
                            //         rectangle.width
                            //     ).unwrap();
                        }
                        None => {
                            let fill_color = rectangle.color;
                            canvas.render_primitive(
                                &gl,
                                &viewport,
                                &mut frame,
                                &geom_data,
                                &model,
                                (fill_color.x, fill_color.y, fill_color.z),
                                *with_projection
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
                    // unimplemented!();
                    // let scale = 30f32;
                    // let orthographic = orthographic(dims.0, dims.1).to_homogeneous();
                    // let view = get_view(canvas.observer()).to_homogeneous();
                    // let model = Translation::from(Vector3::new(text.position.x, text.position.y, -1f32))
                    //     .to_homogeneous();
                    // let mut scaler = scale * Matrix4::identity();
                    // let scale_len = scaler.len();
                    // scaler[scale_len - 1] = 1.0;
                    // let matrix = orthographic * model * scaler;
                    // // let text = glium_text_rusttype::TextDisplay::new(&text_data.text_system, &text_data.font, &text.text);
                    // // glium_text_rusttype::draw(&text, &text_data.text_system, &mut target, matrix, (1.0, 1.0, 1.0, 1.0));
                }
                _ => ()
            }
        }
        // target.finish().unwrap();
    }
}

#[derive(Default)]
pub struct GUISystem;

impl<'a> System<'a> for GUISystem {
    type SystemData = (
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
        ReadExpect<'a, red::Viewport>,
        WriteExpect<'a, Canvas>,
        // ReadExpect<'a, PreloadedParticles>,
        Write<'a, World<f32>>,
        Write<'a, EventChannel<Primitive>>,
        Write<'a, IngameUI>,
        Read<'a, Progress>,
        Write<'a, AppState>,
        Read<'a, Mouse>,
        Write<'a, PlayerStats>,
        WriteExpect<'a, PreloadedImages>,
        ReadExpect<'a, PreloadedSounds>,
        Write<'a, Touches>,
        Write<'a, EventChannel<Sound>>,
        Write<'a, EventChannel<InsertEvent>>,
        Read<'a, AvaliableUpgrades>,
    );

    fn run(&mut self, data: Self::SystemData) {
          let (
            entities,
            isometries,
            mut velocities,
            physics,
            character_markers,
            ship_markers,
            lifes,
            shields,
            mut spins,
            mut guns,
            viewport,
            mut canvas,
            // preloaded_particles,
            mut world,
            mut primitives_channel,
            mut ingame_ui,
            progress,
            mut app_state,
            mouse,
            mut player_stats,
            mut preloaded_images,
            preloaded_sounds,
            mut touches,
            mut sounds_channel,
            mut insert_channel,
            avaliable_upgrades
        ) = data;
        let (character, _) = (&entities, &character_markers).join().next().unwrap();
        let dims = viewport.dimensions();
        let (w, h) = (dims.0 as f32, dims.1 as f32);
        let dims = (dims.0 as u32, dims.1 as u32);
        
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


                for (entity, iso, _vel, spin, _char_marker) in (
                    &entities,
                    &isometries,
                    &mut velocities,
                    &mut spins,
                    &character_markers,
                ).join()
                {
                    let player_torque = dt
                        * calculate_player_ship_spin_for_aim(
                            dir,
                            iso.rotation(),
                            spin.0,
                        );
                    spin.0 += player_torque.max(-MAX_TORQUE).min(MAX_TORQUE);
                }


                // let rotation = isometries.get(character).unwrap().0.rotation;
                // let thrust = player_stats.thrust_force * (rotation * Vector3::new(0.0, -1.0, 0.0));
                let thrust = player_stats.thrust_force * Vector3::new(dir.x, dir.y, 0.0);
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
                let gun = guns.get_mut(character);
                if let Some(gun) = gun {
                    if gun.shoot() {
                        let isometry = *isometries.get(character).unwrap();
                        let position = isometry.0.translation.vector;
                        // let direction = isometry.0 * Vector3::new(0f32, -1f32, 0f32);
                        let velocity_rel = player_stats.bullet_speed * dir;
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
                            damage: gun.bullets_damage,
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
                    position: Point2::new(w/20.0, h - h / 20.0), 
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
        let life_color = Point3::new(0.0, 0.6, 0.1); // TODO move in consts?
        let shield_color = Point3::new(0.0, 0.1, 0.6); 
        let experience_color = Point3::new(0.8, 0.8, 0.8);
        let white_color = Point3::new(1.0, 1.0, 1.0);
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
        let mut choosed_upgrade = None;
        let (upgrade_button_w, upgrade_button_h) = ((w/4f32).min(h/2f32), (w/4f32).min(h/2f32));
        let shift = upgrade_button_h / 10f32;
        let mut rng = thread_rng();
        match *app_state {
            AppState::Play(PlayState::Upgrade{list: upgrades}) => {
                for (i, upg_id) in upgrades.iter().enumerate() {
                    let upg = &avaliable_upgrades[*upg_id];
                    let mut current_point = 
                        Point2::new(
                            i as f32 * (upgrade_button_w + shift), 
                            h - upgrade_button_h - shift
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
                        choosed_upgrade = Some(upg.upgrade_type);
                        *app_state = AppState::Play(PlayState::Action);
                    }

                }
            }
            _ => ()
        }

        match choosed_upgrade {
            Some(choosed_upgrade) => {
                match choosed_upgrade {
                    UpgradeType::AttackSpeed => {
                        match guns.get_mut(character) {
                            Some(gun) => {
                                gun.recharge_time = (gun.recharge_time as f32 * 0.9) as usize;
                            }
                            None => ()
                        }
                    }
                    UpgradeType::ShipSpeed => {
                        player_stats.thrust_force += 0.1 * THRUST_FORCE_INIT;
                    }
                    UpgradeType::ShipRotationSpeed => {
                        player_stats.ship_rotation_speed += 0.1 * SHIP_ROTATION_SPEED_INIT;
                    }
                    UpgradeType::BulletSpeed => {
                        player_stats.bullet_speed += 0.1 * BULLET_SPEED_INIT;
                    }
                }
            }
            None => ()
        }

        // lifes and shields bars
        for (isometry, life, shield, _ship) in (&isometries, &lifes, &shields, &ship_markers).join() {
            let position = isometry.0.translation.vector;
            // let position = unproject_with_z(
            //     canvas.observer(), 
            //     &Point2::new(position.x, position.y), 
            //     1f32, dims.0, dims.1
            // );
            // let position = ortho_unproject(dims.0, dims.1, Point2::new(position.x, position.y));
            let ship_lifes_bar = Rectangle {
                position: Point2::new(position.x, position.y),
                width: (life.0 as f32/ MAX_SHIELDS as f32) * 1.5,
                height: 0.1,
                color: life_color.clone()
            };
            let ship_shield_bar = Rectangle {
                position: Point2::new(position.x, position.y - 1.0),
                width: (shield.0 as f32/ MAX_SHIELDS as f32) * 1.5,
                height: 0.1,
                color: shield_color.clone()
            };
            ingame_ui.primitives.push(
                Primitive {
                    kind: PrimitiveKind::Rectangle(ship_lifes_bar),
                    with_projection: true,
                    image: None
                }
            );
            ingame_ui.primitives.push(
                Primitive {
                    kind: PrimitiveKind::Rectangle(ship_shield_bar),
                    with_projection: true,
                    image: None
                }
            )
        }

        for (life, shield, _character) in (&lifes, &shields, &character_markers).join() {
            let (lifebar_w, lifebar_h) = (w/4f32, h/50.0);
            let lifes_bar = Rectangle {
                position: Point2::new(w/2.0 - lifebar_w / 2.0, h/20.0),
                width: (life.0 as f32 / MAX_LIFES as f32) * lifebar_w,
                height: lifebar_h,
                color: life_color.clone()
            };
            let shields_bar = Rectangle {
                position: Point2::new(w/2.0 - lifebar_w / 2.0, h/40.0),
                width: (shield.0 as f32 / MAX_SHIELDS as f32) * lifebar_w,
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
            ReadStorage<'a, Geometry>,
            ReadStorage<'a, Size>,
            ReadStorage<'a, Polygon>,
            ReadStorage<'a, Lifes>,
            ReadStorage<'a, Shield>,
            ReadStorage<'a, Lazer>,
            WriteStorage<'a, ThreadPin<ParticlesData>>,
        ),
        ReadExpect<'a, ThreadPin<red::GL>>,
        ReadExpect<'a, red::Viewport>,
        WriteExpect<'a, Canvas>,
        ReadExpect<'a, PreloadedParticles>,
        Read<'a, World<f32>>,
        Write<'a, EventChannel<Primitive>>,
        Write<'a, IngameUI>,
        Read<'a, Progress>,
        Write<'a, AppState>,
        Read<'a, Mouse>,
        // ReadExpect<'a, ThreadPin<TextData>>,
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
                geometries,
                sizes,
                polygons,
                lifes,
                shields,
                lazers,
                mut particles_datas,
            ),
            gl,
            viewport,
            mut canvas,
            preloaded_particles,
            world,
            mut primitives_channel,
            mut ingame_ui,
            progress,
            mut app_state,
            mouse,
            // text_data
        ) = data;
        let dims = viewport.dimensions();
        let dims = (dims.0 as u32, dims.1 as u32);
        let mut frame = red::Frame::new(&gl);
        frame.set_clear_color(0.0, 0.0, 0.0, 1.0);
        frame.clear_color();
        // target.clear_stencil(0i32);
        let (char_iso, char_pos, char_vel) = {
            let mut opt_iso = None;
            let mut opt_vel = None;
            for (iso, vel, _) in (&isometries, &velocities, &character_markers).join() {
                canvas.update_observer(
                    Point2::new(iso.0.translation.vector.x, iso.0.translation.vector.y),
                    vel.0.norm() / VELOCITY_MAX,
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
        // {
        //     let rectangle = (
        //         char_pos.x - LIGHT_RECTANGLE_SIZE,
        //         char_pos.y - LIGHT_RECTANGLE_SIZE,
        //         char_pos.x + LIGHT_RECTANGLE_SIZE,
        //         char_pos.y + LIGHT_RECTANGLE_SIZE,
        //     );
        //     let mut light_poly = LightningPolygon::new_rectangle(
        //         rectangle.0,
        //         rectangle.1,
        //         rectangle.2,
        //         rectangle.3,
        //         Point2::new(char_pos.x, char_pos.y),
        //     );
        //     // TODO fix lights to be able to use without sorting
        //     let mut data = (&entities, &isometries, &geometries, &asteroid_markers)
        //         .join()
        //         .collect::<Vec<_>>(); // TODO move variable to field  to avoid allocations
        //     let distance = |a: &Isometry| (char_pos - a.0.translation.vector).norm();
        //     data.sort_by(|&a, &b| (distance(b.1).partial_cmp(&distance(a.1)).unwrap_or(Equal)));
        //     // UNCOMMENT TO ADD LIGHTS
        //     for (_entity, iso, geom, _) in data.iter() {
        //         let pos = Point2::new(iso.0.translation.vector.x, iso.0.translation.vector.y);
        //         if pos.x > rectangle.0
        //             && pos.x < rectangle.2
        //             && pos.y > rectangle.1
        //             && pos.y < rectangle.3
        //         {
        //             light_poly.clip_one(**geom, pos);
        //         }
        //     }
        //     let triangulation = light_poly.triangulate();
        //     let geom_data = GeometryData::new(&display, &triangulation.points, &triangulation.indicies);
            for (entity, particles_data) in (&entities, &mut particles_datas).join() {
                match **particles_data {
                    ParticlesData::Engine(ref mut particles) => {
                        let mut direction = Vector3::new(0f32, -1f32, 0f32);
                        direction = (char_iso * direction);
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
        // }

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
            // canvas
            //     .render_geometry(
            //         &gl,
            //         &viewport,
            //         &mut frame,
            //         &vao,
            //         &geom_data,
            //         &Isometry3::identity(),
            //         true,
            //     )
            //     .unwrap();
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
                    false
                );
            // canvas
            //     .render(
            //         &display,
            //         &mut target,
            //         &image_datas.get(image.0).unwrap(),
            //         &iso.0,
            //         size.0,
            //         false,
            //     );
        }
        for (_entity, iso, image, size, _light) in
            (&entities, &isometries, &image_ids, &sizes, &light_markers).join()
        {
            let mut translation_vec = iso.0.translation.vector;
            // translation_vec.z = canvas.get_z_shift();
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
                        false,
                    )
                // canvas
                //     .render(&display, &mut target, &image_datas.get(image.0).unwrap(), &iso, size.0, true)
                //     .unwrap();
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
            // dbg!("{:?}", triangulation);
            canvas
                .render_geometry(
                    &gl, &viewport, 
                    &mut frame, 
                    &geom_data, 
                    &iso.0, 
                )
        }
        for (iso, lazer) in (&isometries, &lazers).join() {
            if lazer.active {
                let h = lazer.current_distance;
                let w = 0.05f32;
                let positions = [
                    Point2::new(-w / 2.0, 0f32),
                    // Point2::new(-w / 2.0, -h),
                    // Point2::new(w / 2.0, -h),
                    Point2::new(w / 2.0, 0f32),
                    Point2::new(0.0, -h)
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
                    &iso.0
                );
            }
        };
        primitives_channel.iter_write(ingame_ui.primitives.drain(..));
        for primitive in primitives_channel.read(&mut self.reader) {
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
                                    &mut frame, 
                                    image_datas.get(image.0).unwrap(),
                                    &model, 
                                    *with_projection, 
                                    rectangle.width
                                );
                        }
                        None => {
                            let fill_color = (rectangle.color.x, rectangle.color.y, rectangle.color.z);
                            canvas
                                .render_primitive(&gl, &viewport, &mut frame, &geom_data, &model, fill_color, *with_projection);

                        }
                    }
                }
                Primitive {
                    kind: PrimitiveKind::Text(text),
                    with_projection: _,
                    image: _
                } => {
                    let scale = 30f32;
                    let orthographic = orthographic(dims.0, dims.1).to_homogeneous();
                    let view = get_view(canvas.observer()).to_homogeneous();
                    let model = Translation::from(Vector3::new(text.position.x, text.position.y, -1f32))
                        .to_homogeneous();
                    let mut scaler = scale * Matrix4::identity();
                    let scale_len = scaler.len();
                    scaler[scale_len - 1] = 1.0;
                    let matrix = orthographic * model * scaler;
                    // let text = glium_text_rusttype::TextDisplay::new(&text_data.text_system, &text_data.font, &text.text);
                    // glium_text_rusttype::draw(&text, &text_data.text_system, &mut target, matrix, (1.0, 1.0, 1.0, 1.0));
                }
                _ => ()
            }
        }
    }
}