
use std::mem::swap;

use crate::types::{*};
use rand::prelude::*;
use sdl2::keyboard::Keycode;
use sdl2::TimerSubsystem;

use ncollide2d::shape::ShapeHandle;
use ncollide2d::world::CollisionGroups;
use ncollide2d::world::CollisionObjectHandle;
use ncollide2d::query::{Ray};
use nphysics2d::object::{Body, BodyStatus, BodyHandle};
use nphysics2d::world::World;
use shrev::EventChannel;
use specs::prelude::*;
use specs::Join;

use crate::components::*;
use crate::geometry::{generate_convex_polygon, Polygon, TriangulateFromCenter, EPS};
use crate::gfx::{GeometryData, Engine, ParticlesData, Explosion};
use crate::physics::CollisionId;
use crate::sound::{PreloadedSounds, SoundData, MusicData};
use crate::gui::{Primitive, PrimitiveKind, Button, IngameUI, Text};

mod rendering;
pub use rendering::*;

const DAMPING_FACTOR: f32 = 0.98f32;
const VELOCITY_MAX: f32 = 1f32;
const MAX_TORQUE: f32 = 10f32;

const SCREEN_AREA: f32 = 10f32;
// it's a kludge -- TODO redo with camera and screen sizes

// we will spwan new objects in ACTIVE_AREA but not in PLAYER_AREA
const PLAYER_AREA: f32 = 20f32;
const ACTIVE_AREA: f32 = 40f32;
// the same for NEBULAS
const NEBULA_PLAYER_AREA: f32 = 90f32;
const NEBULA_ACTIVE_AREA: f32 = 110f32;
const NEBULA_MIN_NUMBER: usize = 20;

const ASTEROIDS_MIN_NUMBER: usize = 25;
const ASTEROID_MAX_RADIUS: f32 = 4.2f32;
const ASTEROID_MIN_RADIUS: f32 = 0.5;
const ASTEROID_INERTIA: f32 = 2f32;

const AI_COLLISION_DISTANCE: f32 = 3.5f32;
const EXPLOSION_WOBBLE: f32 = 0.4;

const MAGNETO_RADIUS: f32 = 4f32;
const COLLECT_RADIUS: f32 = 0.2;
pub const DT: f32 =  1f32 / 60f32;

pub fn initial_asteroid_velocity() -> Velocity2 {
    let mut rng = thread_rng();
    let rotation = rng.gen_range(-1E-1, 1E-1);
    let linear_velocity = Vector2::new(
        rng.gen_range(-1E-1, 1E-1), 
        rng.gen_range(-1E-1, 1E-1)
    );
    Velocity2::new(linear_velocity, rotation)
}

pub fn spawn_position(char_pos: Point2, forbidden: f32, active: f32) -> Point2 {
    assert!(forbidden < active);
    let mut rng = thread_rng();
    loop {
        let x = rng.gen_range(-active, active);
        let y = rng.gen_range(-active, active);
        if x.abs() >= forbidden || y.abs() >= forbidden {
            return Point2::new(char_pos.x + x, char_pos.y + y);
        }
    }
}

pub fn spawn_in_rectangle(min_w: f32, max_w: f32, min_h: f32, max_h: f32) -> Point2 {
    let mut rng = thread_rng();
    let x = rng.gen_range(min_w, max_w);
    let y = rng.gen_range(min_h, max_h);
    Point2::new(x, y)
}

pub fn is_active(character_position: Point2, point: Point2, active_area: f32) -> bool {
    (point.x - character_position.x).abs() < active_area
        && (point.y - character_position.y).abs() < active_area
}

fn iso2_iso3(iso2: &Isometry2) -> Isometry3 {
    Isometry3::new(
        Vector3::new(iso2.translation.vector.x, iso2.translation.vector.y, 0f32),
        Vector3::new(0f32, 0f32, iso2.rotation.angle()),
    )
}

/// Calculate the shortest distance between two angles expressed in radians.
///
/// Based on https://gist.github.com/shaunlebron/8832585
pub fn angle_shortest_dist(a0: f32, a1: f32) -> f32 {
    let max = std::f32::consts::PI * 2.0;
    let da = (a1 - a0) % max;
    2.0 * da % max - da
}

/// Calculate spin for rotating the player's ship towards a given direction.
///
/// Inspired by proportional-derivative controllers, but approximated with just the current spin
/// instead of error derivatives. Uses arbitrary constants tuned for player control.
pub fn calculate_player_ship_spin_for_aim(aim: Vector2, rotation: f32, speed: f32) -> f32 {
    let target_rot = if aim.x == 0.0 && aim.y == 0.0 {
        rotation
    } else {
        -(-aim.x).atan2(-aim.y)
    };

    let angle_diff = angle_shortest_dist(rotation, target_rot);

    (angle_diff * 10.0 - speed * 55.0)
}

fn get_collision_groups(kind: &EntityType) -> CollisionGroups {
    match kind {
        EntityType::Player => {
            let mut player_bullet_collision_groups = CollisionGroups::new();
            player_bullet_collision_groups
                .set_membership(&[CollisionId::PlayerBullet as usize]);
            player_bullet_collision_groups.set_whitelist(&[
                CollisionId::Asteroid as usize,
                CollisionId::EnemyShip as usize,
            ]);
            player_bullet_collision_groups
                .set_blacklist(&[CollisionId::PlayerShip as usize]);
            player_bullet_collision_groups
        }
        EntityType::Enemy => {
            let mut enemy_bullet_collision_groups = CollisionGroups::new();
            enemy_bullet_collision_groups
                .set_membership(&[CollisionId::EnemyBullet as usize]);
            enemy_bullet_collision_groups.set_whitelist(&[
                CollisionId::Asteroid as usize,
                CollisionId::PlayerShip as usize,
            ]);
            enemy_bullet_collision_groups
                .set_blacklist(&[CollisionId::EnemyShip as usize]);
            enemy_bullet_collision_groups
        }
    }
}

#[derive(Default)]
pub struct PhysicsSystem;

impl<'a> System<'a> for PhysicsSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Isometry>,
        WriteStorage<'a, Velocity>,
        ReadStorage<'a, PhysicsComponent>,
        ReadStorage<'a, CharacterMarker>,
        ReadStorage<'a, EnemyMarker>,
        ReadStorage<'a, Charge>,
        Write<'a, World<f32>>,
        WriteExpect<'a, NebulaGrid>,
        WriteExpect<'a, PlanetGrid>,
        Read<'a, AppState>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            mut isometries, 
            mut velocities, 
            physics, 
            character_markers,
            chargings,
            enemies,
            mut world,
            mut nebula_grid,
            mut planet_grid,
            app_state
        ) = data;
        let (character_position, character_prev_position) = {
            let (character, isometry, _) = (&entities, &isometries, &character_markers).join().next().unwrap();
            let body = world
                .rigid_body(
                    physics
                        .get(character).unwrap()
                        .body_handle
                ).unwrap();
            (*body.position(), *isometry)
        };
        {   // Reactive enemies O(n^2)
            use nphysics2d::algebra::ForceType;
            use nphysics2d::algebra::Force2;
            let mut enemies_entities = vec![];
            for (entity, _phys, _enemy) in (&entities, &physics, &enemies).join() {
                enemies_entities.push(entity);
            }
            for e1 in enemies_entities.iter() {
                for e2 in enemies_entities.iter() {
                    if e1 == e2 {
                        break
                    }
                    if chargings.get(*e1).is_some() || chargings.get(*e2).is_some() {
                        continue
                    }
                    let phys1 = physics.get(*e1).unwrap();
                    let phys2 = physics.get(*e2).unwrap();
                    let (force1, force2, distance) = {
                        let body1 = world.rigid_body(phys1.body_handle).unwrap();
                        let body2 = world.rigid_body(phys2.body_handle).unwrap();
                        let position1 = body1.position().translation.vector;
                        let position2 = body2.position().translation.vector;
                        let distance = (position1 - position2).norm();
                        let center = (position1 + position2) / 2.0;
                        (
                            Force2::new(0.004 * (position1 - center).normalize(), 0.0), 
                            Force2::new(0.004 * (position2 - center).normalize(), 0.0),
                            distance
                        )
                    };
                    if distance < 5f32 {
                        world.rigid_body_mut(phys1.body_handle).unwrap()
                            .apply_force(0, &force1, ForceType::Force, true);
                        world.rigid_body_mut(phys2.body_handle).unwrap()
                            .apply_force(0, &force2, ForceType::Force, true);
                    }
                }
            }
        }
        let char_vec = character_position.translation.vector;
        let prev_vec = character_prev_position.0.translation.vector;
        let diff = Vector3::new(char_vec.x, char_vec.y, 0f32)  - Vector3::new(prev_vec.x, prev_vec.y, 0f32);
        for (isometry, ()) in (&mut isometries, !&physics).join() {
            isometry.0.translation.vector -= diff;
        }
        nebula_grid.grid.shift(-diff.x, -diff.y);
        planet_grid.grid.shift(-diff.x, -diff.y);
        for (isometry, velocity, physics_component) in
            (&mut isometries, &mut velocities, &physics).join()
        {
            let body = world.rigid_body_mut(physics_component.body_handle).unwrap();
            let mut physics_isometry = *body.position();
            // MOVE THE WORLD, NOT ENTITIES
            physics_isometry.translation.vector -= character_position.translation.vector;
            body.set_position(physics_isometry);
            let physics_velocity = body.velocity().as_vector();
            let physics_velocity = Vector2::new(physics_velocity.x, physics_velocity.y);
            isometry.0 = iso2_iso3(&physics_isometry);
            velocity.0 = physics_velocity;
        }
        match *app_state {
            AppState::Play(PlayState::Upgrade) => (),
            _ => {
                world.step();
            }
        }
    }
}

pub struct SoundSystem {
    reader: ReaderId<Sound>,
}

impl SoundSystem {
    pub fn new(reader: ReaderId<Sound>) -> Self {
        SoundSystem { reader: reader }
    }
}

impl<'a> System<'a> for SoundSystem {
    type SystemData = (
        ReadStorage<'a, ThreadPin<SoundData>>,
        ReadStorage<'a, CharacterMarker>,
        ReadStorage<'a, Lazer>,
        WriteExpect<'a, ThreadPin<TimerSubsystem>>,
        ReadExpect<'a, PreloadedSounds>,
        Write<'a, EventChannel<Sound>>,
        Write<'a, LoopSound>,
        ReadExpect<'a, ThreadPin<MusicData<'static>>>,
        Write<'a, Music>,
        Read<'a, AppState>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            sounds, 
            character_markers,
            lazers,
            _timer, 
            preloaded_sounds,
            sounds_channel,
            mut loop_sound,
            music_data,
            mut music,
            app_state
        ) = data;
        for s in sounds_channel.read(&mut self.reader) {
            sdl2::mixer::Channel::all().play(
                &sounds.get(s.0).unwrap().0, 
                0
            ).unwrap();
        }
        for (lazer, _character) in (&lazers, &character_markers).join() {
            if lazer.active {
                if loop_sound.player_lazer_channel.is_none() {
                    let channel = sdl2::mixer::Channel::all().play(
                        &sounds.get(preloaded_sounds.lazer).unwrap().0,
                        -1
                    ).unwrap();
                    music.menu_play = false; // hacky
                    loop_sound.player_lazer_channel = Some(channel);
                }
            } else {
                if let Some(lazer) = loop_sound.player_lazer_channel {
                    lazer.halt();
                    loop_sound.player_lazer_channel = None;
                }
            }
        }
        match *app_state {
            AppState::Play(_) => {
                if music.current_battle.is_none() {
                    let mut rng = thread_rng();
                    let music_id = rng.gen_range(0, music_data.battle_music.len());
                    music_data.battle_music[music_id].play(-1).unwrap();
                    music.current_battle = Some(music_id);
                }
            }
            AppState::Menu => {
                loop_sound.player_lazer_channel = None; // hacky
                if let Some(_music_id) = music.current_battle {
                    sdl2::mixer::Music::halt();
                    music.current_battle = None;
                }
                if !music.menu_play {
                    music_data.menu_music.play(-1).unwrap();
                    music.menu_play = true;
                }
            }
        }
    }
}

/// here we update isometry, velocity
pub struct KinematicSystem;

impl<'a> System<'a> for KinematicSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Isometry>,
        WriteStorage<'a, Velocity>,
        ReadStorage<'a, PhysicsComponent>,
        ReadStorage<'a, Spin>,
        ReadStorage<'a, AttachPosition>,
        ReadStorage<'a, CharacterMarker>,
        ReadStorage<'a, AsteroidMarker>,
        ReadStorage<'a, ShipMarker>,
        ReadStorage<'a, Projectile>,
        ReadStorage<'a, ShipStats>,
        Write<'a, World<f32>>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            mut isometries,
            mut velocities,
            physics,
            spins,
            attach_positions,
            character_markers,
            asteroid_markers,
            ship_markers,
            projectiles,
            ships_stats,
            mut world,
        ) = data;
        for (physics_component, _, _) in (&physics, !&asteroid_markers, !&projectiles).join() {
            let body = world.rigid_body_mut(physics_component.body_handle).unwrap();
            let mut velocity = *body.velocity();
            *velocity.as_vector_mut() *= DAMPING_FACTOR;
            body.set_velocity(velocity);
            body.activate();
        }
        // activate asteroid bodyes
        for (physics_component, _asteroid) in (&physics, &asteroid_markers).join() {
            let body = world.rigid_body_mut(physics_component.body_handle).unwrap();
            body.activate();
        }
        for (entity, ship_stats, _isometry, _velocity, physics_component, spin, _ship) in (
            &entities,
            &ships_stats,
            &mut isometries,
            &mut velocities,
            &physics,
            &spins,
            &ship_markers,
        )
            .join()
        {
            let body = world.rigid_body_mut(physics_component.body_handle).unwrap();
            if let Some(_) = character_markers.get(entity) {
                body.set_angular_velocity(ship_stats.torque * spin.0);
            } else {
                body.set_angular_velocity(spin.0);
            }
        }
        let mut attach_pairs = vec![];
        for (entity, _, attach) in (&entities, &mut isometries, &attach_positions).join() {
            attach_pairs.push((entity, attach.0));
        }
        for (entity, attach) in attach_pairs.iter() {
            // let physics_component = physics.get(*attach).unwrap();
            // let iso2 = world.rigid_body(physics_component.body_handle).position();
            match  isometries.get(*attach) {
                Some(isometry) => {
                    let iso = isometry;
                    isometries.get_mut(*entity).unwrap().0 = iso.0;
                }
                None => {
                    entities.delete(*entity).unwrap();
                }
            }
        }
    }
}


pub fn spawn_asteroids<'a>(
    isometry: Isometry3, 
    polygon: &Polygon, 
    insert_channel: &mut Write<'a, EventChannel<InsertEvent>>,
) {
    let position = isometry.translation.vector;
    let new_polygons = polygon.deconstruct();
    if new_polygons.len() != 1 {
        for poly in new_polygons.iter() {
            let asteroid_shape = Geometry::Polygon(poly.clone());
            let mut rng = thread_rng();
            let insert_event = InsertEvent::Asteroid {
                iso: Point3::new(position.x, position.y, isometry.rotation.angle()),
                velocity: initial_asteroid_velocity(),
                polygon: poly.clone(),
                light_shape: asteroid_shape,
                spin: rng.gen_range(-1E-2, 1E-2),
            };
            insert_channel.single_write(insert_event);
        }
    } else {
        // spawn coins and stuff
        insert_channel.single_write(InsertEvent::Coin{
            value: 1,
            position: Point2::new(position.x, position.y)
        });
    }
}

pub struct ControlSystem {
    reader: ReaderId<Keycode>,
}

impl ControlSystem {
    pub fn new(reader: ReaderId<Keycode>) -> Self {
        ControlSystem { reader: reader }
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
            WriteStorage<'a, Blaster>,
            WriteStorage<'a, ShotGun>,
            WriteStorage<'a, MultyLazer>,
            WriteStorage<'a, Lazer>,
            WriteStorage<'a, Lifes>,
            WriteStorage<'a, Shield>,
            WriteStorage<'a, Polygon>,
            ReadStorage<'a, CharacterMarker>,
            ReadStorage<'a, AsteroidMarker>,
            WriteStorage<'a, ShipStats>,
        ),
        ReadExpect<'a, PreloadedImages>,
        Read<'a, EventChannel<Keycode>>,
        Read<'a, Mouse>,
        Write<'a, EventChannel<Sound>>,
        ReadExpect<'a, PreloadedSounds>,
        Write<'a, World<f32>>,
        Write<'a, BodiesMap>,
        Write<'a, EventChannel<InsertEvent>>,
        Write<'a, Progress>,
        Write<'a, AppState>,
        WriteExpect<'a, Canvas>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            (
                entities,
                isometries,
                mut velocities,
                physics,
                mut spins,
                mut blasters,
                mut shotguns,
                mut multiple_lazers,
                mut lazers,
                mut lifes,
                mut shields,
                polygons,
                character_markers,
                asteroid_markers,
                mut ships_stats,
            ),
            preloaded_images,
            keys_channel,
            mouse_state,
            mut sounds_channel,
            preloaded_sounds,
            mut world,
            bodies_map,
            mut insert_channel,
            mut progress,
            mut app_state,
            mut canvas,
        ) = data;
        let (ship_stats, _) = (&mut ships_stats, &character_markers).join().next().unwrap();
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
            let (_character_isometry, mut character_velocity) = {
                let character_body = world
                    .rigid_body(physics.get(character).unwrap().body_handle)
                    .unwrap();
                (*character_body.position(), *character_body.velocity())
            };
            if let Some(lazer) = lazers.get_mut(character) {
                if mouse_state.left {
                    lazer.active = true;
                } else {
                    lazer.active = false;
                }
            }

            let mut process_lazer = |
                physics_component: &PhysicsComponent,
                lazer: &mut Lazer,
                world: &mut Write<World<f32>>,
                bodies_map: & Write<BodiesMap>,
                is_character: bool,
                rotation,
            | {
                let body = world
                    .rigid_body(physics_component.body_handle)
                    .unwrap();
                let isometry = body.position();
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
                            let mut with_animation = false;
                            if process_damage(
                                lifes.get_mut(*target_entity).unwrap(),
                                shields.get_mut(*target_entity),
                                lazer.damage
                            ) {
                                if asteroid_markers.get(*target_entity).is_some() {
                                    let asteroid = *target_entity;
                                    spawn_asteroids(
                                        isometries.get(asteroid).unwrap().0, 
                                        polygons.get(asteroid).unwrap(), 
                                        &mut insert_channel,
                                    );
                                } else {
                                    let target_position = isometries
                                        .get(*target_entity).unwrap().0.translation.vector;
                                    insert_channel.single_write(
                                        InsertEvent::Exp{
                                            value: 50, 
                                            position: Point2::new(target_position.x, target_position.y)
                                        }
                                    );
                                }
                                explosion_size = 20;
                                with_animation = true;
                                sounds_channel.single_write(Sound(preloaded_sounds.explosion));
                                insert_channel.single_write(InsertEvent::Wobble(EXPLOSION_WOBBLE));
                                entities.delete(*target_entity).unwrap();
                            }
                            let effect_position = position + dir * min_d;
                            let effect = InsertEvent::Explosion {
                                position: Point2::new(effect_position.x, effect_position.y),
                                num: explosion_size,
                                lifetime: 50usize,
                                with_animation
                            };
                            insert_channel.single_write(effect);
                        }
                    }
                } else {
                    lazer.current_distance = lazer.distance
                }
            };

            for (entity, physics_component, multiple_lazers) in (&entities, &physics, &mut multiple_lazers).join() {
                for (i, lazer) in multiple_lazers.lazers.iter_mut().enumerate() {
                    let rotation = Rotation2::new(i as f32 * std::f32::consts::PI / 2.0);
                    if !lazer.active {
                        continue
                    }
                    process_lazer(
                        physics_component,
                        lazer,
                        &mut world,
                        &bodies_map,
                        character_markers.get(entity).is_some(),
                        rotation
                    )
                }
            }
            let zero_rotation = Rotation2::new(0.0);
            for (entity, physics_component, lazer) in (&entities, &physics, &mut lazers).join() {
                if !lazer.active {
                    continue
                }
                process_lazer(
                    physics_component,
                    lazer,
                    &mut world,
                    &bodies_map,
                    character_markers.get(entity).is_some(),
                    zero_rotation
                )
            }

            if mouse_state.left {
                if let Some(blaster) = blasters.get_mut(character) {
                    if blaster.shoot() {
                        let bullets = blaster.spawn_bullets(
                            EntityType::Player,
                            isometries.get(character).unwrap().0,
                            blaster.bullet_speed,
                            blaster.bullets_damage,
                            velocities.get(character).unwrap().0,
                            character
                        );
                        sounds_channel.single_write(Sound(preloaded_sounds.shot));
                        insert_channel.iter_write(bullets.into_iter());
                    }
                }
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
                        sounds_channel.single_write(Sound(preloaded_sounds.shot));
                        insert_channel.iter_write(bullets.into_iter());
                    }
                }
            }
            for key in keys_channel.read(&mut self.reader) {
                match key {
                    Keycode::W => {
                        let thrust = ship_stats.thrust_force * Vector3::new(0.0, -1.0, 0.0);
                        *character_velocity.as_vector_mut() += thrust;
                    }
                    Keycode::S => {
                        let thrust = ship_stats.thrust_force * Vector3::new(0.0, 1.0, 0.0);
                        *character_velocity.as_vector_mut() += thrust;
                    }
                    Keycode::A => {
                        let thrust = ship_stats.thrust_force * Vector3::new(-1.0, 0.0, 0.0);
                        *character_velocity.as_vector_mut() += thrust;
                    }
                    Keycode::D => {
                        let thrust = ship_stats.thrust_force * Vector3::new(1.0, 0.0, 0.0);
                        *character_velocity.as_vector_mut() += thrust;
                    }
                    Keycode::Space => {
                        *app_state = AppState::Play(PlayState::Upgrade)
                    }
                    Keycode::LeftBracket => {
                        canvas.z_far -= 0.5;
                    }
                    Keycode::RightBracket => {
                        canvas.z_far += 0.5;
                    }
                    _ => ()
                    
                }
            }
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
    }
}

// thread local system
pub struct InsertSystem {
    reader: ReaderId<InsertEvent>,
}

impl InsertSystem {
    pub fn new(reader: ReaderId<InsertEvent>) -> Self {
        InsertSystem { reader: reader }
    }
}

impl<'a> System<'a> for InsertSystem {
    type SystemData = (
        (
            Entities<'a>,
            WriteStorage<'a, PhysicsComponent>,
            WriteStorage<'a, Geometry>,
            WriteStorage<'a, Isometry>,
            WriteStorage<'a, Velocity>,
            WriteStorage<'a, Spin>,
            WriteStorage<'a, Blaster>,
            WriteStorage<'a, MultyLazer>,
            WriteStorage<'a, Lazer>,
            WriteStorage<'a, ShotGun>,
            WriteStorage<'a, Cannon>,
            WriteStorage<'a, Damage>,
            WriteStorage<'a, Lifes>,
            WriteStorage<'a, Shield>,
            WriteStorage<'a, Lifetime>,
            WriteStorage<'a, AI>,
            WriteStorage<'a, Charge>,
            WriteStorage<'a, AsteroidMarker>,
            WriteStorage<'a, EnemyMarker>,
            WriteStorage<'a, ShipMarker>,
            WriteStorage<'a, Image>,
            WriteStorage<'a, Size>,
            WriteStorage<'a, Polygon>,
            WriteStorage<'a, Projectile>,
            WriteStorage<'a, Blast>,
        ),
        WriteStorage<'a, NebulaMarker>,
        WriteStorage<'a, PlanetMarker>,
        WriteStorage<'a, AttachPosition>,
        WriteStorage<'a, Animation>,
        WriteStorage<'a, Coin>,
        WriteStorage<'a, Exp>,
        WriteStorage<'a, LightMarker>,
        WriteStorage<'a, CharacterMarker>,
        WriteStorage<'a, ThreadPin<ParticlesData>>,
        WriteStorage<'a, ShipStats>,
        ReadExpect<'a, ThreadPin<red::GL>>,
        Write<'a, Stat>,
        WriteExpect<'a, PreloadedImages>,
        Write<'a, World<f32>>,
        Write<'a, BodiesMap>,
        Write<'a, Progress>,
        Write<'a, EventChannel<InsertEvent>>,
        WriteExpect<'a, Canvas>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            (
                entities,
                mut physics,
                mut geometries,
                mut isometries,
                mut velocities,
                mut spins,
                mut blasters,
                mut multy_lazers,
                mut lazers,
                mut shotguns,
                mut cannons,
                mut damages,
                mut lifes,
                mut shields,
                mut lifetimes,
                mut ais,
                mut chargings,
                mut asteroid_markers,
                mut enemies,
                mut ships,
                mut images,
                mut sizes,
                mut polygons,
                mut projectiles,
                mut blasts,
            ),
            mut nebulas,
            mut planets,
            mut attach_positions,
            mut animations,
            mut coins,
            mut exps,
            mut lights,
            mut character_markers,
            mut particles_datas,
            mut ships_stats,
            gl,
            _stat,
            preloaded_images,
            mut world,
            mut bodies_map,
            mut progress,
            mut insert_channel,
            mut canvas
        ) = data;
        let mut reinsert = vec!();
        for insert in insert_channel.read(&mut self.reader) {
            match insert {
                InsertEvent::Character {
                    gun_kind,
                    ship_stats
                } => {
                    *progress = Progress::default();
                    let char_size = 0.4f32;
                    let character_shape = Geometry::Circle { radius: char_size };
                    let enemy_size = 0.4f32;
                    let _enemy_shape = Geometry::Circle { radius: enemy_size };
                    let life = Lifes(ship_stats.max_health);
                    let shield = Shield(ship_stats.max_shield);
                    let character = match gun_kind {
                        GunKind::Blaster(blaster) => {
                            entities.build_entity()
                                .with(*blaster, &mut blasters)
                        }
                        GunKind::Lazer(lazer) => {
                            entities.build_entity()
                                .with(*lazer, &mut lazers)
                        }
                        GunKind::ShotGun(shotgun) => {
                            entities.build_entity()
                                .with(*shotgun, &mut shotguns)
                        }
                        _ => {
                            unimplemented!()
                        }
                    };
                    let character = character
                        .with(life, &mut lifes)
                        .with(shield, &mut shields)
                        .with(Isometry::new(0f32, 0f32, 0f32), &mut isometries)
                        .with(Velocity::new(0f32, 0f32), &mut velocities)
                        .with(CharacterMarker::default(), &mut character_markers)
                        .with(Damage(ship_stats.damage), &mut damages)
                        .with(ShipMarker::default(), &mut ships)
                        .with(Image(preloaded_images.character), &mut images)
                        .with(Spin::default(), &mut spins)
                        .with(character_shape, &mut geometries)
                        .with(Size(char_size), &mut sizes)
                        .with(*ship_stats, &mut ships_stats)
                        .build();
                    let character_physics_shape = ncollide2d::shape::Ball::new(char_size);

                    let mut character_collision_groups = CollisionGroups::new();
                    character_collision_groups.set_membership(&[CollisionId::PlayerShip as usize]);
                    character_collision_groups.set_whitelist(&[
                        CollisionId::Asteroid as usize,
                        CollisionId::EnemyBullet as usize,
                        CollisionId::EnemyShip as usize,
                    ]);
                    character_collision_groups.set_blacklist(&[CollisionId::PlayerBullet as usize]);

                    PhysicsComponent::safe_insert(
                        &mut physics,
                        character,
                        ShapeHandle::new(character_physics_shape),
                        Isometry2::new(Vector2::new(0f32, 0f32), 0f32),
                        Velocity2::new(Vector2::new(0f32, 0f32), 0f32),
                        BodyStatus::Dynamic,
                        &mut world,
                        &mut bodies_map,
                        character_collision_groups,
                        0.5f32,
                    );
                    // TODO ENGINE
                    // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
                    reinsert.push(InsertEvent::Engine {
                        position: Point2::new(0f32, 0f32),
                        num: 4usize,
                        attached: AttachPosition(character)
                    });
                    {
                        entities
                            .build_entity()
                            .with(Isometry::new(0f32, 0f32, 0f32), &mut isometries)
                            .with(AttachPosition(character), &mut attach_positions)
                            .with(Velocity::new(0f32, 0f32), &mut velocities)
                            .with(Image(preloaded_images.light_white), &mut images)
                            .with(Spin::default(), &mut spins)
                            .with(Size(15f32), &mut sizes)
                            .with(LightMarker, &mut lights)
                            .build();
                    }
                }
                InsertEvent::Asteroid {
                    iso,
                    velocity,
                    polygon,
                    light_shape,
                    spin,
                } => {
                    let physics_polygon =
                        ncollide2d::shape::ConvexPolygon::try_from_points(&polygon.points())
                            .unwrap();
                    let asteroid = entities
                        .build_entity()
                        .with(light_shape.clone(), &mut geometries)
                        .with(Isometry::new(iso.x, iso.y, iso.z), &mut isometries)
                        .with(Velocity::new(velocity.linear.x, velocity.linear.y), &mut velocities)
                        .with(Lifes((ASTEROID_MAX_LIFES as f32 * polygon.min_r / ASTEROID_MAX_RADIUS) as usize), &mut lifes)
                        .with(polygon.clone(), &mut polygons)
                        .with(AsteroidMarker::default(), &mut asteroid_markers)
                        .with(Image(preloaded_images.asteroid), &mut images)
                        .with(Spin(*spin), &mut spins)
                        .with(Size(1f32), &mut sizes)
                        .build();

                    let mut asteroid_collision_groups = CollisionGroups::new();
                    asteroid_collision_groups.set_membership(&[CollisionId::Asteroid as usize]);
                    asteroid_collision_groups.set_whitelist(&[
                        CollisionId::Asteroid as usize,
                        CollisionId::EnemyShip as usize,
                        CollisionId::PlayerShip as usize,
                        CollisionId::PlayerBullet as usize,
                        CollisionId::EnemyBullet as usize,
                    ]);
                    PhysicsComponent::safe_insert(
                        &mut physics,
                        asteroid,
                        ShapeHandle::new(physics_polygon),
                        Isometry2::new(Vector2::new(iso.x, iso.y), iso.z),
                        *velocity,
                        BodyStatus::Dynamic,
                        &mut world,
                        &mut bodies_map,
                        asteroid_collision_groups,
                        ASTEROID_INERTIA,
                    );
                }
                InsertEvent::Ship {
                    iso,
                    light_shape: _,
                    spin: _,
                    kind,
                    gun_kind,
                    ship_stats,
                    size,
                    image,
                } => {
                    let size = *size;
                    let enemy_shape = 
                        Geometry::Circle { radius: size };
                    let enemy_physics_shape = 
                        ncollide2d::shape::Ball::new(size);
                    let mut enemy_collision_groups = CollisionGroups::new();
                    enemy_collision_groups.set_membership(&[CollisionId::EnemyShip as usize]);
                    enemy_collision_groups.set_whitelist(&[
                        CollisionId::Asteroid as usize,
                        CollisionId::EnemyShip as usize,
                        CollisionId::PlayerShip as usize,
                        CollisionId::PlayerBullet as usize,
                    ]);
                    enemy_collision_groups.set_blacklist(&[CollisionId::EnemyBullet as usize]);
                    let mut enemy = entities
                        .build_entity();
                    match gun_kind {
                        GunKind::ShotGun(shotgun) => {
                            let side_num = 3usize;
                            let _shift = std::f32::consts::PI / (side_num as f32 + 1.0);
                            enemy = enemy
                                .with(*shotgun, &mut shotguns)
                        }
                        GunKind::Blaster(blaster) => {
                            enemy = enemy
                                .with(*blaster, &mut blasters)
                        }
                        GunKind::Lazer(lazer) => {
                            enemy = enemy
                                .with(*lazer, &mut lazers)
                        }
                        GunKind::MultyLazer(multy_lazer) => {
                            enemy = enemy
                                .with(multy_lazer.clone(), &mut multy_lazers)
                        }
                        GunKind::Cannon(cannon) => {
                            enemy = enemy
                                .with(cannon.clone(), &mut cannons)
                        }
                    }
                    for kind in kind.kinds.iter() {
                        match kind {
                            AIType::Charging(time) => {
                                enemy = enemy
                                    .with(Charge{recharge_state: 0, recharge_time: *time}, &mut chargings)
                            }
                            _ => ()
                        }                        
                    }

                    let enemy = enemy
                        .with(Isometry::new(iso.x, iso.y, iso.z), &mut isometries)
                        .with(Velocity::new(0f32, 0f32), &mut velocities)
                        .with(EnemyMarker::default(), &mut enemies)
                        .with(ShipMarker::default(), &mut ships)
                        .with(*image, &mut images)
                        .with(Damage(ship_stats.damage), &mut damages)
                        .with(Lifes(ship_stats.max_health), &mut lifes)
                        .with(*ship_stats, &mut ships_stats)
                        // .with(Shield(ENEMY_MAX_SHIELDS), &mut shields)
                        .with(kind.clone(), &mut ais)
                        .with(Spin::default(), &mut spins)
                        .with(enemy_shape, &mut geometries)
                        .with(Size(size), &mut sizes)
                        .build();
                    PhysicsComponent::safe_insert(
                        &mut physics,
                        enemy,
                        ShapeHandle::new(enemy_physics_shape),
                        Isometry2::new(Vector2::new(iso.x, iso.y), iso.z),
                        Velocity2::new(Vector2::new(0f32, 0f32), 0f32),
                        BodyStatus::Dynamic,
                        &mut world,
                        &mut bodies_map,
                        enemy_collision_groups,
                        0.5f32,
                    );
                    // with light
                    //     {
                    // let _light = entities
                    //     .build_entity()
                    //     .with(Isometry::new(0f32, 0f32, 0f32), &mut isometries)
                    //     .with(Velocity::new(0f32, 0f32), &mut velocities)
                    //     .with(Spin::default(), &mut spins)
                    //     .with(AttachPosition(enemy), &mut attach_positions)
                    //     .with(Image(preloaded_images.light_sea), &mut images)
                    //     .with(Size(1f32), &mut sizes)
                    //     .with(LightMarker, &mut lights)
                    //     .build();
                    // }
                }
                InsertEvent::Bullet {
                    kind,
                    iso,
                    velocity,
                    damage,
                    owner,
                    bullet_image,
                    blast
                } => {
                    let r = 0.12;
                    // let image = match kind {
                    //     EntityType::Player => preloaded_images.projectile,
                    //     EntityType::Enemy => preloaded_images.enemy_projectile
                    // };
                    let mut bullet = entities
                        .build_entity()
                        .with(Damage(*damage), &mut damages)
                        .with(Velocity::new(velocity.x, velocity.y), &mut velocities)
                        .with(Isometry::new(iso.x, iso.y, iso.z), &mut isometries)
                        .with(Image(bullet_image.0), &mut images)
                        .with(Spin::default(), &mut spins)
                        .with(Projectile { owner: *owner }, &mut projectiles)
                        .with(Lifetime::new(100usize), &mut lifetimes)
                        .with(Size(r), &mut sizes);
                    if let Some(blast) = blast {
                        bullet = bullet
                            .with(*blast, &mut blasts)
                    }
                    let bullet = bullet
                        .build();
                    let player_bullet_collision_groups = get_collision_groups(kind);
                    let ball = ncollide2d::shape::Ball::new(r);
                    let bullet_physics_component = PhysicsComponent::safe_insert(
                        &mut physics,
                        bullet,
                        ShapeHandle::new(ball),
                        Isometry2::new(Vector2::new(iso.x, iso.y), iso.z),
                        Velocity2::new(Vector2::new(0f32, 0f32), 0f32),
                        BodyStatus::Dynamic,
                        &mut world,
                        &mut bodies_map,
                        player_bullet_collision_groups,
                        0.1f32,
                    );
                    let body = world
                        .rigid_body_mut(bullet_physics_component.body_handle)
                        .unwrap();
                    let mut velocity_tmp = *body.velocity();
                    *velocity_tmp.as_vector_mut() = Vector3::new(velocity.x, velocity.y, 0f32);
                    body.set_velocity(velocity_tmp);
                }
                InsertEvent::Coin {
                    value,
                    position
                } => {
                    let iso = Isometry::new(position.x, position.y, 0f32);
                    let _coin_entity = entities
                        .build_entity()
                        .with(Coin(*value), &mut coins)
                        .with(iso, &mut isometries)
                        .with(Size(0.1), &mut sizes)
                        .with(Image(preloaded_images.coin), &mut images)
                        .build();
                }
                InsertEvent::Exp {
                    value,
                    position
                } => {
                    let iso = Isometry::new(position.x, position.y, 0f32);
                    let _coin_entity = entities
                        .build_entity()
                        .with(Exp(*value), &mut exps)
                        .with(iso, &mut isometries)
                        .with(Size(0.1), &mut sizes)
                        .with(Image(preloaded_images.exp), &mut images)
                        .build();
                }
                InsertEvent::Explosion {
                    position,
                    num,
                    lifetime,
                    with_animation
                } => {
                    let iso = Isometry::new(position.x, position.y, 0f32);
                    if *with_animation {
                        let _animation_entity = entities
                            .build_entity()
                            .with(iso, &mut isometries)
                            .with(preloaded_images.explosion.clone(), &mut animations)
                            .with(Lifetime::new(100usize), &mut lifetimes)
                            .with(Size(4f32), &mut sizes)        
                            .build();            
                    }
                    // particles of explosion                        
                    let explosion_particles = ThreadPin::new(ParticlesData::Explosion(Explosion::new(
                        &gl,
                        *position,
                        *num,
                        Some(*lifetime),
                    )));
                    let _explosion_particles_entity = entities
                        .build_entity()
                        .with(explosion_particles, &mut particles_datas)
                        .build();
                }
                InsertEvent::Animation {
                    animation,
                    lifetime,
                    pos
                } => {
                    let iso = Isometry::new(pos.x, pos.y, 0f32);
                    let _animation_entity = entities
                        .build_entity()
                        .with(iso, &mut isometries)
                        .with(animation.clone(), &mut animations)
                        .with(Lifetime::new(*lifetime), &mut lifetimes)
                        .with(Size(4f32), &mut sizes)        
                        .build();            
                }
                InsertEvent::Engine {
                    position,
                    num,
                    attached: _
                } => {
                    let engine_particles = ThreadPin::new(ParticlesData::Engine(Engine::new(
                        &gl,
                        *position,
                        *num,
                        None,
                    )));
                    let _explosion_particles_entity = entities
                        .build_entity()
                        .with(engine_particles, &mut particles_datas)
                        .build();
                }
                InsertEvent::Nebula {
                    iso
                } => {
                    let mut rng = thread_rng();
                    let z = rng.gen_range(-120f32, -80f32);
                    let nebulas_num = preloaded_images.nebulas.len();
                    let nebula_id = rng.gen_range(0, nebulas_num);
                    let _nebula = entities
                        .build_entity()
                        .with(Isometry::new3d(iso.x, iso.y, z, iso.z), &mut isometries)
                        .with(Image(preloaded_images.nebulas[nebula_id]), &mut images)
                        .with(NebulaMarker::default(), &mut nebulas)
                        .with(Size(45f32), &mut sizes)
                        .build();
                }
                InsertEvent::Planet {
                    iso
                } => {
                    let mut rng = thread_rng();
                    let z = -45.0;
                    let planets_num = preloaded_images.planets.len();
                    let planet_id = rng.gen_range(0, planets_num);
                    let _nebula = entities
                        .build_entity()
                        .with(Isometry::new3d(iso.x, iso.y, z, iso.z), &mut isometries)
                        .with(Image(preloaded_images.planets[planet_id]), &mut images)
                        .with(PlanetMarker::default(), &mut planets)
                        .with(Size(25f32), &mut sizes)
                        .build();
                }
                InsertEvent::Wobble(wobble) => {
                    canvas.add_wobble(*wobble)
                }

            }
        }
        for i in reinsert.into_iter() {
            insert_channel.single_write(i);
        }
    }
}

// TODO: probably move out proc gen 
#[derive(Default)]
pub struct GamePlaySystem;

impl<'a> System<'a> for GamePlaySystem {
    type SystemData = (
        (
            Entities<'a>,
            WriteStorage<'a, PhysicsComponent>,
            WriteStorage<'a, Geometry>,
            WriteStorage<'a, Isometry>,
            WriteStorage<'a, Velocity>,
            WriteStorage<'a, Spin>,
            WriteStorage<'a, Blaster>,
            WriteStorage<'a, ShotGun>,
            WriteStorage<'a, Cannon>,
            WriteStorage<'a, Charge>,
            WriteStorage<'a, Blast>,
            WriteStorage<'a, Lifetime>,
            WriteStorage<'a, AsteroidMarker>,
            ReadStorage<'a, CharacterMarker>,
            ReadStorage<'a, ShipMarker>,
            WriteStorage<'a, Image>,
            WriteStorage<'a, Size>,
            WriteStorage<'a, Polygon>,
            WriteStorage<'a, NebulaMarker>,
            WriteStorage<'a, PlanetMarker>,
            WriteStorage<'a, Shield>,
            WriteStorage<'a, Lifes>,
            ReadStorage<'a, ShipStats>,
            ReadStorage<'a, Coin>,
            ReadStorage<'a, Exp>,
            ReadStorage<'a, Projectile>
        ),
        WriteExpect<'a, MacroGame>,
        Write<'a, Stat>,
        WriteExpect<'a, PreloadedImages>,
        Write<'a, World<f32>>,
        Write<'a, BodiesMap>,
        Write<'a, EventChannel<InsertEvent>>,
        Write<'a, Progress>,
        Write<'a, SpawnedUpgrades>,
        Read<'a, AvaliableUpgrades>,
        ReadExpect<'a, Description>,
        Write<'a, CurrentWave>,
        Read<'a, Waves>,
        Write<'a, EventChannel<Sound>>,
        ReadExpect<'a, PreloadedSounds>,
        Write<'a, AppState>,
        WriteExpect<'a, NebulaGrid>,
        WriteExpect<'a, PlanetGrid>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            (
                entities,
                _physics,
                _geometries,
                mut isometries,
                _velocities,
                _spins,
                mut blasters,
                mut shotguns,
                mut cannons,
                mut chargings,
                mut blasts,
                mut lifetimes,
                asteroid_markers,
                character_markers,
                ships,
                _images,
                _sizes,
                polygons,
                nebulas,
                planets,
                mut shields,
                mut lifes,
                ships_stats,
                coins,
                exps,
                projectiles,
            ),
            mut macro_game,
            _stat,
            preloaded_images,
            _world,
            _bodies_map,
            mut insert_channel,
            mut progress,
            mut spawned_upgrades,
            avaliable_upgrades,
            description,
            mut current_wave,
            waves,
            mut sounds_channel,
            preloaded_sounds,
            mut app_state,
            mut nebula_grid,
            mut planet_grid,
        ) = data;
        for (shield, life, ship_stats, _character) in (&mut shields, &mut lifes, &ships_stats, &character_markers).join() {
            shield.0 = (shield.0 + ship_stats.shield_regen).min(ship_stats.max_shield);
            life.0 = (life.0 + ship_stats.health_regen).min(ship_stats.max_health);
        }
        if progress.experience >= progress.current_max_experience() {
            progress.level_up();
            let mut rng = thread_rng();
            let up_id = rng.gen_range(0, avaliable_upgrades.len());
            let mut second_id = rng.gen_range(0, avaliable_upgrades.len());
            while second_id == up_id {
                second_id = rng.gen_range(0, avaliable_upgrades.len());
            }
            spawned_upgrades.push([up_id, second_id]);
            // *app_state = AppState::Play(PlayState::Upgrade);
        }
        let (char_entity, char_isometry, _char) = (&entities, &isometries, &character_markers).join().next().unwrap();
        let char_isometry = char_isometry.clone(); // to avoid borrow
        let pos3d = char_isometry.0.translation.vector;
        let character_position = Point2::new(pos3d.x, pos3d.y);
        for charge in (&mut chargings).join() {
            charge.update()
        }
        for gun in (&mut blasters).join() {
            gun.update()
        }
        for cannon in (&mut cannons).join() {
            cannon.update()
        }
        for shotgun in (&mut shotguns).join() {
            shotgun.update()
        }

        // let mut process_blast_damage = |
        //     blast: &Blast, 
        //     owner: specs::Entity,
        //     projectile: specs::Entity,
        // | {
        //     let blast_position = isometries.get(projectile).unwrap().0.translation.vector;
        //     for (entity, life, isometry) in (&entities, &mut lifes, &isometries).join() {
        //         let position = isometry.0.translation.vector;
        //         let is_character = entity == char_entity;
        //         let is_asteroid = asteroid_markers.get(entity).is_some(); 
        //         let affected = 
        //             is_character && owner != char_entity ||
        //             entity != char_entity && (owner == char_entity || is_asteroid);
        //         if affected && (blast_position - position).norm() < blast.blast_radius {
        //             if process_damage(life, shields.get_mut(entity), blast.blast_damage) {
        //                 if is_asteroid {
        //                     spawn_asteroids(
        //                         isometry.0, 
        //                         polygons.get(entity).unwrap(), 
        //                         &mut insert_channel,
        //                     );
        //                 }
        //                 if is_character {
        //                     *app_state = AppState::Menu;
        //                 }
        //                 // delete character
        //                 entities.delete(entity).unwrap();
        //                 // dbg!("dead");
        //             }
        //         }
        //     }
        // };

        for (entity, lifetime) in (&entities, &mut lifetimes).join() {
            lifetime.update();
            if lifetime.delete() {
                if let Some(blast) = blasts.get(entity) {
                    let owner = if let Some(projectile) = projectiles.get(entity) {
                        projectile.owner
                    } else {
                        entity
                    };
                    let position = isometries.get(entity).unwrap().0.translation.vector;
                    insert_channel.single_write(
                        InsertEvent::Animation {
                            animation: preloaded_images.blast.clone(),
                            lifetime: 100,
                            pos: Point2::new(position.x, position.y)
                        }
                    );
                    // insert_channel.single_write(
                    //     InsertEvent::Explosion{
                    //         position: Point2::new(position.x, position.y),
                    //         num: 10usize,
                    //         lifetime: 20usize,
                    //         with_animation: true
                    //     }
                    // );
                    { // process_blast_damage
                        let blast_position = isometries.get(entity).unwrap().0.translation.vector;

                        for (entity, life, isometry) in (&entities, &mut lifes, &isometries).join() {
                            let position = isometry.0.translation.vector;
                            let is_character = entity == char_entity;
                            let is_asteroid = asteroid_markers.get(entity).is_some(); 
                            let affected = 
                                is_character && owner != char_entity ||
                                entity != char_entity && (owner == char_entity || is_asteroid);
                            if affected && (blast_position - position).norm() < blast.blast_radius {
                                sounds_channel.single_write(Sound(preloaded_sounds.explosion));
                                if process_damage(life, shields.get_mut(entity), blast.blast_damage) {
                                    if is_asteroid {
                                        spawn_asteroids(
                                            isometry.0, 
                                            polygons.get(entity).unwrap(), 
                                            &mut insert_channel,
                                        );
                                    }
                                    if is_character {
                                        *app_state = AppState::Menu;
                                    }
                                    // delete character
                                    entities.delete(entity).unwrap();
                                    // dbg!("dead");
                                }
                            }
                        }
                    }
                }
                entities.delete(entity).unwrap()
            }
        }
        for (entity, iso) in (&entities, &mut isometries).join() {
            let coin = coins.get(entity);
            let exp = exps.get(entity);
            let collectable_position = iso.0.translation.vector;
            if coin.is_some() || exp.is_some() {
                if (pos3d - collectable_position).norm() < MAGNETO_RADIUS {
                    let vel = 0.3 * (pos3d - collectable_position).normalize();
                    iso.0.translation.vector += vel;
                }
            }
            if coin.is_some() && (pos3d - collectable_position).norm() < COLLECT_RADIUS {
                sounds_channel.single_write(Sound(preloaded_sounds.coin));
                progress.add_score(coin.unwrap().0);
                macro_game.coins += coin.unwrap().0;
                entities.delete(entity).unwrap();
            }
            if exp.is_some() && (pos3d - collectable_position).norm() < COLLECT_RADIUS {
                sounds_channel.single_write(Sound(preloaded_sounds.exp));
                progress.add_exp(exp.unwrap().0);
                entities.delete(entity).unwrap();
            }
        }
        let cnt = asteroid_markers.count();
        let add_cnt = if ASTEROIDS_MIN_NUMBER > cnt {
            ASTEROIDS_MIN_NUMBER - cnt
        } else {
            0
        };
        for _ in 0..add_cnt {
            let mut rng = thread_rng();
            let size = rng.gen_range(ASTEROID_MIN_RADIUS, ASTEROID_MAX_RADIUS);
            let r = size;
            // let asteroid_shape = Geometry::Circle { radius: r };
            let poly = generate_convex_polygon(10, r);
            let asteroid_shape = Geometry::Polygon(poly.clone());
            let spin = rng.gen_range(-1E-2, 1E-2);
            // let ball = ncollide2d::shape::Ball::new(r);
            let spawn_pos = spawn_position(character_position, PLAYER_AREA, ACTIVE_AREA);
            insert_channel.single_write(InsertEvent::Asteroid {
                iso: Point3::new(
                    spawn_pos.x,
                    spawn_pos.y,
                    char_isometry.0.rotation.euler_angles().2,
                ),
                velocity: initial_asteroid_velocity(),
                polygon: poly,
                light_shape: asteroid_shape,
                spin: spin,
            });
        }
        let cnt = ships.count();
        let wave = &waves.0[current_wave.id];
        let (add_cnt, const_spawn) = if cnt == 1 {
            current_wave.iteration += 1;
            (
                wave.ships_number - cnt + 1,
                true
            )
        } else {
            (
                0,
                false
            )
        };
        if current_wave.iteration > wave.iterations {
            current_wave.iteration = 0;
            current_wave.id = (waves.0.len() - 1).min(current_wave.id + 1);
        }
        let mut rng = thread_rng();
        fn ships2insert(
            spawn_pos: Point2,
            enemy: EnemyKind
        ) -> InsertEvent {
            InsertEvent::Ship {
                iso: Point3::new(spawn_pos.x, spawn_pos.y, 0f32),
                light_shape: Geometry::Circle { radius: 1f32 },
                spin: 0f32,
                kind: enemy.ai_kind,
                gun_kind: enemy.gun_kind,
                ship_stats: enemy.ship_stats,
                size: enemy.size,
                image: enemy.image,
            }
        };
        for _ in 0..add_cnt {
            let spawn_pos = spawn_position(character_position, PLAYER_AREA, ACTIVE_AREA);
            // TODO move from loop 
            let ships = &description.enemies;
            let ship_id = wave.distribution.choose_weighted(&mut rng, |item| item.1).unwrap().0;
            insert_channel.single_write(ships2insert(spawn_pos, ships[ship_id].clone()));
        };
        if const_spawn {
            for kind in wave.const_distribution.iter() {
                let spawn_pos = spawn_position(character_position, PLAYER_AREA, ACTIVE_AREA);
                let ships = &description.enemies;
                let ship_id = kind.0;
                insert_channel.single_write(ships2insert(spawn_pos, ships[ship_id].clone()));
            }
        }
        planet_grid.grid.reset();
        for (isometry, _planet) in (&isometries, &planets).join() {
            let position = isometry.0.translation.vector;
            let point = Point2::new(position.x, position.y);
            match planet_grid.grid.update(point, true) {
                Ok(_) => (),
                Err(_) => ()
            }
        }

        for i in 0..planet_grid.grid.size {
            for j in 0..planet_grid.grid.size {
                let value = *planet_grid.grid.get_cell_value(i, j);
                if !value {
                    let ((min_w, max_w), (min_h, max_h)) = planet_grid.grid.get_rectangle(i, j);
                    let spawn_pos = spawn_in_rectangle(min_w, max_w, min_h, max_h);
                    insert_channel.single_write(InsertEvent::Planet {
                        iso: Point3::new(spawn_pos.x, spawn_pos.y, 0f32)
                    })
                }
            }
        }

        for (entity, isometry, _planet) in (&entities, &isometries, &planets).join() {
            let pos3d = isometry.0.translation.vector;
            if  (pos3d.x - character_position.x).abs() > planet_grid.grid.max_w ||
                (pos3d.y - character_position.y).abs() > planet_grid.grid.max_h {
                entities.delete(entity).unwrap();
            }
        }

        nebula_grid.grid.reset();
        for (isometry, _nebula) in (&isometries, &nebulas).join() {
            let position = isometry.0.translation.vector;
            let point = Point2::new(position.x, position.y);
            match nebula_grid.grid.update(point, true) {
                Ok(_) => (),
                Err(_) => ()
            }
        }
        for i in 0..nebula_grid.grid.size {
            for j in 0..nebula_grid.grid.size {
                let value = *nebula_grid.grid.get_cell_value(i, j);
                if !value {
                    let ((min_w, max_w), (min_h, max_h)) = nebula_grid.grid.get_rectangle(i, j);
                    let spawn_pos = spawn_in_rectangle(min_w, max_w, min_h, max_h);
                    insert_channel.single_write(InsertEvent::Nebula {
                        iso: Point3::new(spawn_pos.x, spawn_pos.y, 0f32)
                    })
                }
            }
        }

        for (entity, isometry, _nebula) in (&entities, &isometries, &nebulas).join() {
            let pos3d = isometry.0.translation.vector;
            if  (pos3d.x - character_position.x).abs() > nebula_grid.grid.max_w ||
                (pos3d.y - character_position.y).abs() > nebula_grid.grid.max_h {
                entities.delete(entity).unwrap();
            }
        }


        // let cnt = nebulas.count();
        // let add_cnt = if NEBULA_MIN_NUMBER > cnt {
        //     NEBULA_MIN_NUMBER - cnt
        // } else {
        //     0
        // };
        // for _ in 0..add_cnt {
        //     let spawn_pos = spawn_position(character_position, NEBULA_PLAYER_AREA, NEBULA_ACTIVE_AREA);
        //     insert_channel.single_write(InsertEvent::Nebula {
        //         iso: Point3::new(spawn_pos.x, spawn_pos.y, 0f32)
        //     })
        // }
        for (entity, isometry, _asteroid) in (&entities, &isometries, &asteroid_markers).join() {
            let pos3d = isometry.0.translation.vector;
            if !is_active(character_position, Point2::new(pos3d.x, pos3d.y), ACTIVE_AREA) {
                entities.delete(entity).unwrap();
            }
        }
        for (entity, isometry, _ship) in (&entities, &isometries, &ships).join() {
            let pos3d = isometry.0.translation.vector;
            if !is_active(character_position, Point2::new(pos3d.x, pos3d.y), ACTIVE_AREA) {
                entities.delete(entity).unwrap();
            }
        }
        // for (entity, isometry, _nebula) in (&entities, &isometries, &nebulas).join() {
        //     let pos3d = isometry.0.translation.vector;
        //     if !is_active(character_position, Point2::new(pos3d.x, pos3d.y), NEBULA_ACTIVE_AREA) {
        //         entities.delete(entity).unwrap();
        //     }
        // }
    }
}

/// returns true if killed
fn process_damage(life: &mut Lifes, mut shield: Option<&mut Shield>, mut projectile_damage: usize) -> bool {
    match shield {
        Some(ref mut shield) if shield.0 > 0usize => {
            if shield.0 > projectile_damage {
                shield.0 -= projectile_damage;
                projectile_damage = 0;
            } else {
                shield.0 = 0;
                projectile_damage -= shield.0;
            }
        }
        _ => {
        }
    };
    if life.0 > projectile_damage {
        life.0 -= projectile_damage
    } else {
        return true;
    }
    false
}

#[derive(Default)]
pub struct CollisionSystem {
    colliding_start_events: Vec<(CollisionObjectHandle, CollisionObjectHandle)>,
    colliding_end_events: Vec<(CollisionObjectHandle, CollisionObjectHandle)>,
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
        WriteStorage<'a, Lifes>,
        WriteStorage<'a, Shield>,
        ReadStorage<'a, Damage>,
        WriteStorage<'a, Polygon>,
        Write<'a, World<f32>>,
        Read<'a, BodiesMap>,
        Write<'a, EventChannel<InsertEvent>>,
        Write<'a, EventChannel<Sound>>,
        ReadExpect<'a, PreloadedSounds>,
        Write<'a, Progress>,
        Write<'a, AppState>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            isometries,
            _physics,
            asteroids,
            character_markers,
            ships,
            projectiles,
            mut lifes,
            mut shields,
            damages,
            polygons,
            mut world,
            bodies_map,
            mut insert_channel,
            mut sounds_channel,
            preloaded_sounds,
            mut progress,
            mut app_state,
        ) = data;
        self.colliding_start_events.clear();
        self.colliding_end_events.clear();
        for event in world.contact_events() {
            match event {
                &ncollide2d::events::ContactEvent::Started(
                    collision_handle1,
                    collision_handle2,
                ) => self
                    .colliding_start_events
                    .push((collision_handle1, collision_handle2)),
                &ncollide2d::events::ContactEvent::Stopped(
                    collision_handle1,
                    collision_handle2,
                ) => self
                    .colliding_end_events
                    .push((collision_handle1, collision_handle2)),
            }
        }
        for (handle1, handle2) in self.colliding_start_events.iter() {
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
                if projectiles.get(entity2).is_some() {
                    let projectile = entity2;
                    let projectile_damage = damages.get(projectile).unwrap().0;
                    if projectile_damage != 0 {
                        entities.delete(projectile).unwrap();
                    }
                    let lifes = lifes.get_mut(asteroid).unwrap();
                    if lifes.0 > projectile_damage {
                        lifes.0 -= projectile_damage
                    } else {
                        asteroid_explosion = true
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
                        lifetime: 20usize,
                        with_animation: false
                    };
                    insert_channel.single_write(effect);
                    if character_markers.get(ship).is_some() {
                        sounds_channel.single_write(Sound(preloaded_sounds.collision));
                    }
                }
                if asteroid_explosion {
                    insert_channel.single_write(InsertEvent::Wobble(EXPLOSION_WOBBLE));
                    let isometry = isometries.get(asteroid).unwrap().0;
                    let position = isometry.translation.vector;
                    let effect = InsertEvent::Explosion {
                        position: Point2::new(position.x, position.y),
                        num: 10usize,
                        lifetime: 20usize,
                        with_animation: true
                    };
                    insert_channel.single_write(effect);
                    sounds_channel.single_write(Sound(preloaded_sounds.explosion));
                    spawn_asteroids(
                        isometries.get(asteroid).unwrap().0, 
                        polygons.get(asteroid).unwrap(), 
                        &mut insert_channel,
                    );
                    entities.delete(asteroid).unwrap();
                }
            }
            if ships.get(entity2).is_some() {
                swap(&mut entity1, &mut entity2);
            }
            if ships.get(entity1).is_some() && projectiles.get(entity2).is_some() {
                let ship = entity1;
                let projectile = entity2;
                let projectile_damage = damages.get(projectile).unwrap().0;
                let isometry = isometries.get(ship).unwrap().0;
                let position = isometry.translation.vector;
                if character_markers.get(ship).is_some() {
                    if process_damage(
                        lifes.get_mut(ship).unwrap(),
                        shields.get_mut(ship),
                        projectile_damage
                    ) {
                        *app_state = AppState::Menu;
                        // delete character
                        entities.delete(ship).unwrap();
                    }
                    insert_channel.single_write(InsertEvent::Wobble(0.1f32));
                } else {
                    let mut explosion_size = 2usize;
                    let mut with_animation = false;
                    let ship_pos = Point2::new(position.x, position.y);
                    if process_damage(
                        lifes.get_mut(ship).unwrap(),
                        shields.get_mut(ship),
                        projectile_damage
                    ) {
                        insert_channel.single_write(
                            InsertEvent::Exp{
                                value: 50, 
                                position: ship_pos
                            }
                        );
                        entities.delete(ship).unwrap();
                        explosion_size = 20;
                        with_animation = true;
                        insert_channel.single_write(InsertEvent::Wobble(EXPLOSION_WOBBLE));
                        sounds_channel.single_write(Sound(preloaded_sounds.explosion));
                    }
                    let effect = InsertEvent::Explosion {
                        position: ship_pos,
                        num: explosion_size,
                        lifetime: 50usize,
                        with_animation: with_animation
                    };
                    insert_channel.single_write(effect);
                }
                // Kludge
                if projectile_damage != 0 {
                    entities.delete(projectile).unwrap();
                }
            }
            if ships.get(entity1).is_some() && ships.get(entity2).is_some() {
                let mut ship1 = entity1;
                let mut ship2 = entity2;
                if character_markers.get(ship2).is_some() {
                    swap(&mut ship1, &mut ship2)
                }
                if character_markers.get(ship1).is_some() {
                    let character_ship = ship1;
                    let other_ship = ship2;
                    // entities.delete(other_ship).unwrap();
                    sounds_channel.single_write(Sound(preloaded_sounds.collision));
                    if process_damage(
                        lifes.get_mut(other_ship).unwrap(),
                        shields.get_mut(other_ship),
                        damages.get(character_ship).unwrap().0
                    ) {
                        entities.delete(other_ship).unwrap();
                    }
                    if process_damage(
                        lifes.get_mut(character_ship).unwrap(),
                        shields.get_mut(character_ship),
                        damages.get(other_ship).unwrap().0
                    ) {
                        *app_state = AppState::Menu;
                        // delete character
                        entities.delete(character_ship).unwrap();
                    }
                }
            }
        }
    }
}

fn get_min_dist(
    world: &mut Write<World<f32>>, 
    ray: Ray<f32>, 
    collision_gropus: CollisionGroups
) -> (f32, Option<BodyHandle>) {
    let mut mintoi = std::f32::MAX;
    let mut closest_body = None;
    for (b, inter) in world
            .collider_world()
            .interferences_with_ray(&ray, &collision_gropus) {
        if !b.query_type().is_proximity_query() && 
                inter.toi < mintoi && 
                inter.toi > EPS {
            mintoi = inter.toi;
            closest_body = Some(b.body());
        }
    }
    (mintoi, closest_body)
}

#[derive(Default)]
pub struct AISystem;

impl<'a> System<'a> for AISystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Isometry>,
        WriteStorage<'a, Velocity>,
        ReadStorage<'a, PhysicsComponent>,
        WriteStorage<'a, Spin>,
        WriteStorage<'a, Blaster>,
        WriteStorage<'a, ShotGun>,
        WriteStorage<'a, Lazer>,
        WriteStorage<'a, MultyLazer>,
        WriteStorage<'a, Cannon>,
        WriteStorage<'a, EnemyMarker>,
        WriteStorage<'a, Shield>,
        WriteStorage<'a, Lifes>,
        WriteStorage<'a, Polygon>,
        WriteStorage<'a, Charge>,
        ReadStorage<'a, CharacterMarker>,
        ReadStorage<'a, AsteroidMarker>,
        ReadStorage<'a, AI>,
        ReadExpect<'a, PreloadedImages>,
        Write<'a, Stat>,
        Write<'a, World<f32>>,
        Write<'a, EventChannel<InsertEvent>>,
        Write<'a, BodiesMap>,
        Write<'a, EventChannel<Sound>>,
        Write<'a, AppState>,
        ReadExpect<'a, PreloadedSounds>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            isometries,
            mut velocities,
            physics,
            mut spins,
            mut blasters,
            mut shotguns,
            mut lazers,
            mut multy_lazers,
            mut cannons,
            enemies,
            _shields,
            _lifes,
            _polygons,
            mut chargings,
            character_markers,
            _asteroid_markers,
            ais,
            _preloaded_images,
            _stat,
            mut world,
            mut insert_channel,
            bodies_map,
            mut sounds_channel,
            _app_state,
            preloaded_sounds,
        ) = data;
        let _rng = thread_rng();
        let (character_entity, character_position, _) = (&entities, &isometries, &character_markers)
            .join().next().unwrap();
        let character_position = character_position.0.translation.vector;
        // let character_position = {
        //     let mut res = None;
        //     for (iso, _) in (&isometries, &character_markers).join() {
        //         res = Some(iso.0.translation.vector)
        //     }
        //     res.unwrap()
        // };
        for (entity, iso, vel, physics_component, spin, _enemy, ai) in (
            &entities,
            &isometries,
            &mut velocities,
            &physics,
            &mut spins,
            &enemies,
            &ais
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
            if let Some(body) = nearby.1 { // body that we facing
                if bodies_map[&body] == character_entity {
                    character_noticed = true;
                }
            };
            let follow_area = if let Some(lazer) = lazers.get(entity) {lazer.distance * 0.95} else {SCREEN_AREA};
            for ai_type in ai.kinds.iter() {
                match ai_type {
                    AIType::Shoot => {
                        let gun = blasters.get_mut(entity);
                        if let Some(gun) = gun {
                            if diff.norm() < SCREEN_AREA && gun.shoot() && character_noticed {
                                let bullets = gun.spawn_bullets(
                                    EntityType::Enemy,
                                    isometry,
                                    gun.bullet_speed,
                                    gun.bullets_damage,
                                    Vector2::new(vel.0.x, vel.0.y),
                                    entity
                                );
                                insert_channel.iter_write(bullets.into_iter());
                                sounds_channel.single_write(Sound(preloaded_sounds.enemy_blaster))
                            }
                        }
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
                                    entity
                                );
                                insert_channel.iter_write(bullets.into_iter());
                                sounds_channel.single_write(Sound(preloaded_sounds.enemy_blaster))
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
                                    entity
                                );
                                insert_channel.iter_write(bullets.into_iter());
                                sounds_channel.single_write(Sound(preloaded_sounds.enemy_shotgun))
                            }
                        }
                        if diff.norm() > follow_area {
                            if let Some(lazer) = lazers.get_mut(entity) {
                                lazer.active = false;
                            }
                            if let Some(multy_lazer) = multy_lazers.get_mut(entity) {
                                for lazer in multy_lazer.lazers.iter_mut() {
                                    lazer.active = false;
                                }
                            }
                        } else {
                            if let Some(lazer) = lazers.get_mut(entity) {
                                lazer.active = true;
                            }
                                if let Some(multy_lazer) = multy_lazers.get_mut(entity) {
                                    for lazer in multy_lazer.lazers.iter_mut() {
                                        lazer.active = true;
                                    }
                                }
                        }

                    }
                    AIType::Follow => {
                        let speed = 0.1f32;
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
                        let charging = chargings.get_mut(entity).expect("no charging component while have charging AI");
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
        }
    }
}
