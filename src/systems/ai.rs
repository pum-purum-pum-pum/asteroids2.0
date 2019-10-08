use super::*;
use log::info;
use physics::*;

#[derive(Default)]
pub struct AISystem;

impl<'a> System<'a> for AISystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Isometry>,
        WriteStorage<'a, Velocity>,
        ReadStorage<'a, PhysicsComponent>,
        WriteStorage<'a, Spin>,
        WriteStorage<'a, ShotGun>,
        WriteStorage<'a, MultyLazer>,
        WriteStorage<'a, Cannon>,
        WriteStorage<'a, RocketGun>,
        WriteStorage<'a, EnemyMarker>,
        WriteStorage<'a, Charge>,
        ReadStorage<'a, CharacterMarker>,
        ReadStorage<'a, AI>,
        ReadStorage<'a, Chain>,
        ReadStorage<'a, ShipStats>,
        Write<'a, World<f32>>,
        Write<'a, EventChannel<InsertEvent>>,
        Write<'a, BodiesMap>,
        Write<'a, EventChannel<Sound>>,
        ReadExpect<'a, PreloadedSounds>,
    );

    fn run(&mut self, data: Self::SystemData) {
        info!("asteroids: ai started");
        let (
            entities,
            isometries,
            mut velocities,
            physics,
            mut spins,
            mut shotguns,
            mut multy_lazers,
            mut cannons,
            mut rocket_guns,
            enemies,
            mut chargings,
            character_markers,
            ais,
            chains,
            ship_stats,
            mut world,
            mut insert_channel,
            bodies_map,
            mut sounds_channel,
            preloaded_sounds,
        ) = data;
        let (character_entity, character_position, _) =
            if let Some(value) = (&entities, &isometries, &character_markers).join().next() {
                value
            } else {
                return;
            };
        let character_position = character_position.0.translation.vector;
        for (entity, iso, vel, physics_component, spin, _enemy, ai) in (
            &entities,
            &isometries,
            &mut velocities,
            &physics,
            &mut spins,
            &enemies,
            &ais,
        )
            .join()
        {
            let isometry = iso.0;
            let position = isometry.translation.vector;
            let diff = character_position - position;
            let dir = Vector2::new(diff.x, diff.y).normalize();
            let pos = Point2::new(position.x, position.y);
            let ray = Ray::new(pos, dir);
            let enemy_collision_groups = get_collision_groups(&EntityType::Enemy);
            let nearby = get_min_dist(&mut world, ray, enemy_collision_groups);
            let mut character_noticed = false;
            if let Some(body) = nearby.1 {
                // body that we facing
                if bodies_map[&body] == character_entity {
                    character_noticed = true;
                }
            };
            let follow_area = if let Some(multy_lazer) = multy_lazers.get(entity) {
                multy_lazer.first_distance() * 0.95
            } else {
                SCREEN_AREA
            };
            for ai_type in ai.kinds.iter() {
                match ai_type {
                    AIType::Shoot => {
                        // Copy paste from top
                        let gun = cannons.get_mut(entity);
                        if let Some(gun) = gun {
                            if diff.norm() < SCREEN_AREA && gun.shoot() && character_noticed {
                                let bullets = gun.spawn_bullets(
                                    EntityType::Enemy,
                                    isometry,
                                    gun.bullet_speed,
                                    gun.bullets_damage,
                                    Vector2::new(vel.0.x, vel.0.y),
                                    entity,
                                );
                                insert_channel.iter_write(bullets.into_iter());
                                sounds_channel.single_write(Sound(
                                    preloaded_sounds.enemy_blaster,
                                    Point2::new(position.x, position.y),
                                ))
                            }
                        }
                        let shotgun = shotguns.get_mut(entity);
                        if let Some(shotgun) = shotgun {
                            if diff.norm() < SCREEN_AREA && shotgun.shoot() {
                                let bullets = shotgun.spawn_bullets(
                                    EntityType::Enemy,
                                    isometry,
                                    shotgun.bullet_speed,
                                    shotgun.bullets_damage,
                                    Vector2::new(vel.0.x, vel.0.y),
                                    entity,
                                );
                                insert_channel.iter_write(bullets.into_iter());
                                sounds_channel.single_write(Sound(
                                    preloaded_sounds.enemy_shotgun,
                                    Point2::new(position.x, position.y),
                                ))
                            }
                        }
                        if let Some(rocket_gun) = rocket_guns.get_mut(entity) {
                            if diff.norm() < SCREEN_AREA && rocket_gun.shoot() {
                                let bullets = rocket_gun.spawn_bullets(
                                    EntityType::Enemy,
                                    isometry,
                                    rocket_gun.bullet_speed,
                                    rocket_gun.bullets_damage,
                                    Vector2::new(vel.0.x, vel.0.y),
                                    entity,
                                );
                                insert_channel.iter_write(bullets.into_iter());
                                sounds_channel.single_write(Sound(
                                    preloaded_sounds.enemy_shotgun,
                                    Point2::new(position.x, position.y),
                                ))
                            }
                        }
                        if diff.norm() > follow_area {
                            if let Some(multy_lazer) = multy_lazers.get_mut(entity) {
                                multy_lazer.set_all(false);
                            }
                        } else {
                            if let Some(multy_lazer) = multy_lazers.get_mut(entity) {
                                multy_lazer.set_all(true);
                            }
                        }
                    }
                    AIType::Follow => {
                        let speed = ship_stats.get(entity).unwrap().thrust_force;
                        let mut is_chain = false;
                        if let Some(chain) = chains.get(entity) {
                            if let Some(iso) = isometries.get(chain.follow) {
                                is_chain = true;
                                let follow_vector = iso.0.translation.vector;
                                let follow_pos = Point2::new(follow_vector.x, follow_vector.y);
                                let diff = follow_pos - pos;
                                // if diff.norm() > 1.5f32 { // for not overlap
                                let dir = diff.normalize();
                                let ai_vel = speed * dir;
                                *vel = Velocity::new(ai_vel.x, ai_vel.y);
                                let body =
                                    world.rigid_body_mut(physics_component.body_handle).unwrap();
                                let mut velocity = *body.velocity();
                                *velocity.as_vector_mut() = Vector3::new(vel.0.x, vel.0.y, spin.0);
                                body.set_velocity(velocity);
                                // }
                            }
                        };
                        if !is_chain {
                            if diff.norm() > follow_area {
                                if character_noticed {
                                    let ai_vel = speed * dir;
                                    *vel = Velocity::new(ai_vel.x, ai_vel.y);
                                }
                            } else {
                                let vel_vec = DAMPING_FACTOR * vel.0;
                                *vel = Velocity::new(vel_vec.x, vel_vec.y);
                            }
                            let body = world.rigid_body_mut(physics_component.body_handle).unwrap();
                            let mut velocity = *body.velocity();
                            *velocity.as_vector_mut() = Vector3::new(vel.0.x, vel.0.y, spin.0);
                            body.set_velocity(velocity);
                        }
                    }
                    AIType::FollowRotate { spin: rot_spin } => {
                        let speed = ship_stats.get(entity).unwrap().thrust_force;
                        let mut is_chain = false;
                        if let Some(chain) = chains.get(entity) {
                            if let Some(iso) = isometries.get(chain.follow) {
                                is_chain = true;
                                let follow_vector = iso.0.translation.vector;
                                let follow_pos = Point2::new(follow_vector.x, follow_vector.y);
                                let diff = follow_pos - pos;
                                // if diff.norm() > 1.5f32 { // for not overlap
                                let dir = diff.normalize();
                                let ai_vel = speed * dir;
                                *vel = Velocity::new(ai_vel.x, ai_vel.y);
                                let body =
                                    world.rigid_body_mut(physics_component.body_handle).unwrap();
                                let mut velocity = *body.velocity();
                                *velocity.as_vector_mut() = Vector3::new(vel.0.x, vel.0.y, spin.0);
                                body.set_velocity(velocity);
                                // }
                            }
                        };
                        let rot_spin = rot_spin.unwrap();
                        if !is_chain {
                            if diff.norm() > rot_spin.abs() {
                                if character_noticed {
                                    let ai_vel = speed * dir;
                                    *vel = Velocity::new(ai_vel.x, ai_vel.y);
                                }
                            } else {
                                // let vel_vec = DAMPING_FACTOR * vel.0;
                                let ai_vel = speed * dir;
                                let tangent_vel =
                                    rot_spin / rot_spin.abs() * Vector2::new(-ai_vel.y, ai_vel.x);
                                let spiral = 0.3;
                                *vel = Velocity::new(
                                    tangent_vel.x + ai_vel.x * spiral,
                                    tangent_vel.y + ai_vel.y * spiral,
                                );
                            }
                            let body = world.rigid_body_mut(physics_component.body_handle).unwrap();
                            let mut velocity = *body.velocity();
                            *velocity.as_vector_mut() = Vector3::new(vel.0.x, vel.0.y, spin.0);
                            body.set_velocity(velocity);
                        }
                    }
                    AIType::Aim => {
                        let ship_torque = DT
                            * calculate_player_ship_spin_for_aim(
                                Vector2::new(character_position.x, character_position.y)
                                    - Vector2::new(position.x, position.y),
                                iso.rotation(),
                                spin.0,
                            );
                        spin.0 += ship_torque.max(-MAX_TORQUE).min(MAX_TORQUE);
                    }
                    AIType::Rotate(speed) => {
                        spin.0 = *speed;
                    }
                    AIType::Kamikadze => {
                        let speed = 0.1f32;
                        let diff = character_position - position;
                        let dir = speed * (diff).normalize();
                        *vel = Velocity::new(dir.x, dir.y);
                        let body = world.rigid_body_mut(physics_component.body_handle).unwrap();
                        let mut velocity = *body.velocity();
                        *velocity.as_vector_mut() = Vector3::new(vel.0.x, vel.0.y, spin.0);
                        body.set_velocity(velocity);
                    }
                    AIType::Charging(_) => {
                        let speed = 0.2f32;
                        let charging = chargings
                            .get_mut(entity)
                            .expect("no charging component while have charging AI");
                        if charging.shoot() {
                            let diff = character_position - position;
                            let dir = speed * (diff).normalize();
                            *vel = Velocity::new(dir.x, dir.y);
                            let body = world.rigid_body_mut(physics_component.body_handle).unwrap();
                            let mut velocity = *body.velocity();
                            *velocity.as_vector_mut() = Vector3::new(vel.0.x, vel.0.y, spin.0);
                            body.set_velocity(velocity);
                        }
                    }
                }
            }
            info!("asteroids: ai ended");
        }
    }
}
