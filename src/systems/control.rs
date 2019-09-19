use sdl2::keyboard::{Keycode, Scancode};
use std::collections::HashSet;

pub use super::*;
use physics::*;
use log::info;

pub struct ControlSystem {
    reader: ReaderId<Keycode>,
    prev_keys: HashSet<Keycode>,
    new_keys: HashSet<Keycode>,
}

impl ControlSystem {
    pub fn new(reader: ReaderId<Keycode>) -> Self {
        ControlSystem { 
            reader: reader,
            prev_keys: HashSet::new(),
            new_keys: HashSet::new(),
        }
    }
}

impl<'a> System<'a> for ControlSystem {
    type SystemData = (
        (
            Entities<'a>,
            WriteStorage<'a, Isometry>,
            WriteStorage<'a, Velocity>,
            WriteStorage<'a, PhysicsComponent>,
            WriteStorage<'a, Spin>,
            WriteStorage<'a, ShotGun>,
            WriteStorage<'a, MultyLazer>,
            WriteStorage<'a, Lifes>,
            WriteStorage<'a, Shield>,
            WriteStorage<'a, Polygon>,
            ReadStorage<'a, CharacterMarker>,
            ReadStorage<'a, AsteroidMarker>,
            WriteStorage<'a, ShipStats>,
            WriteStorage<'a, Rift>,
        ),
        Read<'a, EventChannel<Keycode>>,
        Read<'a, Mouse>,
        Write<'a, EventChannel<Sound>>,
        ReadExpect<'a, PreloadedSounds>,
        ReadExpect<'a, PreloadedImages>,
        Write<'a, World<f32>>,
        Write<'a, BodiesMap>,
        Write<'a, EventChannel<InsertEvent>>,
        Write<'a, AppState>,
        WriteExpect<'a, Canvas>,
        Write<'a, Progress>,
        WriteExpect<'a, MacroGame>,
        WriteExpect<'a, DevInfo>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            (
                entities,
                isometries,
                mut velocities,
                physics,
                mut spins,
                mut shotguns,
                mut multiple_lazers,
                mut lifes,
                mut shields,
                polygons,
                character_markers,
                asteroid_markers,
                mut ships_stats,
                mut rifts,
            ),
            keys_channel,
            mouse_state,
            mut sounds_channel,
            preloaded_sounds,
            preloaded_images,
            mut world,
            bodies_map,
            mut insert_channel,
            mut app_state,
            mut canvas,
            mut progress,
            mut macro_game,
            mut dev_info
        ) = data;
        info!("asteroids: started control system");
        let (ship_stats, _) = if let Some(value) = (&mut ships_stats, &character_markers).join().next() {
            value
        } else {
            return
        };
        #[cfg(not(target_os = "android"))]
        {
            let mut character = None;
            for (entity, iso, _vel, spin, _char_marker) in (
                &entities,
                &isometries,
                &mut velocities,
                &mut spins,
                &character_markers,
            )
                .join()
            {
                character = Some(entity);
                let player_torque = DT
                    * calculate_player_ship_spin_for_aim(
                        Vector2::new(mouse_state.x, mouse_state.y)
                            - Vector2::new(iso.0.translation.vector.x, iso.0.translation.vector.y),
                        iso.rotation(),
                        spin.0,
                    );
                spin.0 += player_torque
                    .max(-MAX_TORQUE)
                    .min(MAX_TORQUE);

            }
            let character = character.unwrap();
            let (character_isometry, mut character_velocity) = {
                let character_body = world
                    .rigid_body(physics.get(character).unwrap().body_handle)
                    .unwrap();
                (*character_body.position(), *character_body.velocity())
            };
            if let Some(multy_lazer) = multiple_lazers.get_mut(character) {
                if mouse_state.left {
                    multy_lazer.set_all(true);
                } else {
                    multy_lazer.set_all(false);
                }
            }
            let mut process_lazer = |
                isometry: &Isometry3,
                lazer: &mut Lazer,
                world: &mut Write<World<f32>>,
                bodies_map: & Write<BodiesMap>,
                is_character: bool,
                rotation,
            | {
                // let body = world
                //     .rigid_body(physics_component.body_handle)
                //     .unwrap();
                // let isometry = body.position();
                let isometry = iso3_iso2(isometry);
                let position = isometry.translation.vector;
                let pos = Point2::new(position.x, position.y);
                let dir = isometry * (rotation * Vector2::new(0f32, -1f32));
                let ray = Ray::new(pos, dir);
                let collision_groups = if is_character {
                    get_collision_groups(&EntityType::Player)
                } else {
                    get_collision_groups(&EntityType::Enemy)
                };
                let (min_d, closest_body) = get_min_dist(
                    world, 
                    ray, 
                    collision_groups
                );
                if min_d < lazer.distance {
                    lazer.current_distance = min_d;
                    if let Some(target_entity) = bodies_map.get(&closest_body.unwrap()) {
                        if let Some(_) = lifes.get(*target_entity) {
                            let mut explosion_size = 1;
                            if process_damage(
                                lifes.get_mut(*target_entity).unwrap(),
                                shields.get_mut(*target_entity),
                                lazer.damage
                            ) {
                                let explosion_isometry = isometries.get(*target_entity).unwrap().0;
                                let explosion_position = explosion_isometry.translation.vector;
                                let explosion_position =
                                    Point2::new(explosion_position.x, explosion_position.y);
                                if asteroid_markers.get(*target_entity).is_some() {
                                    let asteroid = *target_entity;
                                    let polygon = polygons.get(asteroid).unwrap();
                                    asteroid_explode(
                                        explosion_position,
                                        &mut insert_channel,
                                        &mut sounds_channel,
                                        &preloaded_sounds,
                                        &preloaded_images,
                                        polygon.max_r
                                    );
                                    spawn_asteroids(
                                        explosion_isometry, 
                                        polygons.get(asteroid).unwrap(), 
                                        &mut insert_channel,
                                        None
                                    );
                                } else {
                                    let target_position = isometries
                                        .get(*target_entity).unwrap().0.translation.vector;
                                    ship_explode(
                                        Point2::new(target_position.x, target_position.y),
                                        &mut insert_channel,
                                        &mut sounds_channel,
                                        &preloaded_sounds,
                                    );
                                }
                                explosion_size = 20;
                                insert_channel.single_write(InsertEvent::Wobble(EXPLOSION_WOBBLE));
                                if character_markers.get(*target_entity).is_some() {
                                    to_menu(&mut app_state, &mut progress, &mut macro_game.score_table);
                                }
                                let effect_position = position + dir * min_d;
                                let effect = InsertEvent::Explosion {
                                    position: Point2::new(effect_position.x, effect_position.y),
                                    num: explosion_size,
                                    lifetime: Duration::from_millis(800),
                                    with_animation: None
                                };
                                insert_channel.single_write(effect);
                                entities.delete(*target_entity).unwrap();
                            } 
                        }
                    }
                } else {
                    lazer.current_distance = lazer.distance
                }
            };
            info!("asteroids: started process rifts");
            let mut upgdate_rifts = vec![];
            let zero_rotation = Rotation2::new(0.0);
            for (e1, r1) in (&entities, &rifts).join() {
                for (e2, _r2) in (&entities, &rifts).join() {
                    // if e1 == e2 {break};
                    let pos1 = isometries.get(e1).unwrap().0.translation.vector;
                    let pos2 = isometries.get(e2).unwrap().0.translation.vector;
                    if (pos1 - pos2).norm() > r1.distance {continue};
                    let up = Vector2::new(0.0, -1.0);
                    let dir = pos2 - pos1;
                    let mut lazer = Lazer {damage: 5, active: true, distance: dir.norm(), current_distance: dir.norm()};
                    let dir = Vector2::new(dir.x, dir.y);
                    let rotation = Rotation2::rotation_between(&up, &dir);
                    let isometry = Isometry3::new(
                        Vector3::new(pos1.x, pos1.y, pos1.z), Vector3::new(0f32, 0f32, rotation.angle())
                    );

                    process_lazer(
                        &isometry,
                        &mut lazer,
                        &mut world,
                        &bodies_map,
                        character_markers.get(e1).is_some(),
                        zero_rotation
                    );
                    upgdate_rifts.push((e1, lazer.clone(), dir.normalize()));
                    // render_lazer(&Isometry(isometry), &lazer, false, zero_rotation);
                }
            }
            for rift in (&mut rifts).join() {
                rift.lazers = vec![];
            }
            for (e, lazer, dir) in upgdate_rifts.into_iter() {
                let rift = rifts.get_mut(e).unwrap();
                rift.lazers.push((lazer, (dir.x, dir.y)));
            }
            info!("asteroids: ended process rifts");

            info!("asteroids: started process multy lazers");
            for (entity, isometry, multiple_lazers) in (&entities, &isometries, &mut multiple_lazers).join() {
                for (angle, lazer) in multiple_lazers.iter_mut() {
                    // let rotation = Rotation2::new(i as f32 * std::f32::consts::PI / 2.0);
                    let rotation = Rotation2::new(angle);
                    if !lazer.active {
                        continue
                    }
                    process_lazer(
                        &isometry.0,
                        lazer,
                        &mut world,
                        &bodies_map,
                        character_markers.get(entity).is_some(),
                        rotation
                    )
                }
            }
            info!("asteroids: ended process multy lazers");
            info!("asteroids: started process crazyness");
            let gun_position = Point2::new(
                    character_isometry.translation.vector.x, 
                    character_isometry.translation.vector.y
                );
            if mouse_state.left {
                if let Some(shotgun) = shotguns.get_mut(character) {
                    if shotgun.shoot() {
                        let bullets = shotgun.spawn_bullets(
                            EntityType::Player,
                            isometries.get(character).unwrap().0,
                            shotgun.bullet_speed,
                            shotgun.bullets_damage,
                            velocities.get(character).unwrap().0,
                            character
                        );
                        info!("asteroids: bullets {:?} processed", bullets);
                        sounds_channel.single_write(
                            Sound(
                                preloaded_sounds.shot,
                                gun_position
                            )
                        );
                        insert_channel.iter_write(bullets.into_iter());
                    }
                }
            }
            info!("asteroids: started reading keys");
            self.prev_keys = self.new_keys.clone();
            self.new_keys.clear();
            for key in keys_channel.read(&mut self.reader) {
                self.new_keys.insert(*key);
                let mut thrust = match key {
                    Keycode::W => {
                        ship_stats.thrust_force * Vector3::new(0.0, -1.0, 0.0)
                    }
                    Keycode::S => {
                        ship_stats.thrust_force * Vector3::new(0.0, 1.0, 0.0)
                    }
                    Keycode::A => {
                        ship_stats.thrust_force * Vector3::new(-1.0, 0.0, 0.0)
                    }
                    Keycode::D => {
                        ship_stats.thrust_force * Vector3::new(1.0, 0.0, 0.0)
                    }
                    _ => {
                        Vector3::new(0f32, 0f32, 0f32)
                    }
                };
                match key {
                    Keycode::LeftBracket => {
                        canvas.z_far -= 0.5;
                    }
                    Keycode::RightBracket => {
                        canvas.z_far += 0.5;
                    }
                    _ => ()
                };
                let maneuverability = ship_stats.maneuverability.unwrap();
                let depth = 30.0 * thrust.norm();
                let scalar = thrust.normalize().dot(&*character_velocity.as_vector());
                if scalar < depth {
                    let x = scalar - depth;
                    thrust *= maneuverability * (1.0 + x.abs() * x.abs().sqrt());
                }
                *character_velocity.as_vector_mut() += thrust;
            };
            let new_pressed = &self.new_keys - &self.prev_keys;
            for key in new_pressed.iter() {
                match key {
                    Keycode::Space => {
                        *app_state = AppState::Play(PlayState::Upgrade)
                    }
                    Keycode::T => {
                        dev_info.draw_telemetry = !dev_info.draw_telemetry;
                    }
                    _ => ()
                }
            }
            info!("asteroids: ended reading keys");
            if mouse_state.right {
                let rotation = isometries.get(character).unwrap().0.rotation;
                let _vel = velocities.get_mut(character).unwrap();
                let thrust = ship_stats.thrust_force * (rotation * Vector3::new(0.0, 1.0, 0.0));
                *character_velocity.as_vector_mut() += thrust;
            }
            let character_body = world
                .rigid_body_mut(physics.get(character).unwrap().body_handle)
                .unwrap();
            character_body.set_velocity(character_velocity);
        }
        info!("asteroids: ended process crazyness");
        info!("asteroids: ended control system");
    }
}
