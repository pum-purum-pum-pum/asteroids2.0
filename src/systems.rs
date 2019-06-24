use std::cmp::Ordering::Equal;
use std::mem::swap;

use al::prelude::*;
use astro_lib as al;
use rand::prelude::*;

use glium;
use glium::Surface;
use sdl2::keyboard::Keycode;
use sdl2::TimerSubsystem;

use ncollide2d::shape::ShapeHandle;
use ncollide2d::world::CollisionGroups;
use ncollide2d::world::CollisionObjectHandle;
use nphysics2d::object::{Body, BodyStatus};
use nphysics2d::world::World;
use shrev::EventChannel;
use specs::prelude::*;
use specs::Join;

use crate::components::*;
use crate::geometry::{generate_convex_polygon, LightningPolygon, Polygon, TriangulateFromCenter};
use crate::gfx::{Explosion, Engine, GeometryData, ParticlesData};
use crate::physics::CollisionId;
use crate::sound::{PreloadedSounds, SoundData};

const DAMPING_FACTOR: f32 = 0.98f32;
const THRUST_FORCE: f32 = 0.01f32;
const VELOCITY_MAX: f32 = 1f32;
const MAX_TORQUE: f32 = 10f32;
const LIGHT_RECTANGLE_SIZE: f32 = 20f32;
const PLAYER_BULLET_SPEED: f32 = 0.5;
const ENEMY_BULLET_SPEED: f32 = 0.3;

const _SCREEN_AREA: f32 = 10f32;
const PLAYER_AREA: f32 = 15f32;
const ACTIVE_AREA: f32 = 25f32;
// we will spwan new objects in ACTIVE_AREA but not in PLAYER_AREA

const ASTEROIDS_MIN_NUMBER: usize = 5usize;
const SHIPS_NUMBER: usize = 1usize;

pub enum EntityType {
    Player,
    Enemy,
}

pub enum InsertEvent {
    Asteroid {
        iso: Point3,
        polygon: Polygon,
        light_shape: Geometry,
        spin: f32,
    },
    Ship {
        iso: Point3,
        light_shape: Geometry,
        spin: f32,
    },
    Bullet {
        kind: EntityType,
        iso: Point3,
        velocity: Point2,
        owner: specs::Entity,
    },
    Explosion {
        position: Point2,
        num: usize,
        lifetime: usize,
    },
    Engine {
        position: Point2,
        num: usize,
        attached: AttachPosition
    }
}

pub fn spawn_position(char_pos: Point2) -> Point2 {
    let mut rng = thread_rng();
    loop {
        let x = rng.gen_range(-ACTIVE_AREA, ACTIVE_AREA);
        let y = rng.gen_range(-ACTIVE_AREA, ACTIVE_AREA);
        if x.abs() >= PLAYER_AREA || y.abs() >= PLAYER_AREA {
            return Point2::new(char_pos.x + x, char_pos.y + y);
        }
    }
}

pub fn is_active(character_position: Point2, point: Point2) -> bool {
    (point.x - character_position.x).abs() < ACTIVE_AREA
        && (point.y - character_position.y).abs() < ACTIVE_AREA
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

#[derive(Default)]
pub struct RenderingSystem;

impl<'a> System<'a> for RenderingSystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Isometry>,
        ReadStorage<'a, Velocity>,
        ReadStorage<'a, PhysicsComponent>,
        ReadStorage<'a, CharacterMarker>,
        ReadStorage<'a, ShipMarker>,
        ReadStorage<'a, AsteroidMarker>,
        ReadStorage<'a, LightMarker>,
        ReadStorage<'a, Projectile>,
        ReadStorage<'a, ThreadPin<ImageData>>,
        ReadStorage<'a, Image>,
        ReadStorage<'a, Geometry>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, Polygon>,
        WriteStorage<'a, ThreadPin<ParticlesData>>,
        WriteExpect<'a, SDLDisplay>,
        WriteExpect<'a, Canvas<'static>>,
        ReadExpect<'a, PreloadedParticles>,
        Read<'a, World<f32>>,
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
            projectiles,
            image_datas,
            image_ids,
            geometries,
            sizes,
            polygons,
            mut particles_datas,
            display,
            mut canvas,
            preloaded_particles,
            world,
        ) = data;
        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        target.clear_stencil(0i32);
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
        // @vlad TODO rewrite it with screen borders
        let rectangle = (
            char_pos.x - LIGHT_RECTANGLE_SIZE,
            char_pos.y - LIGHT_RECTANGLE_SIZE,
            char_pos.x + LIGHT_RECTANGLE_SIZE,
            char_pos.y + LIGHT_RECTANGLE_SIZE,
        );
        let mut light_poly = LightningPolygon::new_rectangle(
            rectangle.0,
            rectangle.1,
            rectangle.2,
            rectangle.3,
            Point2::new(char_pos.x, char_pos.y),
        );
        // TODO fix lights to be able to use without sorting
        let mut data = (&entities, &isometries, &geometries, &asteroid_markers)
            .join()
            .collect::<Vec<_>>(); // TODO move variable to field  to avoid allocations
        let distance = |a: &Isometry| (char_pos - a.0.translation.vector).norm();
        data.sort_by(|&a, &b| (distance(b.1).partial_cmp(&distance(a.1)).unwrap_or(Equal)));
        // UNCOMMENT TO ADD LIGHTS
        for (_entity, iso, geom, _) in data.iter() {
            let pos = Point2::new(iso.0.translation.vector.x, iso.0.translation.vector.y);
            if pos.x > rectangle.0
                && pos.x < rectangle.2
                && pos.y > rectangle.1
                && pos.y < rectangle.3
            {
                light_poly.clip_one(**geom, pos);
            }
        }
        let triangulation = light_poly.triangulate();
        let geom_data = GeometryData::new(&display, &triangulation.points, &triangulation.indicies);
        for (entity, particles_data) in (&entities, &mut particles_datas).join() {
            match **particles_data {
                ParticlesData::Explosion(ref mut particles) => {
                    if particles.update() {
                        canvas
                            .render_particles(
                                &display,
                                &mut target,
                                &particles.gfx,
                                &Isometry3::new(
                                    Vector3::new(0f32, 0f32, 0f32),
                                    Vector3::new(0f32, 0f32, 0f32),
                                ),
                                1f32,
                            )
                            .unwrap();
                    } else {
                        entities.delete(entity).unwrap();
                    }
                }
                ParticlesData::Engine(ref mut particles) => {
                    // dbg!("ENGINE PARTICLES HERE");
                    let mut direction = Vector3::new(0f32, -1f32, 0f32);
                    direction = (char_iso * direction);
                    if particles.update(
                        Vector2::new(char_pos.x, char_pos.y),
                        Vector2::new(char_vel.x, char_vel.y),
                        Vector2::new(direction.x, direction.y)
                    ) {
                        canvas
                            .render_particles(
                                &display,
                                &mut target,
                                &particles.gfx,
                                &Isometry3::new(
                                    Vector3::new(0f32, 0f32, 0f32),
                                    Vector3::new(0f32, 0f32, 0f32),
                                ),
                                1f32,
                            )
                            .unwrap();
                    }
                }
                _ => (),
            };
        }
        for (iso, vel, _char_marker) in (&isometries, &velocities, &character_markers).join() {
            let translation_vec = iso.0.translation.vector;
            let mut isometry = Isometry3::new(translation_vec, Vector3::new(0f32, 0f32, 0f32));
            let pure_isometry = isometry.clone();
            isometry.translation.vector.z = canvas.get_z_shift();
            // canvas
            //     .render(&display, &mut target, &images[preloaded_images.background], &isometry, BACKGROUND_SIZE, false)
            //     .unwrap();
            match **particles_datas
                .get_mut(preloaded_particles.movement)
                .unwrap()
            {
                ParticlesData::MovementParticles(ref mut particles) => {
                    particles.update(1.0 * Vector2::new(-vel.0.x, -vel.0.y));
                    canvas
                        .render_particles(
                            &display,
                            &mut target,
                            &particles.gfx,
                            &pure_isometry,
                            vel.0.norm() / VELOCITY_MAX,
                        )
                        .unwrap();
                }
                _ => panic!(),
            };
            canvas
                .render_geometry(
                    &display,
                    &mut target,
                    &geom_data,
                    &Isometry3::identity(),
                    true,
                )
                .unwrap();
        }
        for (_entity, iso, image, size, _light) in
            (&entities, &isometries, &image_ids, &sizes, &light_markers).join()
        {
            let mut translation_vec = iso.0.translation.vector;
            translation_vec.z = canvas.get_z_shift();
            let isometry = Isometry3::new(translation_vec, Vector3::new(0f32, 0f32, 0f32));
            canvas
                .render(
                    &display,
                    &mut target,
                    &image_datas.get(image.0).unwrap(),
                    &isometry,
                    size.0,
                    true,
                )
                .unwrap();
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
            // canvas.render(&display, &mut target, &images[*image], &iso.0, size.0, false).unwrap();
            let triangulation = polygon.triangulate();
            let geom_data =
                GeometryData::new(&display, &triangulation.points, &triangulation.indicies);
            canvas
                .render_geometry(&display, &mut target, &geom_data, &iso.0, false)
                .unwrap();
        }
        for (_entity, physics_component, image, size, _ship) in
            (&entities, &physics, &image_ids, &sizes, &ship_markers).join()
        {
            let iso2 = world
                .rigid_body(physics_component.body_handle)
                .unwrap()
                .position();
            let iso = iso2_iso3(iso2);
            canvas
                .render(&display, &mut target, &image_datas.get(image.0).unwrap(), &iso, size.0, true)
                .unwrap();
        }
        for (_entity, iso, image, size, _projectile) in
            (&entities, &isometries, &image_ids, &sizes, &projectiles).join()
        {
            canvas
                .render(
                    &display,
                    &mut target,
                    &image_datas.get(image.0).unwrap(),
                    &iso.0,
                    size.0,
                    false,
                )
                .unwrap();
        }
        target.finish().unwrap();
    }
}

#[derive(Default)]
pub struct PhysicsSystem;

impl<'a> System<'a> for PhysicsSystem {
    type SystemData = (
        WriteStorage<'a, Isometry>,
        WriteStorage<'a, Velocity>,
        ReadStorage<'a, PhysicsComponent>,
        Write<'a, World<f32>>,
        Write<'a, BodiesMap>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut isometries, mut velocities, physics, mut world, _bodies_map) = data;
        for (isometry, velocity, physics_component) in
            (&mut isometries, &mut velocities, &physics).join()
        {
            let body = world.rigid_body(physics_component.body_handle).unwrap();
            let physics_isometry = body.position();
            let physics_velocity = body.velocity().as_vector();
            let physics_velocity = Vector2::new(physics_velocity.x, physics_velocity.y);
            isometry.0 = iso2_iso3(physics_isometry);
            velocity.0 = physics_velocity;
        }
        world.step();
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
            _asteroids,
            ship_markers,
            mut world,
        ) = data;
        for physics_component in (&physics).join() {
            let body = world.rigid_body_mut(physics_component.body_handle).unwrap();
            let mut velocity = *body.velocity();
            *velocity.as_vector_mut() *= DAMPING_FACTOR;
            body.set_velocity(velocity);
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
            let iso = isometries.get(*attach).unwrap();
            isometries.get_mut(*entity).unwrap().0 = iso.0;
        }
    }
}

pub struct ControlSystem {
    _reader: ReaderId<Keycode>,
}

impl ControlSystem {
    pub fn new(reader: ReaderId<Keycode>) -> Self {
        ControlSystem { _reader: reader }
    }
}

impl<'a> System<'a> for ControlSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Isometry>,
        WriteStorage<'a, Velocity>,
        WriteStorage<'a, PhysicsComponent>,
        WriteStorage<'a, Spin>,
        WriteStorage<'a, Image>,
        WriteStorage<'a, Gun>,
        WriteStorage<'a, Projectile>,
        WriteStorage<'a, Geometry>,
        WriteStorage<'a, Lifetime>,
        WriteStorage<'a, Size>,
        ReadStorage<'a, CharacterMarker>,
        Read<'a, EventChannel<Keycode>>,
        Read<'a, Mouse>,
        ReadExpect<'a, PreloadedImages>,
        Write<'a, EventChannel<Sound>>,
        ReadExpect<'a, PreloadedSounds>,
        Write<'a, World<f32>>,
        Write<'a, BodiesMap>,
        Write<'a, EventChannel<InsertEvent>>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            isometries,
            mut velocities,
            physics,
            mut spins,
            _images,
            mut guns,
            _projectiles,
            _geometries,
            _lifetimes,
            _sizes,
            character_markers,
            _keys_channel,
            mouse_state,
            _preloaded_images,
            mut sounds_channel,
            preloaded_sounds,
            mut world,
            _bodies_map,
            mut insert_channel,
        ) = data;
        // TODO add dt in params
        let dt = 1f32 / 60f32;
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
        let (_character_isometry, mut character_velocity) = {
            let character_body = world
                .rigid_body(physics.get(character).unwrap().body_handle)
                .unwrap();
            (*character_body.position(), *character_body.velocity())
        };
        if mouse_state.left {
            let gun = guns.get_mut(character).unwrap();
            if gun.shoot() {
                let isometry = *isometries.get(character).unwrap();
                let position = isometry.0.translation.vector;
                let direction = isometry.0 * Vector3::new(0f32, -1f32, 0f32);
                let velocity_rel = PLAYER_BULLET_SPEED * direction;
                let char_velocity = velocities.get(character).unwrap();
                let projectile_velocity = Velocity::new(
                    char_velocity.0.x + velocity_rel.x,
                    char_velocity.0.y + velocity_rel.y,
                );
                sounds_channel.single_write(Sound(preloaded_sounds.shot));
                insert_channel.single_write(InsertEvent::Bullet {
                    kind: EntityType::Player,
                    iso: Point3::new(position.x, position.y, isometry.0.rotation.euler_angles().2),
                    velocity: Point2::new(projectile_velocity.0.x, projectile_velocity.0.y),
                    owner: character,
                });
            }
        }
        if mouse_state.right {
            let rotation = isometries.get(character).unwrap().0.rotation;
            let _vel = velocities.get_mut(character).unwrap();
            let thrust = THRUST_FORCE * (rotation * Vector3::new(0.0, -1.0, 0.0));
            *character_velocity.as_vector_mut() += thrust;
        }
        let character_body = world
            .rigid_body_mut(physics.get(character).unwrap().body_handle)
            .unwrap();
        character_body.set_velocity(character_velocity);
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
        Entities<'a>,
        WriteStorage<'a, PhysicsComponent>,
        WriteStorage<'a, Geometry>,
        WriteStorage<'a, Isometry>,
        WriteStorage<'a, Velocity>,
        WriteStorage<'a, Spin>,
        WriteStorage<'a, Gun>,
        WriteStorage<'a, Lifetime>,
        WriteStorage<'a, AsteroidMarker>,
        WriteStorage<'a, EnemyMarker>,
        WriteStorage<'a, ShipMarker>,
        WriteStorage<'a, Image>,
        WriteStorage<'a, Size>,
        WriteStorage<'a, Polygon>,
        WriteStorage<'a, Projectile>,
        WriteStorage<'a, ThreadPin<ParticlesData>>,
        WriteExpect<'a, SDLDisplay>,
        Write<'a, Stat>,
        WriteExpect<'a, PreloadedImages>,
        Write<'a, World<f32>>,
        Write<'a, BodiesMap>,
        Read<'a, EventChannel<InsertEvent>>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            mut physics,
            mut geometries,
            mut isometries,
            mut velocities,
            mut spins,
            mut guns,
            mut lifetimes,
            mut asteroid_markers,
            mut enemies,
            mut ships,
            mut images,
            mut sizes,
            mut polygons,
            mut projectiles,
            mut particles_datas,
            display,
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
                        .with(Velocity::new(0f32, 0f32), &mut velocities)
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
                        Isometry2::new(Vector2::new(iso.x, iso.y), 0f32),
                        BodyStatus::Dynamic,
                        &mut world,
                        &mut bodies_map,
                        asteroid_collision_groups,
                        10f32,
                    );
                }
                InsertEvent::Ship {
                    iso,
                    light_shape: _,
                    spin: _,
                } => {
                    let enemy_size = 0.7f32;
                    let r = 1f32;
                    let enemy_shape = Geometry::Circle { radius: enemy_size };
                    let enemy_physics_shape = ncollide2d::shape::Ball::new(r);
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
                        .with(Image(preloaded_images.enemy), &mut images)
                        .with(Gun::new(50u8), &mut guns)
                        .with(Spin::default(), &mut spins)
                        .with(enemy_shape, &mut geometries)
                        .with(Size(enemy_size), &mut sizes)
                        .build();
                    PhysicsComponent::safe_insert(
                        &mut physics,
                        enemy,
                        ShapeHandle::new(enemy_physics_shape),
                        Isometry2::new(Vector2::new(iso.x, iso.y), iso.z),
                        BodyStatus::Dynamic,
                        &mut world,
                        &mut bodies_map,
                        enemy_collision_groups,
                        0.5f32,
                    );
                }
                InsertEvent::Bullet {
                    kind,
                    iso,
                    velocity,
                    owner,
                } => {
                    let bullet = entities
                        .build_entity()
                        .with(Velocity::new(velocity.x, velocity.y), &mut velocities)
                        .with(Isometry::new(iso.x, iso.y, iso.z), &mut isometries)
                        .with(Image(preloaded_images.projectile), &mut images)
                        .with(Spin::default(), &mut spins)
                        .with(Projectile { owner: *owner }, &mut projectiles)
                        .with(Lifetime::new(100u8), &mut lifetimes)
                        .with(Size(0.1), &mut sizes)
                        .build();
                    let player_bullet_collision_groups = match kind {
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
                            let mut player_bullet_collision_groups = CollisionGroups::new();
                            player_bullet_collision_groups
                                .set_membership(&[CollisionId::EnemyBullet as usize]);
                            player_bullet_collision_groups.set_whitelist(&[
                                CollisionId::Asteroid as usize,
                                CollisionId::PlayerShip as usize,
                            ]);
                            player_bullet_collision_groups
                                .set_blacklist(&[CollisionId::EnemyShip as usize]);
                            player_bullet_collision_groups
                        }
                    };
                    let r = 1f32;
                    let ball = ncollide2d::shape::Ball::new(r);
                    let bullet_physics_component = PhysicsComponent::safe_insert(
                        &mut physics,
                        bullet,
                        ShapeHandle::new(ball),
                        Isometry2::new(Vector2::new(iso.x, iso.y), iso.z),
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
                        &display,
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
                        &display,   
                        *position,
                        *num,
                        None,
                    )));
                    let _explosion_particles_entity = entities
                        .build_entity()
                        .with(engine_particles, &mut particles_datas)
                        .build();
                }
            }
        }
    }
}

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
        WriteStorage<'a, Gun>,
        WriteStorage<'a, Lifetime>,
        WriteStorage<'a, AsteroidMarker>,
        ReadStorage<'a, CharacterMarker>,
        ReadStorage<'a, ShipMarker>,
        WriteStorage<'a, Image>,
        WriteStorage<'a, Size>,
        WriteStorage<'a, Polygon>,
        Write<'a, Stat>,
        WriteExpect<'a, PreloadedImages>,
        Write<'a, World<f32>>,
        Write<'a, BodiesMap>,
        Write<'a, EventChannel<InsertEvent>>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            _physics,
            _geometries,
            isometries,
            _velocities,
            _spins,
            mut guns,
            mut lifetimes,
            asteroid_markers,
            character_markers,
            ships,
            _images,
            _sizes,
            _polygons,
            _stat,
            _preloaded_images,
            _world,
            _bodies_map,
            mut insert_channel,
        ) = data;
        let (char_isometry, _char) = (&isometries, &character_markers).join().next().unwrap();
        let pos3d = char_isometry.0.translation.vector;
        let character_position = Point2::new(pos3d.x, pos3d.y);
        for gun in (&mut guns).join() {
            gun.update()
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
            let size = rng.gen_range(0.4f32, 2f32);
            let r = size;
            let asteroid_shape = Geometry::Circle { radius: r };
            let poly = generate_convex_polygon(10, r);
            let spin = rng.gen_range(-1E-2, 1E-2);
            // let ball = ncollide2d::shape::Ball::new(r);
            let spawn_pos = spawn_position(character_position);
            insert_channel.single_write(InsertEvent::Asteroid {
                iso: Point3::new(
                    spawn_pos.x,
                    spawn_pos.y,
                    char_isometry.0.rotation.euler_angles().2,
                ),
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
            let spawn_pos = spawn_position(character_position);
            insert_channel.single_write(InsertEvent::Ship {
                iso: Point3::new(spawn_pos.x, spawn_pos.y, 0f32),
                light_shape: ship_shape,
                spin: 0f32,
            })
        }
        for (entity, isometry, _asteroid) in (&entities, &isometries, &asteroid_markers).join() {
            let pos3d = isometry.0.translation.vector;
            if !is_active(character_position, Point2::new(pos3d.x, pos3d.y)) {
                entities.delete(entity).unwrap();
            }
        }
        for (entity, isometry, _ship) in (&entities, &isometries, &ships).join() {
            let pos3d = isometry.0.translation.vector;
            if !is_active(character_position, Point2::new(pos3d.x, pos3d.y)) {
                entities.delete(entity).unwrap();
            }
        }
    }
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
        WriteStorage<'a, Polygon>,
        Write<'a, World<f32>>,
        Read<'a, BodiesMap>,
        Write<'a, EventChannel<InsertEvent>>,
        Write<'a, EventChannel<Sound>>,
        ReadExpect<'a, PreloadedSounds>,
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
            polygons,
            mut world,
            bodies_map,
            mut insert_channel,
            mut sounds_channel,
            preloaded_sounds,
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
                if projectiles.get(entity2).is_some() {
                    let asteroid = entity1;
                    let projectile = entity2;
                    entities.delete(projectile).unwrap();
                    let isometry = isometries.get(asteroid).unwrap().0;
                    let position = isometry.translation.vector;
                    let polygon = polygons.get(asteroid).unwrap();
                    let new_polygons = polygon.deconstruct();
                    let effect = InsertEvent::Explosion {
                        position: Point2::new(position.x, position.y),
                        num: 6usize,
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
                let _projectile = entity2;
                let isometry = isometries.get(ship).unwrap().0;
                let position = isometry.translation.vector;
                if character_markers.get(ship).is_some() {
                } else {
                    entities.delete(ship).unwrap();
                    let effect = InsertEvent::Explosion {
                        position: Point2::new(position.x, position.y),
                        num: 20usize,
                        lifetime: 50usize,
                    };
                    insert_channel.single_write(effect);
                }
            }
        }
    }
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
        WriteStorage<'a, Gun>,
        WriteStorage<'a, EnemyMarker>,
        ReadStorage<'a, CharacterMarker>,
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
            mut guns,
            enemies,
            character_markers,
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
        let dt = 1.0 / 60.0;
        for (entity, iso, vel, physics_component, spin, gun, _enemy) in (
            &entities,
            &isometries,
            &mut velocities,
            &physics,
            &mut spins,
            &mut guns,
            &enemies,
        )
            .join()
        {
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
            if diff.norm() > 4f32 {
                let dir = speed * (diff).normalize();
                *vel = Velocity::new(dir.x, dir.y);
            } else {
                let vel_vec = DAMPING_FACTOR * vel.0;
                *vel = Velocity::new(vel_vec.x, vel_vec.y);
            }
            let body = world.rigid_body_mut(physics_component.body_handle).unwrap();
            let mut velocity = *body.velocity();
            *velocity.as_vector_mut() = Vector3::new(vel.0.x, vel.0.y, spin.0);
            body.set_velocity(velocity);
            if gun.shoot() {
                insert_channel.single_write(InsertEvent::Bullet {
                    kind: EntityType::Enemy,
                    iso: Point3::new(position.x, position.y, isometry.rotation.euler_angles().2),
                    velocity: Point2::new(projectile_velocity.0.x, projectile_velocity.0.y),
                    owner: entity,
                });
            }
        }
    }
}
