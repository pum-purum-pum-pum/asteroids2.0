
use std::mem::swap;

use common::*;
use rand::prelude::*;
use sdl2::keyboard::Keycode;
use std::time::{Instant, Duration};

use ncollide2d::shape::ShapeHandle;
use ncollide2d::world::CollisionGroups;
use ncollide2d::world::CollisionObjectHandle;
use ncollide2d::query::{Ray, ContactManifold};
use nphysics2d::object::{Body, BodyStatus, BodyHandle};
use nphysics2d::world::World;
use nphysics2d::algebra::ForceType;
use nphysics2d::algebra::Force2;
use shrev::EventChannel;
use specs::prelude::*;
use specs::Join;

use crate::components::*;
use crate::geometry::{generate_convex_polygon, Polygon, TriangulateFromCenter, EPS};
use gfx_h::{GeometryData, ParticlesData, Explosion};
use crate::physics::CollisionId;
use crate::sound::{PreloadedSounds, SoundData, MusicData, EFFECT_MAX_VOLUME};
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
const ASTEROIDS_MIN_NUMBER: usize = 25;
const ASTEROID_MAX_RADIUS: f32 = 4.2f32;
const ASTEROID_MIN_RADIUS: f32 = 0.5;
const ASTEROID_INERTIA: f32 = 2f32;

const EXPLOSION_WOBBLE: f32 = 0.4;
const DAMAGED_RED: f32 = 0.2;

const MAGNETO_RADIUS: f32 = 4f32;
const COLLECT_RADIUS: f32 = 0.2;
pub const DT: f32 =  1f32 / 60f32;
const COIN_LIFETIME_SECS: u64 = 5;
const EXPLOSION_LIFETIME_SECS: u64 = 1;
const BLAST_LIFETIME_SECS: u64 = 1;
const BULLET_CONTACT_LIFETIME_SECS: u64 = 1;
const COLLECTABLE_SIDE_BULLET: u64 = 5;
const SIDE_BULLET_LIFETIME_SEC: u64 = 6;
const DOUBLE_COINS_LIFETIME_SEC: u64 = 5;
const COLLECTABLE_DOUBLE_COINS_SEC: u64 = 5;

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

fn iso3_iso2(iso3: &Isometry3) -> Isometry2 {
    Isometry2::new(
        Vector2::new(iso3.translation.vector.x, iso3.translation.vector.y),
        iso3.rotation.euler_angles().2
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
        ReadStorage<'a, Rocket>,
        ReadStorage<'a, Charge>,
        ReadStorage<'a, Chain>,
        WriteStorage<'a, Spin>,
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
            enemies,
            rockets,
            chargings,
            chains,
            mut spins,
            mut world,
            mut nebula_grid,
            mut planet_grid,
            app_state
        ) = data;
        flame::start("physics");
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
        let char_vec = character_position.translation.vector;
        { // rockets movements logic
            for (_entity, iso, vel, spin, phys, rocket) in (&entities, &isometries, &velocities, &mut spins, &physics, &rockets).join() {
                let rocket_vec = iso.0.translation.vector;
                let rocket_pos = Point2::new(rocket_vec.x, rocket_vec.y);
                // let _middle = (rocket_pos + char_vec) / 2.0;
                let direct = char_vec - rocket_pos.coords;
                let near_vel = 0.13 * direct.normalize();
                let rigid_body = world
                    .rigid_body_mut(phys.body_handle).unwrap();
                if Instant::now() - rocket.0 > Duration::from_secs(2) {
                    rigid_body.set_velocity(nphysics2d::math::Velocity::linear(near_vel.x, near_vel.y))

                } else {
                    let maneuver = if direct.dot(&vel.0) < 0.0 { 20.0 } else {1.0};
                    let force = Force2::new(maneuver * 0.00014 * (direct).normalize(), 0.0);
                    rigid_body.apply_force(0, &force, ForceType::Force, true);
                }
                // rigid_body.activate();

                let rocket_torque = DT
                    * calculate_player_ship_spin_for_aim(
                        Vector2::new(char_vec.x, char_vec.y)
                            - Vector2::new(rocket_pos.x, rocket_pos.y),
                        iso.rotation(),
                        spin.0,
                    );
                spin.0 += rocket_torque.max(-MAX_TORQUE).min(MAX_TORQUE);
                // TODO move to Kinematic system?
                rigid_body.set_angular_velocity(spin.0);
            }
        }

        {   // Reactive enemies O(n^2)
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
                    if chains.get(*e1).is_some() || chains.get(*e2).is_some() {
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
                            Force2::new(0.006 * (position1 - center).normalize(), 0.0), 
                            Force2::new(0.006 * (position2 - center).normalize(), 0.0),
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
        flame::end("physics");
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
        ReadStorage<'a, MultyLazer>,
        WriteStorage<'a, SoundPlacement>,
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
            multy_lazers,
            mut sound_placements,
            preloaded_sounds,
            sounds_channel,
            mut loop_sound,
            music_data,
            mut music,
            app_state
        ) = data;
        for s in sounds_channel.read(&mut self.reader) {
            let sound = &sounds.get(s.0).unwrap().0;
            let position = s.1;
            let placement = sound_placements.get_mut(s.0).unwrap();
            for i in placement.start..placement.end {
                let current_channel = sdl2::mixer::Channel(i as i32);
                if !current_channel.is_playing() && 
                    Instant::now()
                        .duration_since(placement.last_upd) >= placement.gap {
                    placement.last_upd = Instant::now();
                    current_channel.play(sound, 0).unwrap();
                    let n = position.coords.norm();
                    // let smooth = 1.0; // more value less depend on l
                    let l = 1.0 + n;
                    let mut fade = 1.0 / (l.ln());
                    if n < 10f32 {
                        fade = 1.0;
                    }
                    current_channel.set_volume(
                        (EFFECT_MAX_VOLUME as f32 * fade) as i32
                    );
                    break;
                }
            }
        }
        for (lazer, _character) in (&multy_lazers, &character_markers).join() {
            if lazer.active() {
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
                    sdl2::mixer::Music::halt();
                    music.menu_play = false;
                    // music_data.battle_music[music_id].play(-1).unwrap();
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
                    // music_data.menu_music.play(-1).unwrap();
                    music.menu_play = true;
                }
            }
            AppState::ScoreTable => {
                
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
                    isometries.get_mut(*entity).unwrap().0.translation.vector 
                        = iso.0.translation.vector;
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
    bullet_position: Option<Point2>,
) {
    let position = isometry.translation.vector;
    let new_polygons = if let Some(bullet_position) = bullet_position {
        polygon.deconstruct(bullet_position - Vector2::new(position.x, position.y))
    }
    else {
        polygon.deconstruct(polygon.center())
    };
    let mut rng = thread_rng();
    if new_polygons.len() > 1 {
        for poly in new_polygons.iter() {
            let insert_event = InsertEvent::Asteroid {
                iso: Point3::new(position.x, position.y, isometry.rotation.euler_angles().2),
                velocity: initial_asteroid_velocity(),
                polygon: poly.clone(),
                spin: rng.gen_range(-1E-2, 1E-2),
            };
            insert_channel.single_write(insert_event);
        }
    } else {
        // spawn coins and stuff
        let spawn_position = Point2::new(position.x, position.y);
        if rng.gen_range(0.0, 1.0) < 0.41 {
            insert_channel.single_write(InsertEvent::Health{
                value: 100,
                position: spawn_position
            })
        }

        if rng.gen_range(0.0, 1.0) < 0.02 {
            insert_channel.single_write(InsertEvent::Coin{
                value: 1,
                position: spawn_position
            });
        }
        if rng.gen_range(0.0, 1.0) < 0.05 {
            insert_channel.single_write(InsertEvent::SideBulletCollectable{position: spawn_position});
        }
        if rng.gen_range(0.0, 1.0) < 0.02 {
            insert_channel.single_write(InsertEvent::DoubleCoinsCollectable{position: spawn_position});
        }
        if rng.gen_range(0.0, 1.0) < 0.02 {
            insert_channel.single_write(InsertEvent::DoubleExpCollectable{position: spawn_position});
        }
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
                                entities.delete(*target_entity).unwrap();
                            }
                            let effect_position = position + dir * min_d;
                            let effect = InsertEvent::Explosion {
                                position: Point2::new(effect_position.x, effect_position.y),
                                num: explosion_size,
                                lifetime: Duration::from_millis(800),
                                with_animation: None
                            };
                            insert_channel.single_write(effect);
                        }
                    }
                } else {
                    lazer.current_distance = lazer.distance
                }
            };
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
            WriteStorage<'a, Image>,
            WriteStorage<'a, Size>,
        ),
        (
            WriteStorage<'a, StarsMarker>,
            WriteStorage<'a, NebulaMarker>,
            WriteStorage<'a, PlanetMarker>,
            WriteStorage<'a, AttachPosition>,
            WriteStorage<'a, LightMarker>,
        ),
        ReadExpect<'a, ThreadPin<red::GL>>,
        WriteExpect<'a, PreloadedImages>,
        Write<'a, World<f32>>,
        Write<'a, BodiesMap>,
        Write<'a, Progress>,
        Read<'a, EventChannel<InsertEvent>>,
        WriteExpect<'a, Canvas>,
        Read<'a, LazyUpdate>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            (
                entities,
                mut physics,
                _geometries,
                mut isometries,
                mut velocities,
                mut spins,
                mut images,
                mut sizes,
            ),
            (
                _stars,
                _nebulas,
                _planets,
                mut attach_positions,
                mut lights,
            ),
            gl,
            preloaded_images,
            mut world,
            mut bodies_map,
            mut progress,
            insert_channel,
            mut canvas,
            lazy_update,
        ) = data;
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
                    let character = entities.create();
                    match gun_kind {
                        GunKind::MultyLazer(multy_lazer) => {
                            lazy_update.insert(character, multy_lazer.clone())
                        }
                        GunKind::ShotGun(shotgun) => {
                            lazy_update.insert(character, *shotgun);
                        }
                        _ => {
                            unimplemented!()
                        }
                    };
                    lazy_update.insert(character, life);
                    lazy_update.insert(character, shield);
                    lazy_update.insert(character, Isometry::new(0f32, 0f32, 0f32));
                    lazy_update.insert(character, Velocity::new(0f32, 0f32));
                    lazy_update.insert(character, CharacterMarker::default());
                    lazy_update.insert(character, Damage(ship_stats.damage));
                    lazy_update.insert(character, ShipMarker::default());
                    lazy_update.insert(character, Image(preloaded_images.character));
                    lazy_update.insert(character, Spin::default());
                    lazy_update.insert(character, character_shape);
                    lazy_update.insert(character, Size(char_size));
                    lazy_update.insert(character, *ship_stats);
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
                    spin,
                } => {
                    let mut polygon = polygon.clone();
                    let center = polygon.center();
                    polygon.centralize(Rotation2::new(iso.z));
                    let light_shape = Geometry::Polygon(polygon.clone());
                    let iso = Point3::new(iso.x + center.x, iso.y + center.y, 0.0);
                    let physics_polygon =
                        if let Some(physics_polygon) = ncollide2d::shape::ConvexPolygon::try_from_points(&polygon.points()) {
                            physics_polygon
                        } else {
                            // TODO: looks like BUG!
                            dbg!(&polygon.points);
                            break
                            // panic!();
                        };
                    let asteroid = entities.create();
                    lazy_update.insert(asteroid, light_shape.clone());
                    lazy_update.insert(asteroid, Isometry::new(iso.x, iso.y, iso.z));
                    lazy_update.insert(asteroid, Velocity::new(velocity.linear.x, velocity.linear.y));
                    lazy_update.insert(asteroid, Lifes((ASTEROID_MAX_LIFES as f32 * polygon.min_r / ASTEROID_MAX_RADIUS) as usize));
                    lazy_update.insert(asteroid, polygon);
                    lazy_update.insert(asteroid, AsteroidMarker::default());
                    lazy_update.insert(asteroid, Image(preloaded_images.asteroid));
                    lazy_update.insert(asteroid, Spin(*spin));
                    lazy_update.insert(asteroid, Size(1f32));
                    
                    // let asteroid = entities
                    //     .build_entity()
                    //     .with(light_shape.clone(), &mut geometries)
                    //     .with(Isometry::new(iso.x, iso.y, iso.z), &mut isometries)
                    //     .with(Velocity::new(velocity.linear.x, velocity.linear.y), &mut velocities)
                    //     .with(Lifes((ASTEROID_MAX_LIFES as f32 * polygon.min_r / ASTEROID_MAX_RADIUS) as usize), &mut lifes)
                    //     .with(polygon, &mut polygons)
                    //     .with(AsteroidMarker::default(), &mut asteroid_markers)
                    //     .with(Image(preloaded_images.asteroid), &mut images)
                    //     .with(Spin(*spin), &mut spins)
                    //     .with(Size(1f32), &mut sizes)
                    //     .build();

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
                    snake,
                    rift,
                } => {
                    let num = if let Some(chains) = snake {*chains} else {1};
                    let mut last_entity = None;
                    for i in 0..num {
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
                        let enemy = entities.create();


                        match gun_kind {
                            GunKind::ShotGun(shotgun) => {
                                let side_num = 3usize;
                                let _shift = std::f32::consts::PI / (side_num as f32 + 1.0);
                                lazy_update.insert(enemy, *shotgun);
                            }
                            GunKind::MultyLazer(multy_lazer) => {
                                lazy_update.insert(enemy, multy_lazer.clone());
                            }
                            GunKind::Cannon(cannon) => {
                                lazy_update.insert(enemy, cannon.clone());
                            }
                            GunKind::RocketGun(rocket_gun) => {
                                lazy_update.insert(enemy, *rocket_gun);
                            }
                        }
                        for kind in kind.kinds.iter() {
                            match kind {
                                AIType::Charging(time) => {
                                    lazy_update.insert(enemy, Charge::new(*time));
                                    // enemy = enemy
                                    //     .with(Charge::new(*time), &mut chargings)
                                }
                                _ => ()
                            }                        
                        }
                        let iso = Point3::new(iso.x + i as f32, iso.y, iso.z);
                        lazy_update.insert(enemy, Isometry::new(iso.x, iso.y, iso.z));
                        lazy_update.insert(enemy, Velocity::new(0f32, 0f32));
                        lazy_update.insert(enemy, EnemyMarker::default());
                        lazy_update.insert(enemy, ShipMarker::default());
                        lazy_update.insert(enemy, *image);
                        lazy_update.insert(enemy, Damage(ship_stats.damage));
                        lazy_update.insert(enemy, Lifes(ship_stats.max_health));
                        lazy_update.insert(enemy, *ship_stats);
                        lazy_update.insert(enemy, kind.clone());
                        lazy_update.insert(enemy, Spin::default());
                        lazy_update.insert(enemy, enemy_shape);
                        lazy_update.insert(enemy, Size(size));
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
                        // snake thing
                        if let Some(last_entity) = last_entity {
                            if snake.is_some() {
                                lazy_update.insert(enemy, Chain{follow: last_entity})
                            }
                        }
                        if let Some(rift) = rift {
                            lazy_update.insert(enemy, rift.clone());
                            // lazy_update.insert(enemy, Aim(last_entity))
                        }
                        last_entity = Some(enemy);
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
                }
                InsertEvent::Bullet {
                    kind,
                    iso,
                    size,
                    velocity,
                    damage,
                    owner,
                    lifetime,
                    bullet_image,
                    blast,
                    reflection
                } => {
                    let bullet = entities.create();
                    lazy_update.insert(bullet, Damage(*damage));
                    lazy_update.insert(bullet, Velocity::new(velocity.x, velocity.y));
                    lazy_update.insert(bullet, Isometry::new(iso.x, iso.y, iso.z));
                    lazy_update.insert(bullet, Image(bullet_image.0));
                    lazy_update.insert(bullet, Spin::default());
                    lazy_update.insert(bullet, Projectile { owner: *owner });
                    lazy_update.insert(bullet, Lifetime::new(*lifetime));
                    lazy_update.insert(bullet, Size(*size));
                    if let Some(reflection) = reflection {
                        lazy_update.insert(bullet, *reflection);
                    }

                    // let mut bullet = entities
                    //     .build_entity()
                    //     .with(Damage(*damage), &mut damages)
                    //     .with(Velocity::new(velocity.x, velocity.y), &mut velocities)
                    //     .with(Isometry::new(iso.x, iso.y, iso.z), &mut isometries)
                    //     .with(Image(bullet_image.0), &mut images)
                    //     .with(Spin::default(), &mut spins)
                    //     .with(Projectile { owner: *owner }, &mut projectiles)
                    //     .with(Lifetime::new(*lifetime), &mut lifetimes)
                    //     .with(Size(r), &mut sizes);
                    if let Some(blast) = blast {
                        lazy_update.insert(bullet, *blast);
                        // bullet = bullet
                        //     .with(*blast, &mut blasts)
                    }
                    // let bullet = bullet
                    //     .build();
                    let bullet_collision_groups = get_collision_groups(kind);
                    let ball = ncollide2d::shape::Ball::new(*size);
                    let bullet_physics_component = PhysicsComponent::safe_insert(
                        &mut physics,
                        bullet,
                        ShapeHandle::new(ball),
                        Isometry2::new(Vector2::new(iso.x, iso.y), iso.z),
                        Velocity2::new(Vector2::new(0f32, 0f32), 0f32),
                        BodyStatus::Dynamic,
                        &mut world,
                        &mut bodies_map,
                        bullet_collision_groups,
                        0.1f32,
                    );
                    let body = world
                        .rigid_body_mut(bullet_physics_component.body_handle)
                        .unwrap();
                    let mut velocity_tmp = *body.velocity();
                    *velocity_tmp.as_vector_mut() = Vector3::new(velocity.x, velocity.y, 0f32);
                    body.set_velocity(velocity_tmp);
                }
                InsertEvent::Rocket {
                    kind,
                    iso,
                    damage,
                    owner,
                    rocket_image,
                } => {
                    let r = 0.3;
                    let entity = entities.create();
                    lazy_update.insert(entity, Damage(*damage));
                    lazy_update.insert(entity, Isometry::new(iso.x, iso.y, iso.z));
                    lazy_update.insert(entity, Velocity::new(0f32, 0f32));
                    lazy_update.insert(entity, Image(rocket_image.0));
                    lazy_update.insert(entity, Spin::default());
                    lazy_update.insert(entity, Rocket(Instant::now()));
                    lazy_update.insert(entity, Projectile { owner: *owner });
                    lazy_update.insert(entity, Size(r));
                    let bullet_collision_groups = get_collision_groups(kind);
                    let ball = ncollide2d::shape::Ball::new(r);
                    let bullet_physics_component = PhysicsComponent::safe_insert(
                        &mut physics,
                        entity,
                        ShapeHandle::new(ball),
                        Isometry2::new(Vector2::new(iso.x, iso.y), iso.z),
                        Velocity2::new(Vector2::new(0f32, 0f32), 0f32),
                        BodyStatus::Dynamic,
                        &mut world,
                        &mut bodies_map,
                        bullet_collision_groups,
                        0.25f32,
                    );
                    let _body = world
                        .rigid_body_mut(bullet_physics_component.body_handle)
                        .unwrap();
                }
                InsertEvent::Coin {
                    value,
                    position
                } => {
                    let iso = Isometry::new(position.x, position.y, 0f32);
                    let entity = entities.create();
                    lazy_update.insert(entity, CollectableMarker);
                    lazy_update.insert(entity, Coin(*value));
                    lazy_update.insert(entity, iso);
                    lazy_update.insert(entity, Size(0.25));
                    lazy_update.insert(entity, Image(preloaded_images.coin));
                    lazy_update.insert(entity, Lifetime::new(Duration::from_secs(COIN_LIFETIME_SECS)));
                }
                InsertEvent::SideBulletCollectable {
                    position
                } => {
                    let iso = Isometry::new(position.x, position.y, 0f32);
                    let entity = entities.create();
                    lazy_update.insert(entity, CollectableMarker);
                    lazy_update.insert(entity, SideBulletCollectable);
                    lazy_update.insert(entity, Lifetime::new(Duration::from_secs(COLLECTABLE_SIDE_BULLET)));
                    lazy_update.insert(entity, iso);
                    lazy_update.insert(entity, Size(0.5));
                    lazy_update.insert(entity, Image(preloaded_images.side_bullet_ability));
                }
                InsertEvent::SideBulletAbility => {
                    let entity = entities.create();
                    lazy_update.insert(entity, SideBulletAbility);
                    lazy_update.insert(entity, Lifetime::new(Duration::from_secs(SIDE_BULLET_LIFETIME_SEC)));
                }
                InsertEvent::DoubleCoinsCollectable {
                    position
                } => {
                    let entity = entities.create();
                    let iso = Isometry::new(position.x, position.y, 0f32);
                    lazy_update.insert(entity, CollectableMarker);
                    lazy_update.insert(entity, DoubleCoinsCollectable);
                    lazy_update.insert(entity, Lifetime::new(Duration::from_secs(COLLECTABLE_DOUBLE_COINS_SEC)));
                    lazy_update.insert(entity, iso);
                    lazy_update.insert(entity, Size(0.5));
                    lazy_update.insert(entity, Image(preloaded_images.double_coin));
                }
                InsertEvent::DoubleCoinsAbility => {
                    let entity = entities.create();
                    lazy_update.insert(entity, DoubleCoinsAbility);
                    lazy_update.insert(entity, Lifetime::new(Duration::from_secs(DOUBLE_COINS_LIFETIME_SEC)));
                }
                InsertEvent::DoubleExpCollectable {
                    position
                } => {
                    let iso = Isometry::new(position.x, position.y, 0f32);
                    let coin_entity = entities.create();
                    lazy_update.insert(coin_entity, CollectableMarker);
                    lazy_update.insert(coin_entity, DoubleExpCollectable);
                    lazy_update.insert(coin_entity, Lifetime::new(Duration::from_secs(COLLECTABLE_DOUBLE_COINS_SEC)));
                    lazy_update.insert(coin_entity, iso);
                    lazy_update.insert(coin_entity, Size(0.5));
                    lazy_update.insert(coin_entity, Image(preloaded_images.double_exp));
                    lazy_update.insert(coin_entity, Lifetime::new(Duration::from_secs(COIN_LIFETIME_SECS)));
                }
                InsertEvent::DoubleExpAbility => {
                    let entity = entities.create();
                    lazy_update.insert(entity, DoubleExpAbility);
                    lazy_update.insert(entity, Lifetime::new(Duration::from_secs(DOUBLE_COINS_LIFETIME_SEC)));
                }
                InsertEvent::Health {
                    value,
                    position,
                } => {
                    let iso = Isometry::new(position.x, position.y, 0f32);
                    let coin_entity = entities.create();
                    lazy_update.insert(coin_entity, CollectableMarker);
                    lazy_update.insert(coin_entity, Health(*value));
                    lazy_update.insert(coin_entity, iso);
                    lazy_update.insert(coin_entity, Size(0.25));
                    lazy_update.insert(coin_entity, Image(preloaded_images.health));
                    lazy_update.insert(coin_entity, Lifetime::new(Duration::from_secs(COIN_LIFETIME_SECS)));
                }
                InsertEvent::Exp {
                    value,
                    position
                } => {
                    let iso = Isometry::new(position.x, position.y, 0f32);
                    let exp_entity = entities.create();
                    lazy_update.insert(exp_entity, CollectableMarker);
                    lazy_update.insert(exp_entity, Exp(*value));
                    lazy_update.insert(exp_entity, iso);
                    lazy_update.insert(exp_entity, Size(0.25));
                    lazy_update.insert(exp_entity, Image(preloaded_images.exp));
                }
                InsertEvent::Explosion {
                    position,
                    num,
                    lifetime,
                    with_animation
                } => {
                    let iso = Isometry::new(position.x, position.y, 0f32);
                    if let Some(size) = with_animation {
                        let animation_entity = entities.create();
                        lazy_update.insert(animation_entity, iso);
                        lazy_update.insert(animation_entity, preloaded_images.explosion.clone());
                        lazy_update.insert(animation_entity, Lifetime::new(Duration::from_secs(EXPLOSION_LIFETIME_SECS)));
                        lazy_update.insert(animation_entity, Size(size * 2.0));
                    }
                    // particles of explosion                        
                    let explosion_particles = ThreadPin::new(ParticlesData::Explosion(Explosion::new(
                        &gl,
                        *position,
                        *num,
                        Some(*lifetime),
                    )));
                    let explosion_particles_entity = entities.create();
                    lazy_update.insert(explosion_particles_entity, explosion_particles);
                }
                InsertEvent::Animation {
                    animation,
                    lifetime,
                    pos,
                    size
                } => {
                    let iso = Isometry::new(pos.x, pos.y, 0f32);
                    let animation_entity = entities.create();
                    lazy_update.insert(animation_entity, iso);
                    lazy_update.insert(animation_entity, animation.clone());
                    lazy_update.insert(animation_entity, Lifetime::new(*lifetime));
                    lazy_update.insert(animation_entity, Size(*size));        
                }
                InsertEvent::Nebula {
                    iso
                } => {
                    let mut rng = thread_rng();
                    let z = rng.gen_range(-120f32, -80f32);
                    let nebulas_num = preloaded_images.nebulas.len();
                    let nebula_id = rng.gen_range(0, nebulas_num);
                    let nebula = entities.create();
                    lazy_update.insert(nebula, Isometry::new3d(iso.x, iso.y, z, iso.z));
                    lazy_update.insert(nebula, Image(preloaded_images.nebulas[nebula_id]));
                    lazy_update.insert(nebula, NebulaMarker::default());
                    lazy_update.insert(nebula, Size(60f32));
                }
                InsertEvent::Stars {
                    iso
                } => {
                    let mut rng = thread_rng();
                    let z = rng.gen_range(-180f32, -140f32);
                    let stars_num = preloaded_images.stars.len();
                    let stars_id = rng.gen_range(0, stars_num);
                    let stars = entities.create();
                    lazy_update.insert(stars, Isometry::new3d(iso.x, iso.y, z, iso.z));
                    lazy_update.insert(stars, Image(preloaded_images.stars[stars_id]));
                    lazy_update.insert(stars, StarsMarker);
                    lazy_update.insert(stars, Size(30f32));
                }
                InsertEvent::BigStar {
                    iso
                } => {
                    let mut rng = thread_rng();
                    let z = rng.gen_range(-140f32, -120f32);
                    let entity = entities.create();
                    lazy_update.insert(entity, Isometry::new3d(iso.x, iso.y, z, iso.z));
                    lazy_update.insert(entity, Image(preloaded_images.big_star));
                    lazy_update.insert(entity, BigStarMarker);
                    lazy_update.insert(entity, Size(40f32));
                }
                InsertEvent::Planet {
                    iso
                } => {
                    let mut rng = thread_rng();
                    let z = -45.0;
                    let planets_num = preloaded_images.planets.len();
                    let planet_id = rng.gen_range(0, planets_num);
                    let nebula = entities.create();
                    lazy_update.insert(nebula, Isometry::new3d(iso.x, iso.y, z, iso.z));
                    lazy_update.insert(nebula, Image(preloaded_images.planets[planet_id]));
                    lazy_update.insert(nebula, PlanetMarker::default());
                    lazy_update.insert(nebula, Size(25f32));
                }
                InsertEvent::Wobble(wobble) => {
                    canvas.add_wobble(*wobble)
                }

            }
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
            WriteStorage<'a, Isometry>,
            WriteStorage<'a, Blast>,
            WriteStorage<'a, MultyLazer>,
            WriteStorage<'a, ShotGun>,
            WriteStorage<'a, Lifetime>,
            WriteStorage<'a, AsteroidMarker>,
            ReadStorage<'a, CharacterMarker>,
            ReadStorage<'a, ShipMarker>,
            WriteStorage<'a, Polygon>,
            WriteStorage<'a, StarsMarker>,
            WriteStorage<'a, BigStarMarker>,
            WriteStorage<'a, NebulaMarker>,
            WriteStorage<'a, PlanetMarker>,
            WriteStorage<'a, Shield>,
            WriteStorage<'a, Lifes>,
            ReadStorage<'a, ShipStats>,
            ReadStorage<'a, Coin>,
            ReadStorage<'a, Exp>,
            ReadStorage<'a, Health>,
            ReadStorage<'a, SideBulletCollectable>,
            ReadStorage<'a, SideBulletAbility>,
            ReadStorage<'a, DoubleCoinsCollectable>,
            ReadStorage<'a, DoubleCoinsAbility>,
            ReadStorage<'a, DoubleExpCollectable>,
            ReadStorage<'a, CollectableMarker>,
        ),
        ReadStorage<'a, Projectile>,
        WriteExpect<'a, MacroGame>,
        ReadExpect<'a, PreloadedImages>,
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
        WriteExpect<'a, BigStarGrid>,
        WriteExpect<'a, StarsGrid>,
        WriteExpect<'a, NebulaGrid>,
        WriteExpect<'a, PlanetGrid>,
        WriteExpect<'a, ScoreTable>,
        WriteExpect<'a, GlobalParams>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            (
                entities,
                mut isometries,
                blasts,
                mut multiple_lazers,
                mut shotguns,
                mut lifetimes,
                asteroid_markers,
                character_markers,
                ships,
                polygons,
                stars,
                big_star_markers,
                nebulas,
                planets,
                mut shields,
                mut lifes,
                ships_stats,
                coins,
                exps,
                healths,
                side_bullet_collectables,
                side_bullet_ability,
                double_coins_collectable,
                _double_coins_ability,
                double_exp_collectable,
                collectables,
            ),
            projectiles,
            mut macro_game,
            preloaded_images,
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
            mut big_star_grid,
            mut stars_grid,
            mut nebula_grid,
            mut planet_grid,
            mut score_table,
            mut global_params
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
        for (entity, lifetime) in (&entities, &mut lifetimes).join() {
            if lifetime.delete() {
                if side_bullet_ability.get(entity).is_some() {
                    if let Some(gun) = shotguns.get_mut(char_entity) {
                        gun.side_projectiles_number -= 1;
                    }
                    if let Some(multy_lazer) = multiple_lazers.get_mut(char_entity) {
                        multy_lazer.minus_side_lazers();
                    }
                }
                if let Some(blast) = blasts.get(entity) {
                    let owner = if let Some(projectile) = projectiles.get(entity) {
                        projectile.owner
                    } else {
                        entity
                    };
                    let position = isometries.get(entity).unwrap().0.translation.vector;
                    blast_explode(
                        Point2::new(position.x, position.y),
                        &mut insert_channel,
                        &mut sounds_channel,
                        &preloaded_sounds,
                        &preloaded_images,
                        blast.blast_radius
                    );
                    
                    // process_blast_damage
                    let blast_position = isometries.get(entity).unwrap().0.translation.vector;
                    for (entity, life, isometry) in (&entities, &mut lifes, &isometries).join() {
                        let position = isometry.0.translation.vector;
                        let is_character = entity == char_entity;
                        let is_asteroid = asteroid_markers.get(entity).is_some(); 
                        let affected = 
                            is_character && owner != char_entity ||
                            entity != char_entity && (owner == char_entity || is_asteroid);
                        if affected && (blast_position - position).norm() < blast.blast_radius {
                            if is_character {
                                global_params.damaged(DAMAGED_RED);
                            }
                            if process_damage(life, shields.get_mut(entity), blast.blast_damage) {
                                if is_asteroid {
                                    let polygon = polygons.get(entity).unwrap();
                                    asteroid_explode(
                                        Point2::new(position.x, position.y),
                                        &mut insert_channel,
                                        &mut sounds_channel,
                                        &preloaded_sounds,
                                        &preloaded_images,
                                        polygon.max_r
                                    );
                                    spawn_asteroids(
                                        isometry.0, 
                                        polygons.get(entity).unwrap(), 
                                        &mut insert_channel,
                                        None
                                    );
                                }
                                if is_character {
                                    // *app_state = AppState::Menu;
                                    to_menu(&mut app_state, &mut progress, &mut score_table);
                                }
                                // delete character
                                entities.delete(entity).unwrap();
                                // dbg!("dead");
                            }
                        }
                    }
                }
                entities.delete(entity).unwrap()
            }
        }
        for (entity, iso, _collectable) in (&entities, &mut isometries, &collectables).join() {
            let collectable_position = iso.0.translation.vector;
            if (pos3d - collectable_position).norm() < MAGNETO_RADIUS {
                let vel = 0.3 * (pos3d - collectable_position).normalize();
                iso.0.translation.vector += vel;
            }
            if (pos3d - collectable_position).norm() < COLLECT_RADIUS {
                let mut rng = thread_rng();
                if let Some(coin) = coins.get(entity) {
                    let coin_number = rng.gen_range(0, 2);
                    let coin_sound = if coin_number == 0 {
                        preloaded_sounds.coin
                    } else {
                        preloaded_sounds.coin2
                    };
                    sounds_channel.single_write(Sound(
                            coin_sound,
                            Point2::new(collectable_position.x, collectable_position.y)
                        )
                    );
                    progress.add_coins(coin.0);
                    progress.add_score(coin.0);
                    macro_game.coins += coin.0;
                }
                if let Some(exp) = exps.get(entity) {
                    sounds_channel.single_write(
                        Sound(
                            preloaded_sounds.exp,
                            Point2::new(collectable_position.x, collectable_position.y)
                        )
                    );
                    progress.add_score(3 * exp.0);
                    progress.add_exp(exp.0);
                }
                if let Some(health) = healths.get(entity) {
                    lifes.get_mut(char_entity).unwrap().0 += health.0;
                    // dbg!("wow");
                }
                if side_bullet_collectables.get(entity).is_some() {
                    insert_channel.single_write(InsertEvent::SideBulletAbility);
                    if let Some(gun) = shotguns.get_mut(char_entity) {
                        gun.side_projectiles_number += 1;
                    }
                    if let Some(multy_lazer) = multiple_lazers.get_mut(char_entity) {
                        multy_lazer.plus_side_lazers();
                    }
                }
                if double_coins_collectable.get(entity).is_some() {
                    insert_channel.single_write(InsertEvent::DoubleCoinsAbility)
                }
                if double_exp_collectable.get(entity).is_some() {
                    insert_channel.single_write(InsertEvent::DoubleExpAbility)
                }
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
            let poly = generate_convex_polygon(10, r);
            let spin = rng.gen_range(-1E-2, 1E-2);
            // let ball = ncollide2d::shape::Ball::new(r);
            let spawn_pos = spawn_position(character_position, PLAYER_AREA, ACTIVE_AREA);
            insert_channel.single_write(InsertEvent::Asteroid {
                iso: Point3::new(
                    spawn_pos.x,
                    spawn_pos.y,
                    0.0,
                ),
                velocity: initial_asteroid_velocity(),
                polygon: poly,
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
                snake: enemy.snake,
                rift: enemy.rift,
            }
        };
        for _ in 0..add_cnt {
            if wave.distribution.len() > 0 {
                let spawn_pos = spawn_position(character_position, PLAYER_AREA, ACTIVE_AREA);
                // TODO move from loop 
                let ships = &description.enemies;
                let ship_id = wave.distribution.choose_weighted(&mut rng, |item| item.1).unwrap().0;
                insert_channel.single_write(ships2insert(spawn_pos, ships[ship_id].clone()));
            }
        };
        if const_spawn {
            for kind in wave.const_distribution.iter() {
                // dbg!(kind);
                for _ in 0..kind.1 {
                    let spawn_pos = spawn_position(character_position, PLAYER_AREA, ACTIVE_AREA);
                    let ships = &description.enemies;
                    let ship_id = kind.0;
                    insert_channel.single_write(ships2insert(spawn_pos, ships[ship_id].clone()));
                }
            }
        }
        // TOOOOOO MANY COOPY PASTE %-P
        big_star_grid.grid.reset();
        for (isometry, _star) in (&isometries, &big_star_markers).join() {
            let position = isometry.0.translation.vector;
            let point = Point2::new(position.x, position.y);
            match big_star_grid.grid.update(point, true) {
                Ok(_) => (),
                Err(_) => ()
            }
        }

        for i in 0..big_star_grid.grid.size {
            for j in 0..big_star_grid.grid.size {
                let value = *big_star_grid.grid.get_cell_value(i, j);
                if !value {
                    let ((min_w, max_w), (min_h, max_h)) = big_star_grid.grid.get_rectangle(i, j);
                    let spawn_pos = spawn_in_rectangle(min_w, max_w, min_h, max_h);
                    let mut rng = thread_rng();
                    let angle = rng.gen_range(0.0, 2.0 * std::f32::consts::PI);
                    insert_channel.single_write(InsertEvent::BigStar {
                        iso: Point3::new(spawn_pos.x, spawn_pos.y, angle)
                    })
                }
            }
        }

        for (entity, isometry, _star) in (&entities, &isometries, &big_star_markers).join() {
            let pos3d = isometry.0.translation.vector;
            if  (pos3d.x - character_position.x).abs() > big_star_grid.grid.max_w ||
                (pos3d.y - character_position.y).abs() > big_star_grid.grid.max_h {
                entities.delete(entity).unwrap();
            }
        }        

        // TOOOOOO MANY COOPY PASTE %-P
        stars_grid.grid.reset();
        for (isometry, _stars) in (&isometries, &stars).join() {
            let position = isometry.0.translation.vector;
            let point = Point2::new(position.x, position.y);
            match stars_grid.grid.update(point, true) {
                Ok(_) => (),
                Err(_) => ()
            }
        }

        for i in 0..stars_grid.grid.size {
            for j in 0..stars_grid.grid.size {
                let value = *stars_grid.grid.get_cell_value(i, j);
                if !value {
                    let ((min_w, max_w), (min_h, max_h)) = stars_grid.grid.get_rectangle(i, j);
                    let spawn_pos = spawn_in_rectangle(min_w, max_w, min_h, max_h);
                    let mut rng = thread_rng();
                    let angle = rng.gen_range(0.0, 2.0 * std::f32::consts::PI);
                    insert_channel.single_write(InsertEvent::Stars {
                        iso: Point3::new(spawn_pos.x, spawn_pos.y, angle)
                    })
                }
            }
        }

        for (entity, isometry, _stars) in (&entities, &isometries, &stars).join() {
            let pos3d = isometry.0.translation.vector;
            if  (pos3d.x - character_position.x).abs() > stars_grid.grid.max_w ||
                (pos3d.y - character_position.y).abs() > stars_grid.grid.max_h {
                entities.delete(entity).unwrap();
            }
        }

        // TOOOOOO MANY COOPY PASTE %-P
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
                    let mut rng = thread_rng();
                    let angle = rng.gen_range(0.0, 2.0 * std::f32::consts::PI);
                    insert_channel.single_write(InsertEvent::Planet {
                        iso: Point3::new(spawn_pos.x, spawn_pos.y, angle)
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

        // TOOOOOO MANY COOPY PASTE %-P
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

fn ship_explode(
    ship_pos: Point2,
    insert_channel: &mut Write<EventChannel<InsertEvent>>,
    sounds_channel: &mut Write<EventChannel<Sound>>,
    preloaded_sounds: &ReadExpect<PreloadedSounds>,
) {
    insert_channel.single_write(
        InsertEvent::Exp{
            value: 50, 
            position: ship_pos
        }                    

    );
    let effect = InsertEvent::Explosion {
        position: ship_pos,
        num: 20,
        lifetime: Duration::from_secs(EXPLOSION_LIFETIME_SECS),
        with_animation: Some(1f32)
    };

    insert_channel.single_write(InsertEvent::Wobble(EXPLOSION_WOBBLE));
    insert_channel.single_write(effect);
    sounds_channel.single_write(Sound(
            preloaded_sounds.ship_explosion,
            ship_pos
        )
    );
}

fn bullet_contact(
    contact_pos: Point2,
    insert_channel: &mut Write<EventChannel<InsertEvent>>,
    _sounds_channel: &mut Write<EventChannel<Sound>>,
    _preloaded_sounds: &ReadExpect<PreloadedSounds>,
    preloaded_images: &ReadExpect<PreloadedImages>,
) {
    let effect = InsertEvent::Explosion {
        position: contact_pos,
        num: 2,
        lifetime: Duration::from_secs(EXPLOSION_LIFETIME_SECS),
        with_animation: None
    };
    let animation = InsertEvent::Animation {
        animation: preloaded_images.bullet_contact.clone(),
        lifetime: Duration::from_secs(BULLET_CONTACT_LIFETIME_SECS),
        pos: contact_pos,
        size: 1f32
    };
    insert_channel.single_write(animation);
    insert_channel.single_write(effect);
}

fn asteroid_explode(
    explode_position: Point2,
    insert_channel: &mut Write<EventChannel<InsertEvent>>,
    sounds_channel: &mut Write<EventChannel<Sound>>,
    preloaded_sounds: &ReadExpect<PreloadedSounds>,
    _preloaded_images: &ReadExpect<PreloadedImages>,
    size: f32,
) {
    sounds_channel.single_write(
        Sound(
            preloaded_sounds.asteroid_explosion,
            explode_position
        )
    );
    let effect = InsertEvent::Explosion {
        position: Point2::new(explode_position.x, explode_position.y),
        num: 10usize,
        lifetime: Duration::from_secs(EXPLOSION_LIFETIME_SECS),
        with_animation: Some(size)
    };
    insert_channel.single_write(effect);
    sounds_channel.single_write(Sound(
        preloaded_sounds.asteroid_explosion,
        Point2::new(explode_position.x, explode_position.y)
    ));

}

fn blast_explode(
    position: Point2,
    insert_channel: &mut Write<EventChannel<InsertEvent>>,
    sounds_channel: &mut Write<EventChannel<Sound>>,
    preloaded_sounds: &ReadExpect<PreloadedSounds>,
    preloaded_images: &ReadExpect<PreloadedImages>,
    blast_radius: f32,
) {
    insert_channel.single_write(
        InsertEvent::Animation {
            animation: preloaded_images.blast.clone(),
            lifetime: Duration::from_secs(BLAST_LIFETIME_SECS),
            pos: Point2::new(position.x, position.y),
            size: blast_radius
        }
    );
    sounds_channel.single_write(
        Sound(
            preloaded_sounds.blast,
            Point2::new(position.x, position.y)
        )
    );

}

pub fn to_menu(
    app_state: &mut Write<AppState>,
    progress: &mut Write<Progress>,
    score_table: &mut WriteExpect<ScoreTable>
) {
    **app_state = AppState::Menu;
    // score_table.0 = score_table.0.sort()
    score_table.0.push(progress.score);
    score_table.0.sort_by(|a, b| b.cmp(a));
    progress.score = 0;
}

fn reflect(d: Vector2, n: Vector2) -> Vector2 {
    d - 2.0 * (d.dot(&n)) * n
}

fn reflect_bullet(
    projectile: specs::Entity,
    physics_components: &ReadStorage<PhysicsComponent>,
    world: &mut Write<World<f32>>,
    reflection: &Reflection,
    normal: Vector2
) {
    let physics_component = physics_components.get(projectile).unwrap();
    let body = world.rigid_body_mut(physics_component.body_handle).unwrap();
    let position = body.position();
    let mut velocity = *body.velocity();
    let vel = reflection.speed * reflect(velocity.linear, normal.normalize()).normalize();
    *velocity.as_vector_mut() = Vector3::new(vel.x, vel.y, 0.0);
    let standart = Vector2::new(0.0, -1.0);
    let alpha = Rotation2::rotation_between(&standart, &velocity.linear).angle();
    let position = Isometry2::new(
        Vector2::new(position.translation.vector.x, position.translation.vector.y),
        alpha
    );
    body.set_position(position);
    body.set_velocity(velocity);

}


#[derive(Default)]
pub struct CollisionSystem {
    colliding_start_events: Vec<(CollisionObjectHandle, CollisionObjectHandle, Vector2)>,
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
        ReadStorage<'a, Reflection>,
        WriteStorage<'a, Lifes>,
        WriteStorage<'a, Shield>,
        ReadStorage<'a, Damage>,
        WriteStorage<'a, Polygon>,
        Write<'a, World<f32>>,
        Read<'a, BodiesMap>,
        Write<'a, EventChannel<InsertEvent>>,
        Write<'a, EventChannel<Sound>>,
        ReadExpect<'a, PreloadedSounds>,
        ReadExpect<'a, PreloadedImages>,
        Write<'a, Progress>,
        Write<'a, AppState>,
        WriteExpect<'a, ScoreTable>,
        WriteExpect<'a, GlobalParams>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            isometries,
            physics_components,
            asteroids,
            character_markers,
            ships,
            projectiles,
            reflections,
            mut lifes,
            mut shields,
            damages,
            polygons,
            mut world,
            bodies_map,
            mut insert_channel,
            mut sounds_channel,
            preloaded_sounds,
            preloaded_images,
            mut progress,
            mut app_state,
            mut score_table,
            mut global_params,
        ) = data;
        self.colliding_start_events.clear();
        for (collider1, collider2, _, manifold) in world.collider_world_mut().contact_pairs(false) {
            if let Some(tracked_contact) = manifold.deepest_contact() {
                let contact_normal = tracked_contact.contact.normal;
                self.colliding_start_events.push((collider1.handle(), collider2.handle(), *contact_normal));
            }
        }
        // for event in world.contact_events() {
        //     match event {
        //         &ncollide2d::events::ContactEvent::Started(
        //             collision_handle1,
        //             collision_handle2,
        //         ) => self
        //             .colliding_start_events
        //             .push((collision_handle1, collision_handle2)),
        //         _ => ()
        //     }
        // }
        for (handle1, handle2, normal) in self.colliding_start_events.iter() {
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
                if projectiles.get(entity2).is_some() {
                    let proj_pos = isometries.get(entity2).unwrap().0.translation.vector;
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
                        if let Some(reflection) = reflections.get(projectile) {
                            reflect_bullet(projectile, &physics_components, &mut world, &reflection, *normal);
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
                        with_animation: None
                    };
                    insert_channel.single_write(effect);
                    if character_markers.get(ship).is_some() {
                        sounds_channel.single_write(
                            Sound(
                                preloaded_sounds.collision,
                                Point2::new(position.x, position.y)
                            )
                        );
                    }
                }
                if asteroid_explosion {
                    insert_channel.single_write(InsertEvent::Wobble(EXPLOSION_WOBBLE));
                    let isometry = isometries.get(asteroid).unwrap().0;
                    let position = isometry.translation.vector;
                    let polygon = polygons.get(asteroid).unwrap();
                    asteroid_explode(
                        Point2::new(position.x, position.y),
                        &mut insert_channel,
                        &mut sounds_channel,
                        &preloaded_sounds,
                        &preloaded_images,
                        polygon.max_r
                    );
                    spawn_asteroids(
                        isometries.get(asteroid).unwrap().0, 
                        polygons.get(asteroid).unwrap(), 
                        &mut insert_channel,
                        bullet_position
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
                let projectile_pos = isometries.get(projectile).unwrap().0.translation.vector;
                let projectile_pos = Point2::new(projectile_pos.x, projectile_pos.y);
                let position = isometry.translation.vector;
                if character_markers.get(ship).is_some() {
                    global_params.damaged(DAMAGED_RED);
                    if process_damage(
                        lifes.get_mut(ship).unwrap(),
                        shields.get_mut(ship),
                        projectile_damage
                    ) {
                        // *app_state = AppState::Menu;
                        to_menu(
                            &mut app_state, 
                            &mut progress,
                            &mut score_table
                        );
                        // delete character
                        entities.delete(ship).unwrap();
                    } else {

                        bullet_contact(
                            projectile_pos,
                            &mut insert_channel,
                            &mut sounds_channel,
                            &preloaded_sounds,
                            &preloaded_images,
                        );
                    }
                    insert_channel.single_write(InsertEvent::Wobble(0.1f32));
                } else {
                    let ship_pos = Point2::new(position.x, position.y);
                    if process_damage(
                        lifes.get_mut(ship).unwrap(),
                        shields.get_mut(ship),
                        projectile_damage
                    ) {
                        ship_explode(
                            ship_pos,
                            &mut insert_channel,
                            &mut sounds_channel,
                            &preloaded_sounds,
                        );
                        entities.delete(ship).unwrap();
                    }
                    bullet_contact(
                        projectile_pos,
                        &mut insert_channel,
                        &mut sounds_channel,
                        &preloaded_sounds,
                        &preloaded_images,
                    );
                }
                // Kludge
                if projectile_damage != 0 {
                    if let Some(reflection) = reflections.get(projectile) {
                        reflect_bullet(projectile, &physics_components, &mut world, &reflection, *normal);
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
                        Point2::new(0f32, 0f32))
                    );
                    if process_damage(
                        lifes.get_mut(other_ship).unwrap(),
                        shields.get_mut(other_ship),
                        damages.get(character_ship).unwrap().0
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
                        damages.get(other_ship).unwrap().0
                    ) {
                        to_menu(&mut app_state, &mut progress, &mut score_table);
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
        WriteStorage<'a, ShotGun>,
        WriteStorage<'a, MultyLazer>,
        WriteStorage<'a, Cannon>,
        WriteStorage<'a, RocketGun>,
        WriteStorage<'a, EnemyMarker>,
        WriteStorage<'a, Charge>,
        ReadStorage<'a, CharacterMarker>,
        ReadStorage<'a, AI>,
        ReadStorage<'a, Chain>,
        Write<'a, World<f32>>,
        Write<'a, EventChannel<InsertEvent>>,
        Write<'a, BodiesMap>,
        Write<'a, EventChannel<Sound>>,
        ReadExpect<'a, PreloadedSounds>,
    );

    fn run(&mut self, data: Self::SystemData) {
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
            mut world,
            mut insert_channel,
            bodies_map,
            mut sounds_channel,
            preloaded_sounds,
        ) = data;
        let (character_entity, character_position, _) = (&entities, &isometries, &character_markers)
            .join().next().unwrap();
        let character_position = character_position.0.translation.vector;
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
            let follow_area = if let Some(multy_lazer) = multy_lazers.get(entity) {
                multy_lazer.first_distance() * 0.95
            } else {SCREEN_AREA};
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
                                    entity
                                );
                                insert_channel.iter_write(bullets.into_iter());
                                sounds_channel.single_write(
                                    Sound(
                                        preloaded_sounds.enemy_blaster,
                                        Point2::new(position.x, position.y),
                                    ),
                                )
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
                                sounds_channel.single_write(
                                    Sound(
                                        preloaded_sounds.enemy_shotgun,
                                        Point2::new(position.x, position.y)
                                    )
                                )
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
                                    entity
                                );
                                insert_channel.iter_write(bullets.into_iter());
                                sounds_channel.single_write(
                                    Sound(
                                        preloaded_sounds.enemy_shotgun,
                                        Point2::new(position.x, position.y)
                                    )
                                )
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
                        let speed = 0.1f32;
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
                                    let body = world.rigid_body_mut(physics_component.body_handle).unwrap();
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
