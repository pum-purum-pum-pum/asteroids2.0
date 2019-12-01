use super::*;
use log::info;
use nphysics2d::algebra::Force2;
use nphysics2d::algebra::ForceType;

const ASTEROID_DAMAGE: usize = 140usize;

fn reflect_bullet(
    projectile: specs::Entity,
    physics_components: &ReadStorage<PhysicsComponent>,
    world: &mut Write<World<f32>>,
    reflections: &mut WriteStorage<Reflection>,
    normal: Vector2,
    lifetimes: &mut WriteStorage<Lifetime>,
) {
    let reflection = reflections.get_mut(projectile).unwrap();
    let physics_component = physics_components.get(projectile).unwrap();
    let body = world.rigid_body_mut(physics_component.body_handle).unwrap();
    let position = body.position();
    let mut velocity = *body.velocity();
    let vel = reflection.speed
        * reflect(velocity.linear, normal.normalize()).normalize();
    *velocity.as_vector_mut() = Vector3::new(vel.x, vel.y, 0.0);
    let standart = Vector2::new(0.0, -1.0);
    let alpha =
        Rotation2::rotation_between(&standart, &velocity.linear).angle();
    let position = Isometry2::new(
        Vector2::new(
            position.translation.vector.x,
            position.translation.vector.y,
        ),
        alpha,
    );
    let mut new_reflection = reflection.clone();
    new_reflection.times = Some(1);
    let lifetime = if let Some(times) = reflection.times {
        new_reflection.times = Some(times + 1);
        // mb tweak this later
        Duration::from_secs(0)
    } else {
        new_reflection.times = Some(1);
        reflection.lifetime
    };
    *lifetimes.get_mut(projectile).unwrap() = Lifetime::new(lifetime);
    *reflection = new_reflection;
    body.set_position(position);
    body.set_velocity(velocity);
}

fn damage_ship(
    is_character: bool,
    ship: specs::Entity,
    lifes: &mut WriteStorage<Lifes>,
    shields: &mut WriteStorage<Shield>,
    entities: &Entities,
    app_state: &mut Write<AppState>,
    progress: &mut Write<Progress>,
    macro_game: &mut WriteExpect<MacroGame>,
    insert_channel: &mut Write<EventChannel<InsertEvent>>,
    sounds_channel: &mut Write<EventChannel<Sound>>,
    preloaded_sounds: &ReadExpect<PreloadedSounds>,
    preloaded_images: &ReadExpect<PreloadedImages>,
    global_params: &mut WriteExpect<GlobalParams>,
    contact_pos: Point2,
    ship_pos: Point2,
    damage: usize,
    bullet: bool,
) {
    if is_character {
        if bullet {
            global_params.damaged(DAMAGED_RED);
            insert_channel.single_write(InsertEvent::Wobble(0.1f32));
        }
    }
    if bullet {
        bullet_contact(
            contact_pos,
            insert_channel,
            sounds_channel,
            preloaded_sounds,
            preloaded_images,
        );
    }
    if process_damage(
        lifes.get_mut(ship).unwrap(),
        shields.get_mut(ship),
        damage,
    ) {
        // ship is done... Explode it
        ship_explode(
            ship_pos,
            insert_channel,
            sounds_channel,
            preloaded_sounds,
        );
        if is_character {
            to_menu(app_state, progress, &mut macro_game.score_table);
        }
        entities.delete(ship).unwrap();
    }
}

#[derive(Default)]
pub struct CollisionSystem {
    colliding_pairs:
        Vec<(CollisionObjectHandle, CollisionObjectHandle, Vector2)>,
    colliding_start_events: Vec<(CollisionObjectHandle, CollisionObjectHandle)>,
}

impl<'a> System<'a> for CollisionSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Isometry>,
        ReadStorage<'a, PhysicsComponent>,
        ReadStorage<'a, AsteroidMarker>,
        ReadStorage<'a, CharacterMarker>,
        ReadStorage<'a, ShipMarker>,
        ReadStorage<'a, Projectile>,
        WriteStorage<'a, Reflection>,
        WriteStorage<'a, Lifes>,
        WriteStorage<'a, Shield>,
        WriteStorage<'a, Lifetime>,
        ReadStorage<'a, Damage>,
        WriteStorage<'a, Polygon>,
        ReadStorage<'a, Size>,
        WriteStorage<'a, DamageFlash>,
        Write<'a, World<f32>>,
        Read<'a, BodiesMap>,
        Write<'a, EventChannel<InsertEvent>>,
        Write<'a, EventChannel<Sound>>,
        ReadExpect<'a, PreloadedSounds>,
        ReadExpect<'a, PreloadedImages>,
        Write<'a, Progress>,
        Write<'a, AppState>,
        WriteExpect<'a, MacroGame>,
        WriteExpect<'a, GlobalParams>,
        ReadExpect<'a, Arc<Mutex<EventChannel<InsertEvent>>>>,
    );

    fn run(&mut self, data: Self::SystemData) {
        info!("asteroids: collision started");
        let (
            entities,
            isometries,
            physics_components,
            asteroids,
            character_markers,
            ships,
            projectiles,
            mut reflections,
            mut lifes,
            mut shields,
            mut lifetimes,
            damages,
            polygons,
            sizes,
            mut flashes,
            mut world,
            bodies_map,
            mut insert_channel,
            mut sounds_channel,
            preloaded_sounds,
            preloaded_images,
            mut progress,
            mut app_state,
            mut macro_game,
            mut global_params,
            asteroids_channel,
        ) = data;
        self.colliding_pairs.clear();
        self.colliding_start_events.clear();
        for event in world.contact_events() {
            match event {
                &ncollide2d::events::ContactEvent::Started(
                    collision_handle1,
                    collision_handle2,
                ) => {
                    self.colliding_start_events
                        .push((collision_handle1, collision_handle2));
                }
                _ => (),
            }
        }
        for (h1, h2) in self.colliding_start_events.iter() {
            if let Some((_h1, _h2, _, manifold)) =
                world.collider_world_mut().contact_pair(*h1, *h2, true)
            {
                if let Some(tracked_contact) = manifold.deepest_contact() {
                    let contact_normal = tracked_contact.contact.normal;
                    self.colliding_pairs.push((*h1, *h2, *contact_normal))
                }
            }
        }

        for (handle1, handle2, normal) in self.colliding_pairs.iter() {
            let (body_handle1, body_handle2) = {
                // get body handles
                let collider_world = world.collider_world_mut();
                (
                    collider_world.collider_mut(*handle1).unwrap().body(),
                    collider_world.collider_mut(*handle2).unwrap().body(),
                )
            };
            let mut entity1 = bodies_map[&body_handle1];
            let mut entity2 = bodies_map[&body_handle2];
            if asteroids.get(entity2).is_some() {
                swap(&mut entity1, &mut entity2);
            }
            if asteroids.get(entity1).is_some() {
                let asteroid = entity1;
                let mut asteroid_explosion = false;
                let mut bullet_position = None;
                // 
                if asteroids.get(entity2).is_some() {
                    // let asteroid1 = entity1;
                    // let asteroid2 = entity2;
                    // let (pos1, pos2) = {
                    //     let iso1 = isometries.get(entity1).unwrap();
                    //     let iso2 = isometries.get(entity2).unwrap();
                    //     let (pos1, pos2) = (iso1.0.translation.vector, iso2.0.translation.vector);
                    //     (Vector2::new(pos1.x, pos1.y), Vector2::new(pos2.x, pos2.y))
                    // };
                    // let (poly1, poly2) = {
                    //     (polygons.get(asteroid1).unwrap(), polygons.get(asteroid2).unwrap())
                    // };
                    // let phys1 = physics_components.get(asteroid1).unwrap();
                    // let phys2 = physics_components.get(asteroid2).unwrap();
                    // let rigid_body1 = world
                    //     .rigid_body_mut(phys1.body_handle).unwrap();
                    // let dir1 = pos1 - pos2;
                    // let force1 = Force2::new(poly1.min_r * 0.3 * (dir1).normalize(), 0.0);
                    // rigid_body1.apply_force(0, &force1, ForceType::Force, true);
                    // let rigid_body2 = world
                    //     .rigid_body_mut(phys2.body_handle).unwrap();
                    // let dir2 = pos2 - pos1;
                    // let force2 = Force2::new(poly2.min_r * 0.3 * (dir2).normalize(), 0.0);
                    // rigid_body2.apply_force(0, &force2, ForceType::Force, true);
                }
                if projectiles.get(entity2).is_some() {
                    let proj_pos =
                        isometries.get(entity2).unwrap().0.translation.vector;
                    let proj_pos2d = Point2::new(proj_pos.x, proj_pos.y);
                    bullet_position = Some(proj_pos2d);
                    bullet_contact(
                        proj_pos2d,
                        &mut insert_channel,
                        &mut sounds_channel,
                        &preloaded_sounds,
                        &preloaded_images,
                    );
                    let projectile = entity2;
                    let projectile_damage = damages.get(projectile).unwrap().0;
                    if projectile_damage != 0 {
                        if reflections.get(projectile).is_some() {
                            reflect_bullet(
                                projectile,
                                &physics_components,
                                &mut world,
                                &mut reflections,
                                *normal,
                                &mut lifetimes,
                            );
                        } else {
                            entities.delete(projectile).unwrap();
                        }
                    }
                    let lifes = lifes.get_mut(asteroid).unwrap();
                    if lifes.0 > projectile_damage {
                        lifes.0 -= projectile_damage
                    } else {
                        if lifes.0 > 0 {
                            lifes.0 = 0;
                            asteroid_explosion = true
                        }
                    }
                };
                if ships.get(entity2).is_some() {
                    let ship = entity2;
                    let isometry = isometries.get(ship).unwrap().0;
                    let position = isometry.translation.vector;
                    // asteroid_explosion = true;
                    let effect = InsertEvent::Explosion {
                        position: Point2::new(position.x, position.y),
                        num: 3usize,
                        lifetime: Duration::from_secs(EXPLOSION_LIFETIME_SECS),
                        with_animation: None,
                    };
                    insert_channel.single_write(effect);
                    if character_markers.get(ship).is_some() {
                        sounds_channel.single_write(Sound(
                            preloaded_sounds.collision,
                            Point2::new(position.x, position.y),
                        ));
                    }
                    let is_character = character_markers.get(ship).is_some();
                    if is_character {
                        damage_ship(
                            is_character,
                            ship,
                            &mut lifes,
                            &mut shields,
                            &entities,
                            &mut app_state,
                            &mut progress,
                            &mut macro_game,
                            &mut insert_channel,
                            &mut sounds_channel,
                            &preloaded_sounds,
                            &preloaded_images,
                            &mut global_params,
                            Point2::new(position.x, position.y),
                            Point2::new(position.x, position.y),
                            (ASTEROID_DAMAGE as f32
                                * sizes.get(asteroid).unwrap().0)
                                as usize,
                            false,
                        );
                        bullet_contact(
                            Point2::new(position.x, position.y),
                            &mut insert_channel,
                            &mut sounds_channel,
                            &preloaded_sounds,
                            &preloaded_images,
                        );
                        global_params.damaged(2.0 * DAMAGED_RED);
                    }
                }
                if asteroid_explosion {
                    insert_channel
                        .single_write(InsertEvent::Wobble(EXPLOSION_WOBBLE));
                    let isometry = isometries.get(asteroid).unwrap().0;
                    let position = isometry.translation.vector;
                    let polygon = polygons.get(asteroid).unwrap();
                    asteroid_explode(
                        Point2::new(position.x, position.y),
                        &mut insert_channel,
                        &mut sounds_channel,
                        &preloaded_sounds,
                        &preloaded_images,
                        polygon.max_r,
                    );
                    let iso = isometries.get(asteroid).unwrap().0;
                    let poly = polygons.get(asteroid).unwrap().clone();
                    let channel_arc = (*asteroids_channel).clone();
                    thread::spawn(move || {
                        spawn_asteroids(
                            iso,
                            poly,
                            channel_arc,
                            bullet_position,
                        );
                    });
                    entities.delete(asteroid).unwrap();
                }
            }
            if ships.get(entity2).is_some() {
                swap(&mut entity1, &mut entity2);
            }
            if ships.get(entity1).is_some()
                && projectiles.get(entity2).is_some()
            {
                let ship = entity1;
                let projectile = entity2;
                let projectile_damage = damages.get(projectile).unwrap().0;
                let isometry = isometries.get(ship).unwrap().0;
                let projectile_pos =
                    isometries.get(projectile).unwrap().0.translation.vector;
                let projectile_pos =
                    Point2::new(projectile_pos.x, projectile_pos.y);
                let position = isometry.translation.vector;
                if let Some(flash) = flashes.get_mut(ship) {
                    flash.0 = (flash.0 + 0.5).min(1f32);
                }
                damage_ship(
                    character_markers.get(ship).is_some(),
                    ship,
                    &mut lifes,
                    &mut shields,
                    &entities,
                    &mut app_state,
                    &mut progress,
                    &mut macro_game,
                    &mut insert_channel,
                    &mut sounds_channel,
                    &preloaded_sounds,
                    &preloaded_images,
                    &mut global_params,
                    projectile_pos,
                    Point2::new(position.x, position.y),
                    projectile_damage,
                    true,
                );
                // Kludge
                if projectile_damage != 0 {
                    if reflections.get(projectile).is_some() {
                        reflect_bullet(
                            projectile,
                            &physics_components,
                            &mut world,
                            &mut reflections,
                            *normal,
                            &mut lifetimes,
                        );
                    } else {
                        entities.delete(projectile).unwrap();
                    }
                }
            }
            if ships.get(entity1).is_some() && ships.get(entity2).is_some() {
                let mut ship1 = entity1;
                let mut ship2 = entity2;
                let isometry = isometries.get(ship1).unwrap().0;
                let position = isometry.translation.vector;
                if character_markers.get(ship2).is_some() {
                    swap(&mut ship1, &mut ship2)
                }
                if character_markers.get(ship1).is_some() {
                    let character_ship = ship1;
                    let other_ship = ship2;
                    // entities.delete(other_ship).unwrap();
                    sounds_channel.single_write(Sound(
                        preloaded_sounds.collision,
                        Point2::new(0f32, 0f32),
                    ));
                    if process_damage(
                        lifes.get_mut(other_ship).unwrap(),
                        shields.get_mut(other_ship),
                        damages.get(character_ship).unwrap().0,
                    ) {
                        ship_explode(
                            Point2::new(position.x, position.y),
                            &mut insert_channel,
                            &mut sounds_channel,
                            &preloaded_sounds,
                        );
                        entities.delete(other_ship).unwrap();
                    }
                    global_params.damaged(DAMAGED_RED);
                    if process_damage(
                        lifes.get_mut(character_ship).unwrap(),
                        shields.get_mut(character_ship),
                        damages.get(other_ship).unwrap().0,
                    ) {
                        to_menu(
                            &mut app_state,
                            &mut progress,
                            &mut macro_game.score_table,
                        );
                        // delete character
                        entities.delete(character_ship).unwrap();
                    }
                }
            }
        }
        info!("asteroids: collision ended");
    }
}
