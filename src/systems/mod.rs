use std::cmp::Ordering::Equal;
use std::mem::swap;

use crate::types::{*};
use rand::prelude::*;

use sdl2::keyboard::Keycode;
use sdl2::TimerSubsystem;
use sdl2::sys::SDL_Finger;
use rand::distributions::{Bernoulli, Distribution};

use ncollide2d::shape::ShapeHandle;
use ncollide2d::world::CollisionGroups;
use ncollide2d::world::CollisionObjectHandle;
use ncollide2d::query::{Ray};
use nphysics2d::object::{Body, BodyStatus, BodyHandle};
use nphysics2d::world::World;
use nalgebra::geometry::{Translation};
use shrev::EventChannel;
use specs::prelude::*;
use specs::Join;

use crate::components::*;
use crate::geometry::{generate_convex_polygon, LightningPolygon, Polygon, TriangulateFromCenter, EPS};
use crate::gfx::{GeometryData, Engine, ParticlesData, Explosion,
                unproject_with_z, ortho_unproject, orthographic, orthographic_from_zero, get_view};
use crate::physics::CollisionId;
use crate::sound::{PreloadedSounds, SoundData};
use crate::gui::{Primitive, PrimitiveKind, Button, IngameUI, Text};

mod rendering;
pub use rendering::*;

const DAMPING_FACTOR: f32 = 0.98f32;
const THRUST_FORCE: f32 = 0.01f32;
const VELOCITY_MAX: f32 = 1f32;
const MAX_TORQUE: f32 = 10f32;
const LIGHT_RECTANGLE_SIZE: f32 = 20f32;
const PLAYER_BULLET_SPEED: f32 = 0.5;
const ENEMY_BULLET_SPEED: f32 = 0.3;

const SCREEN_AREA: f32 = 10f32;
// it's a kludge -- TODO redo with camera and screen sizes
// we will spwan new objects in ACTIVE_AREA but not in PLAYER_AREA
const PLAYER_AREA: f32 = 15f32;
const ACTIVE_AREA: f32 = 25f32;
// the same for NEBULAS
const NEBULA_PLAYER_AREA: f32 = 90f32;
const NEBULA_ACTIVE_AREA: f32 = 110f32;
const NEBULA_MIN_NUMBER: usize = 20;

const ASTEROIDS_MIN_NUMBER: usize = 10;
const ASTEROID_MAX_RADIUS: f32 = 2.2f32;
const ASTEROID_MIN_RADIUS: f32 = 0.5;
const ASTEROID_INERTIA: f32 = 2f32;

const AI_COLLISION_DISTANCE: f32 = 3.5f32;

const SHIPS_NUMBER: usize = 1 + 3; // character's ship counts
pub const dt: f32 =  1f32 / 60f32;

pub enum EntityType {
    Player,
    Enemy,
}

pub enum InsertEvent {
    Asteroid {
        iso: Point3,
        velocity: Velocity2,
        polygon: Polygon,
        light_shape: Geometry,
        spin: f32,
    },
    Ship {
        iso: Point3,
        light_shape: Geometry,
        spin: f32,
        kind: AIType,
        image: Image
    },
    Bullet {
        kind: EntityType,
        iso: Point3,
        velocity: Point2,
        damage: usize,
        owner: specs::Entity,
    },
    // Lazer {
    //     kind: EntityType,
    //     iso: Isometry2,
    //     damage: usize,
    //     distance: f32,
    //     owner: specs::Entity
    // },
    Explosion {
        position: Point2,
        num: usize,
        lifetime: usize,
    },
    Engine {
        position: Point2,
        num: usize,
        attached: AttachPosition
    },
    Nebula {
        iso: Point3
    }
}

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
        ReadStorage<'a, AsteroidMarker>,
        Write<'a, World<f32>>,
        Write<'a, BodiesMap>,
        Read<'a, AppState>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            mut isometries, 
            mut velocities, 
            physics, 
            character_markers,
            asteroid_markers,
            mut world, 
            _bodies_map, 
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
        for (isometry, ()) in (&mut isometries, !&physics).join() {
            let char_vec = character_position.translation.vector;
            let diff = Vector3::new(char_vec.x, char_vec.y, 0f32)  - character_prev_position.0.translation.vector;
            isometry.0.translation.vector -= diff;
        }

        for (isometry, velocity, physics_component) in
            (&mut isometries, &mut velocities, &physics).join()
        {
            let mut body = world.rigid_body_mut(physics_component.body_handle).unwrap();
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
        WriteExpect<'a, ThreadPin<TimerSubsystem>>,
        Write<'a, EventChannel<Sound>>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (sounds, _timer, sounds_channel) = data;
        for s in sounds_channel.read(&mut self.reader) {
            sdl2::mixer::Channel::all().play(&sounds.get(s.0).unwrap().0, 0).unwrap();
        }
        // eprintln!("SOUNDS");
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
            _character_markers,
            asteroid_markers,
            ship_markers,
            mut world,
        ) = data;
        for (physics_component, _) in (&physics, !&asteroid_markers).join() {
            let body = world.rigid_body_mut(physics_component.body_handle).unwrap();
            let mut velocity = *body.velocity();
            *velocity.as_vector_mut() *= DAMPING_FACTOR;
            body.set_velocity(velocity);
            body.activate();
        }
        // activate asteroid bodyes
        for (physics_component, _asteroid) in (&physics, &asteroid_markers).join() {
            let mut body = world.rigid_body_mut(physics_component.body_handle).unwrap();
            body.activate();
        }
        for (_isometry, _velocity, physics_component, spin, _ship) in (
            &mut isometries,
            &mut velocities,
            &physics,
            &spins,
            &ship_markers,
        )
            .join()
        {
            let body = world.rigid_body_mut(physics_component.body_handle).unwrap();
            body.set_angular_velocity(spin.0);
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
            WriteStorage<'a, Image>,
            WriteStorage<'a, Blaster>,
            WriteStorage<'a, ShotGun>,
            WriteStorage<'a, Lazer>,
            WriteStorage<'a, Projectile>,
            WriteStorage<'a, Geometry>,
            WriteStorage<'a, Lifetime>,
            WriteStorage<'a, Size>,
            WriteStorage<'a, Lifes>,
            WriteStorage<'a, Shield>,
            ReadStorage<'a, CharacterMarker>,
            ReadStorage<'a, ShipMarker>,
        ),
        Read<'a, EventChannel<Keycode>>,
        Read<'a, Mouse>,
        ReadExpect<'a, PreloadedImages>,
        Write<'a, EventChannel<Sound>>,
        ReadExpect<'a, PreloadedSounds>,
        Write<'a, World<f32>>,
        Write<'a, BodiesMap>,
        Write<'a, EventChannel<InsertEvent>>,
        Write<'a, Progress>,
        Read<'a, PlayerStats>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            (
                entities,
                isometries,
                mut velocities,
                physics,
                mut spins,
                _images,
                mut blasters,
                mut shotguns,
                mut lazers,
                _projectiles,
                _geometries,
                _lifetimes,
                _sizes,
                mut lifes,
                mut shields,
                character_markers,
                ships,
            ),
            keys_channel,
            mouse_state,
            _preloaded_images,
            mut sounds_channel,
            preloaded_sounds,
            mut world,
            bodies_map,
            mut insert_channel,
            mut progress,
            player_stats,
        ) = data;
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
                let player_torque = dt
                    * calculate_player_ship_spin_for_aim(
                        Vector2::new(mouse_state.x, mouse_state.y)
                            - Vector2::new(iso.0.translation.vector.x, iso.0.translation.vector.y),
                        iso.rotation(),
                        spin.0,
                    );
                spin.0 += player_torque.max(-MAX_TORQUE).min(MAX_TORQUE);
            }
            let character = character.unwrap();
            let (character_isometry, mut character_velocity) = {
                let character_body = world
                    .rigid_body(physics.get(character).unwrap().body_handle)
                    .unwrap();
                (*character_body.position(), *character_body.velocity())
            };
            if let Some(lazer) = lazers.get_mut(character) {
                if mouse_state.left {
                    lazer.active = true;
                    let position = character_isometry.translation.vector;
                    let pos = Point2::new(position.x, position.y);
                    let dir = character_isometry * Vector2::new(0f32, -1f32);
                    let ray = Ray::new(pos, dir);
                    let mut gun_groups = CollisionGroups::new();
                    let (min_d, closest_body) = get_min_dist(
                        &mut world, 
                        ray, 
                        get_collision_groups(&EntityType::Player)
                    );
                    if min_d < lazer.distance {
                        // dbg!("bang bang you're dead");
                        lazer.current_distance = min_d;
                        if let Some(target_entity) = bodies_map.get(&closest_body.unwrap()) {
                            if let Some(_) = lifes.get(*target_entity) {
                                let explosion_size = 1;
                                if process_damage(
                                    lifes.get_mut(*target_entity).unwrap(),
                                    shields.get_mut(*target_entity),
                                    lazer.damage
                                ) {
                                    progress.experience += 50usize;
                                    entities.delete(*target_entity).unwrap();
                                }
                                let effect_position = position + dir * min_d;
                                let effect = InsertEvent::Explosion {
                                    position: Point2::new(effect_position.x, effect_position.y),
                                    num: explosion_size,
                                    lifetime: 50usize,
                                };
                                insert_channel.single_write(effect);
                            }
                        }
                    } else {
                        lazer.current_distance = lazer.distance
                    }

                } else {
                    lazer.active = false;
                }
            }
            if mouse_state.left {
                if let Some(blaster) = blasters.get_mut(character) {
                    if blaster.shoot() {
                        let isometry = *isometries.get(character).unwrap();
                        let position = isometry.0.translation.vector;
                        let direction = isometry.0 * Vector3::new(0f32, -1f32, 0f32);
                        let velocity_rel = player_stats.bullet_speed * direction;
                        let char_velocity = velocities.get(character).unwrap();
                        let projectile_velocity = Velocity::new(
                            char_velocity.0.x + velocity_rel.x,
                            char_velocity.0.y + velocity_rel.y,
                        ) ;
                        sounds_channel.single_write(Sound(preloaded_sounds.shot));
                        insert_channel.single_write(InsertEvent::Bullet {
                            kind: EntityType::Player,
                            iso: Point3::new(position.x, position.y, isometry.0.rotation.euler_angles().2),
                            velocity: Point2::new(projectile_velocity.0.x, projectile_velocity.0.y),
                            damage: blaster.bullets_damage,
                            owner: character,
                        });
                    }
                }
                if let Some(shotgun) = shotguns.get_mut(character) {
                    let isometry = *isometries.get(character).unwrap();
                    if shotgun.shoot() {
                        let position = isometry.0.translation.vector;
                        {
                            let direction = isometry.0 * Vector3::new(0f32, -1f32, 0f32);
                            let velocity_rel = player_stats.bullet_speed * direction;
                            let char_velocity = velocities.get(character).unwrap();
                            let projectile_velocity = Velocity::new(
                                char_velocity.0.x + velocity_rel.x,
                                char_velocity.0.y + velocity_rel.y,
                            ) ;
                            sounds_channel.single_write(Sound(preloaded_sounds.shot));
                            insert_channel.single_write(InsertEvent::Bullet {
                                kind: EntityType::Player,
                                iso: Point3::new(position.x, position.y, isometry.0.rotation.euler_angles().2),
                                velocity: Point2::new(projectile_velocity.0.x, projectile_velocity.0.y),
                                damage: shotgun.bullets_damage,
                                owner: character,
                            });
                        }
                        for i in 1..=shotgun.side_projectiles_number {
                            for j in 0i32..=1 {
                                let sign = j * 2 - 1;
                                let shift = shotgun.angle_shift * i as f32 * sign as f32;
                                let rotation = Rotation3::new(Vector3::new(0f32, 0f32, shift));
                                let direction = isometry.0 * (rotation * Vector3::new(0f32, -1f32, 0f32));
                                let velocity_rel = player_stats.bullet_speed * direction;
                                let char_velocity = velocities.get(character).unwrap();
                                let projectile_velocity = Velocity::new(
                                    char_velocity.0.x + velocity_rel.x,
                                    char_velocity.0.y + velocity_rel.y,
                                ) ;
                                insert_channel.single_write(InsertEvent::Bullet {
                                    kind: EntityType::Player,
                                    iso: Point3::new(
                                        position.x, 
                                        position.y, 
                                        isometry.0.rotation.euler_angles().2 + shift
                                    ),
                                    velocity: Point2::new(projectile_velocity.0.x, projectile_velocity.0.y),
                                    damage: shotgun.bullets_damage,
                                    owner: character,
                                });
                            }
                        }
                    }
                }
            }
            for key in keys_channel.read(&mut self.reader) {
                match key {
                    Keycode::W => {
                        let thrust = player_stats.thrust_force * Vector3::new(0.0, 1.0, 0.0);
                        *character_velocity.as_vector_mut() += thrust;
                    }
                    Keycode::S => {
                        let thrust = player_stats.thrust_force * Vector3::new(0.0, -1.0, 0.0);
                        *character_velocity.as_vector_mut() += thrust;
                    }
                    Keycode::A => {
                        let thrust = player_stats.thrust_force * Vector3::new(-1.0, 0.0, 0.0);
                        *character_velocity.as_vector_mut() += thrust;
                    }
                    Keycode::D => {
                        let thrust = player_stats.thrust_force * Vector3::new(1.0, 0.0, 0.0);
                        *character_velocity.as_vector_mut() += thrust;
                    }
                    _ => ()
                    
                }
            }
            if mouse_state.right {
                let rotation = isometries.get(character).unwrap().0.rotation;
                let _vel = velocities.get_mut(character).unwrap();
                let thrust = player_stats.thrust_force * (rotation * Vector3::new(0.0, -1.0, 0.0));
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
            WriteStorage<'a, Damage>,
            WriteStorage<'a, Lifes>,
            WriteStorage<'a, Shield>,
            WriteStorage<'a, Lifetime>,
            WriteStorage<'a, AIType>,
            WriteStorage<'a, AsteroidMarker>,
            WriteStorage<'a, EnemyMarker>,
            WriteStorage<'a, ShipMarker>,
            WriteStorage<'a, Image>,
            WriteStorage<'a, Size>,
            WriteStorage<'a, Polygon>,
            WriteStorage<'a, Projectile>,
            WriteStorage<'a, NebulaMarker>,
            WriteStorage<'a, AttachPosition>,
            WriteStorage<'a, LightMarker>,
            WriteStorage<'a, ThreadPin<ParticlesData>>,
        ),
        ReadExpect<'a, ThreadPin<red::GL>>,
        Write<'a, Stat>,
        WriteExpect<'a, PreloadedImages>,
        Write<'a, World<f32>>,
        Write<'a, BodiesMap>,
        Read<'a, EventChannel<InsertEvent>>,
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
                mut damages,
                mut lifes,
                mut shields,
                mut lifetimes,
                mut ai_types,
                mut asteroid_markers,
                mut enemies,
                mut ships,
                mut images,
                mut sizes,
                mut polygons,
                mut projectiles,
                mut nebulas,
                mut attach_positions,
                mut lights,
                mut particles_datas,
            ),
            gl,
            _stat,
            preloaded_images,
            mut world,
            mut bodies_map,
            insert_channel,
        ) = data;
        for insert in insert_channel.read(&mut self.reader) {
            match insert {
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
                        .with(*light_shape, &mut geometries)
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
                    image,
                } => {
                    let size = 0.4f32;

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

                    let enemy = entities
                        .build_entity()
                        .with(Isometry::new(iso.x, iso.y, iso.z), &mut isometries)
                        .with(Velocity::new(0f32, 0f32), &mut velocities)
                        .with(EnemyMarker::default(), &mut enemies)
                        .with(ShipMarker::default(), &mut ships)
                        .with(*image, &mut images)
                        .with(Lifes(ENEMY_MAX_LIFES), &mut lifes)
                        .with(Shield(ENEMY_MAX_SHIELDS), &mut shields)
                        .with(*kind, &mut ai_types)
                        .with(Blaster::new(50usize, 10usize), &mut blasters)
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
                        {
                    let _light = entities
                        .build_entity()
                        .with(Isometry::new(0f32, 0f32, 0f32), &mut isometries)
                        .with(Velocity::new(0f32, 0f32), &mut velocities)
                        .with(Spin::default(), &mut spins)
                        .with(AttachPosition(enemy), &mut attach_positions)
                        .with(Image(preloaded_images.light_sea), &mut images)
                        .with(Size(1f32), &mut sizes)
                        .with(LightMarker, &mut lights)
                        .build();
                    }
                }
                InsertEvent::Bullet {
                    kind,
                    iso,
                    velocity,
                    damage,
                    owner,
                } => {
                    let r = 0.1;
                    let image = match kind {
                        EntityType::Player => preloaded_images.projectile,
                        EntityType::Enemy => preloaded_images.enemy_projectile
                    };
                    let bullet = entities
                        .build_entity()
                        .with(Damage(*damage), &mut damages)
                        .with(Velocity::new(velocity.x, velocity.y), &mut velocities)
                        .with(Isometry::new(iso.x, iso.y, iso.z), &mut isometries)
                        .with(Image(image), &mut images)
                        .with(Spin::default(), &mut spins)
                        .with(Projectile { owner: *owner }, &mut projectiles)
                        .with(Lifetime::new(100usize), &mut lifetimes)
                        .with(Size(r), &mut sizes)
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
                InsertEvent::Explosion {
                    position,
                    num,
                    lifetime,
                } => {
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
                InsertEvent::Engine {
                    position,
                    num,
                    attached
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
                    let z = rng.gen_range(-70f32, -40f32);
                    let nebulas_num = preloaded_images.nebulas.len();
                    let nebula_id = rng.gen_range(0, nebulas_num);
                    let nebula = entities
                        .build_entity()
                        .with(Isometry::new3d(iso.x, iso.y, z, iso.z), &mut isometries)
                        .with(Image(preloaded_images.nebulas[nebula_id]), &mut images)
                        .with(NebulaMarker::default(), &mut nebulas)
                        .with(Size(40f32), &mut sizes)
                        .build();
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
        Entities<'a>,
        WriteStorage<'a, PhysicsComponent>,
        WriteStorage<'a, Geometry>,
        WriteStorage<'a, Isometry>,
        WriteStorage<'a, Velocity>,
        WriteStorage<'a, Spin>,
        WriteStorage<'a, Blaster>,
        WriteStorage<'a, ShotGun>,
        WriteStorage<'a, Lifetime>,
        WriteStorage<'a, AsteroidMarker>,
        ReadStorage<'a, CharacterMarker>,
        ReadStorage<'a, ShipMarker>,
        WriteStorage<'a, Image>,
        WriteStorage<'a, Size>,
        WriteStorage<'a, Polygon>,
        WriteStorage<'a, NebulaMarker>,
        Write<'a, Stat>,
        WriteExpect<'a, PreloadedImages>,
        Write<'a, World<f32>>,
        Write<'a, BodiesMap>,
        Write<'a, EventChannel<InsertEvent>>,
        Write<'a, Progress>,
        Write<'a, AppState>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            _physics,
            _geometries,
            isometries,
            _velocities,
            _spins,
            mut blasters,
            mut shotguns,
            mut lifetimes,
            asteroid_markers,
            character_markers,
            ships,
            _images,
            _sizes,
            _polygons,
            nebulas,
            _stat,
            preloaded_images,
            _world,
            _bodies_map,
            mut insert_channel,
            mut progress,
            mut app_state
        ) = data;
        if progress.experience >= progress.current_max_experience() {
            progress.level_up();
            *app_state = AppState::Play(PlayState::Upgrade);
        }
        let (char_isometry, _char) = (&isometries, &character_markers).join().next().unwrap();
        let pos3d = char_isometry.0.translation.vector;
        let character_position = Point2::new(pos3d.x, pos3d.y);
        for gun in (&mut blasters).join() {
            gun.update()
        }
        for shotgun in (&mut shotguns).join() {
            shotgun.update()
        }
        for (entity, lifetime) in (&entities, &mut lifetimes).join() {
            lifetime.update();
            if lifetime.delete() {
                entities.delete(entity).unwrap()
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
            let asteroid_shape = Geometry::Circle { radius: r };
            let poly = generate_convex_polygon(10, r);
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
        let add_cnt = if SHIPS_NUMBER > cnt {
            SHIPS_NUMBER - cnt
        } else {
            0
        };
        let r = 1f32;
        let ship_shape = Geometry::Circle { radius: r };
        for _ in 0..add_cnt {
            let spawn_pos = spawn_position(character_position, PLAYER_AREA, ACTIVE_AREA);
            let d = Bernoulli::new(0.5);
            let v = d.sample(&mut rand::thread_rng());
            if v {
                insert_channel.single_write(InsertEvent::Ship {
                    iso: Point3::new(spawn_pos.x, spawn_pos.y, 0f32),
                    light_shape: ship_shape,
                    spin: 0f32,
                    kind: AIType::Kamikadze,
                    image: Image(preloaded_images.enemy2)
                })
            } else {
                insert_channel.single_write(InsertEvent::Ship {
                    iso: Point3::new(spawn_pos.x, spawn_pos.y, 0f32),
                    light_shape: ship_shape,
                    spin: 0f32,
                    kind: AIType::ShootAndFollow,
                    image: Image(preloaded_images.enemy)
                })
            }
        };
        // for _ in 0..add_cnt {
        //     let spawn_pos = spawn_position(character_position, PLAYER_AREA, ACTIVE_AREA);
        //     insert_channel.single_write(InsertEvent::Ship {
        //         iso: Point3::new(spawn_pos.x, spawn_pos.y, 0f32),
        //         light_shape: ship_shape,
        //         spin: 0f32,
        //         kind: AIType::ShootAndFollow,
        //         image: Image(preloaded_images.enemy)
        //     })
        // }
        let cnt = nebulas.count();
        let add_cnt = if NEBULA_MIN_NUMBER > cnt {
            NEBULA_MIN_NUMBER - cnt
        } else {
            0
        };
        for _ in 0..add_cnt {
            let spawn_pos = spawn_position(character_position, NEBULA_PLAYER_AREA, NEBULA_ACTIVE_AREA);
            insert_channel.single_write(InsertEvent::Nebula {
                iso: Point3::new(spawn_pos.x, spawn_pos.y, 0f32)
            })
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
        for (entity, isometry, _nebula) in (&entities, &isometries, &nebulas).join() {
            let pos3d = isometry.0.translation.vector;
            if !is_active(character_position, Point2::new(pos3d.x, pos3d.y), NEBULA_ACTIVE_AREA) {
                entities.delete(entity).unwrap();
            }
        }
    }
}

/// returns true if killed
fn process_damage(life: &mut Lifes, mut shield: Option<&mut Shield>, projectile_damage: usize) -> bool {
    match shield {
        Some(ref mut shield) if shield.0 > 0usize => {
            if shield.0 > projectile_damage {
                shield.0 -= projectile_damage
            } else {
                shield.0 = 0
            }
        }
        _ => {
            if life.0 > projectile_damage {
                life.0 -= projectile_damage
            } else {
                return true;
            }
        }
    };
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
        // WriteStorage<'a, Velocity>,
        ReadStorage<'a, PhysicsComponent>,
        // WriteStorage<'a, Spin>,
        // ReadStorage<'a, Geometry>,
        // ReadStorage<'a, Projectile>,
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
            mut progress
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
                    entities.delete(projectile).unwrap();
                    let projectile_damage = damages.get(projectile).unwrap().0;
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
                    };
                    insert_channel.single_write(effect);
                }
                if asteroid_explosion {
                    let isometry = isometries.get(asteroid).unwrap().0;
                    let position = isometry.translation.vector;
                    let polygon = polygons.get(asteroid).unwrap();
                    let new_polygons = polygon.deconstruct();
                    let effect = InsertEvent::Explosion {
                        position: Point2::new(position.x, position.y),
                        num: 10usize,
                        lifetime: 20usize,
                    };
                    insert_channel.single_write(effect);
                    sounds_channel.single_write(Sound(preloaded_sounds.explosion));
                    if new_polygons.len() == 1 {

                    } else {
                        for poly in new_polygons.iter() {
                            let r = poly.min_r;
                            let asteroid_shape = Geometry::Circle { radius: r };
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
                    }
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
                    let shield = shields.get_mut(ship).unwrap();
                    let lifes = lifes.get_mut(ship).unwrap();
                    if shield.0 > 0 {
                        shield.0 -= projectile_damage
                    } else {
                        lifes.0 -= projectile_damage
                    }
                } else {
                    let mut explosion_size = 2usize;
                    if process_damage(
                        lifes.get_mut(ship).unwrap(),
                        shields.get_mut(ship),
                        projectile_damage
                    ) {
                        progress.experience += 50usize;
                        entities.delete(ship).unwrap();
                        explosion_size = 20;
                    }
                    let effect = InsertEvent::Explosion {
                        position: Point2::new(position.x, position.y),
                        num: explosion_size,
                        lifetime: 50usize,
                    };
                    insert_channel.single_write(effect);
                }
                entities.delete(projectile).unwrap();
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
                    match damages.get(other_ship) {
                        Some(damage) => {
                            lifes.get_mut(character_ship).unwrap().0 -= damage.0;
                        }
                        None => ()
                    }
                    entities.delete(other_ship).unwrap();
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
                inter.toi > EPS  {
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
        WriteStorage<'a, EnemyMarker>,
        ReadStorage<'a, CharacterMarker>,
        ReadStorage<'a, AIType>,
        Write<'a, Stat>,
        Write<'a, World<f32>>,
        Write<'a, EventChannel<InsertEvent>>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            isometries,
            mut velocities,
            physics,
            mut spins,
            mut blasters,
            enemies,
            character_markers,
            ai_types,
            _stat,
            mut world,
            mut insert_channel,
        ) = data;
        let _rng = thread_rng();
        let character_position = {
            let mut res = None;
            for (iso, _) in (&isometries, &character_markers).join() {
                res = Some(iso.0.translation.vector)
            }
            res.unwrap()
        };
        for (entity, iso, vel, physics_component, spin, _enemy, ai_type) in (
            &entities,
            &isometries,
            &mut velocities,
            &physics,
            &mut spins,
            &enemies,
            &ai_types
        )
            .join()
        {
            match ai_type {
                AIType::ShootAndFollow => {
                    let gun = blasters.get_mut(entity).unwrap();
                    let isometry = iso.0;
                    let position = isometry.translation.vector;
                    let ship_torque = dt
                        * calculate_player_ship_spin_for_aim(
                            Vector2::new(character_position.x, character_position.y)
                                - Vector2::new(position.x, position.y),
                            iso.rotation(),
                            spin.0,
                        );
                    spin.0 += ship_torque.max(-MAX_TORQUE).min(MAX_TORQUE);
                    let speed = 0.1f32;
                    let diff = character_position - position;
                    let velocity_rel = ENEMY_BULLET_SPEED * diff.normalize();
                    let projectile_velocity =
                        Velocity::new(vel.0.x + velocity_rel.x, vel.0.y + velocity_rel.y);
                    if diff.norm() > SCREEN_AREA {
                        let dir = Vector2::new(diff.x, diff.y).normalize();
                        let pos = Point2::new(position.x, position.y);
                        let ray = Ray::new(pos, dir);
                        let mut all_groups = CollisionGroups::new();
                        let ai_vel = if get_min_dist(&mut world, ray, all_groups).0 < AI_COLLISION_DISTANCE {
                            let rays_half_num = 3;
                            let step = std::f32::consts::PI / 2.0 / rays_half_num as f32;
                            let mut result_dir = Vector2::new(0f32, 0f32);
                            for i in 1..=rays_half_num {
                                let rotation1 = Rotation2::new(step * i as f32);
                                let rotation2 = Rotation2::new(-step * i as f32);
                                let dir1 = rotation1 * dir;
                                let dir2 = rotation2 * dir;
                                let ray1 = Ray::new(pos, dir1);
                                let ray2 = Ray::new(pos, dir2);
                                if get_min_dist(&mut world, ray1, all_groups).0 > AI_COLLISION_DISTANCE {
                                    result_dir = dir1;
                                    // break;
                                }
                                if get_min_dist(&mut world, ray2, all_groups).0 > AI_COLLISION_DISTANCE {
                                    result_dir = dir2;
                                    // break;
                                }
                            }
                            speed * result_dir
                        } else {
                            speed * dir
                        };
                        *vel = Velocity::new(ai_vel.x, ai_vel.y);
                    } else {
                        let vel_vec = DAMPING_FACTOR * vel.0;
                        *vel = Velocity::new(vel_vec.x, vel_vec.y);
                    }
                    let body = world.rigid_body_mut(physics_component.body_handle).unwrap();
                    let mut velocity = *body.velocity();
                    *velocity.as_vector_mut() = Vector3::new(vel.0.x, vel.0.y, spin.0);
                    body.set_velocity(velocity);
                    if diff.norm() < SCREEN_AREA && gun.shoot() {
                        insert_channel.single_write(InsertEvent::Bullet {
                            kind: EntityType::Enemy,
                            iso: Point3::new(position.x, position.y, isometry.rotation.euler_angles().2),
                            velocity: Point2::new(projectile_velocity.0.x, projectile_velocity.0.y),
                            damage: gun.bullets_damage,
                            owner: entity,
                        });
                    }
                }
                AIType::Kamikadze => {
                    let isometry = iso.0;
                    let position = isometry.translation.vector;
                    let ship_torque = dt
                        * calculate_player_ship_spin_for_aim(
                            Vector2::new(character_position.x, character_position.y)
                                - Vector2::new(position.x, position.y),
                            iso.rotation(),
                            spin.0,
                        );
                    spin.0 += ship_torque.max(-MAX_TORQUE).min(MAX_TORQUE);
                    let speed = 0.1f32;
                    let diff = character_position - position;
                    let vel_vec = DAMPING_FACTOR * vel.0;
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
