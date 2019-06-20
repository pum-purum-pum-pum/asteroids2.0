use std::cmp::Ordering::Equal;
use std::mem::swap;

use al::prelude::*;
use astro_lib as al;
use rand::prelude::*;

use glium;
use glium::Surface;
use sdl2::keyboard::Keycode;
use sdl2::TimerSubsystem;

use shrev::EventChannel;
use specs::prelude::*;
use specs::Join;
use nphysics2d::object::{BodyStatus, Body};
use nphysics2d::world::{World};
use ncollide2d::world::CollisionGroups;
use ncollide2d::shape::ShapeHandle;
use ncollide2d::world::CollisionObjectHandle;

use crate::components::*;
use crate::geometry::{LightningPolygon, EPS, TriangulateFromCenter, Polygon, generate_convex_polygon};
use crate::gfx::{GeometryData, BACKGROUND_SIZE};
use crate::sound::{PreloadedSounds};
use crate::physics::CollisionId;

const DAMPING_FACTOR: f32 = 0.98f32;
const THRUST_FORCE: f32 = 0.01f32;
const VELOCITY_MAX: f32 = 1f32;
const MAX_TORQUE: f32 = 10f32;
const LIGHT_RECTANGLE_SIZE: f32 = 20f32;

const ASTEROIDS_NUMBER: usize = 10usize;

pub enum InsertEvent {
    Asteroid {
        pos: Point2,
        polygon: Polygon,
        light_shape: Geometry,
        spin: f32,
    },
}

fn iso2_iso3(iso2: &Isometry2) -> Isometry3{
     Isometry3::new(
        Vector3::new(iso2.translation.vector.x, iso2.translation.vector.y, 0f32),
        Vector3::new(0f32, 0f32, iso2.rotation.angle())
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

    (angle_diff * 3.0 - speed * 55.0)
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
        ReadStorage<'a, Image>,
        ReadStorage<'a, Geometry>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, Polygon>,
        WriteExpect<'a, SDLDisplay>,
        WriteExpect<'a, Canvas>,
        ReadExpect<'a, ThreadPin<Images>>,
        WriteExpect<'a, ThreadPin<ParticlesSystems>>,
        ReadExpect<'a, PreloadedImages>,
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
            image_ids, 
            geometries,
            sizes,
            polygons,
            display, 
            mut canvas, 
            images,
            mut particles_systems,
            preloaded_images,
            preloaded_particles,
            world,
        ) = data;
        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        target.clear_stencil(0i32);
        let char_pos = {
            let mut opt_iso = None;
            for (iso, vel, _) in (&isometries, &velocities,  &character_markers).join() {
                canvas.update_observer(
                    Point2::new(
                        iso.0.translation.vector.x,
                        iso.0.translation.vector.y,
                    ),
                    vel.0.norm() / VELOCITY_MAX
                );
                opt_iso = Some(iso)
            }
            opt_iso.unwrap().0.translation.vector
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
        let mut data = (&entities, &isometries, &geometries, &asteroid_markers).join().collect::<Vec<_>>(); // TODO move variable to field  to avoid allocations
        let distance = |a: &Isometry| {(char_pos - a.0.translation.vector).norm()};
        data.sort_by(|&a, &b| {(distance(b.1).partial_cmp(&distance(a.1)).unwrap_or(Equal))});
        // UNCOMMENT TO ADD LIGHTS
        for (_entity, iso, geom, _) in data.iter() {
            let pos = Point2::new(iso.0.translation.vector.x, iso.0.translation.vector.y);
            if pos.x > rectangle.0 && pos.x < rectangle.2 && pos.y > rectangle.1 && pos.y < rectangle.3 {
                light_poly.clip_one(**geom, pos);
            }
        }
        let triangulation = light_poly.triangulate();
        let geom_data = GeometryData::new(&display, &triangulation.points, &triangulation.indicies);
        for (iso, vel, _char_marker) in (&isometries, &velocities, &character_markers).join() {
            let translation_vec =iso.0.translation.vector;
            let mut isometry = Isometry3::new(translation_vec, Vector3::new(0f32, 0f32, 0f32));
            let pure_isometry = isometry.clone();
            isometry.translation.vector.z = canvas.get_z_shift();
            // canvas
            //     .render(&display, &mut target, &images[preloaded_images.background], &isometry, BACKGROUND_SIZE, false)
            //     .unwrap();
            particles_systems[preloaded_particles.movement].update(1.0 * Vector2::new(-vel.0.x, -vel.0.y));
            canvas
                .render_particles(&display, &mut target, &particles_systems[preloaded_particles.movement], &pure_isometry, vel.0.norm() / VELOCITY_MAX).unwrap();
            canvas
                .render_geometry(&display, &mut target, &geom_data, &Isometry3::identity(), true)
                .unwrap();
        }
        for (_entity, iso, image, size, _light) in (&entities, &isometries, &image_ids, &sizes, &light_markers).join() {
            let mut translation_vec =iso.0.translation.vector;
            translation_vec.z = canvas.get_z_shift();
            let isometry = Isometry3::new(translation_vec, Vector3::new(0f32, 0f32, 0f32));
            canvas.render(&display, &mut target, &images[*image], &isometry, size.0, true).unwrap();
        }
        for (_entity, iso, image, size, polygon, _asteroid) in (&entities, &isometries, &image_ids, &sizes, &polygons, &asteroid_markers).join() {
            // canvas.render(&display, &mut target, &images[*image], &iso.0, size.0, false).unwrap();
            let triangulation = polygon.triangulate();
            let geom_data = GeometryData::new(&display, &triangulation.points, &triangulation.indicies);
            canvas.render_geometry(&display, &mut target, &geom_data, &iso.0, false).unwrap();
        }
        for (_entity, physics_component, image, size, _ship) in (&entities, &physics, &image_ids, &sizes, &ship_markers).join() {
            let iso2 = world.rigid_body(physics_component.body_handle).unwrap().position();
            let iso = iso2_iso3(iso2);
            canvas.render(&display, &mut target, &images[*image], &iso, size.0, false).unwrap();
        }
        for (_entity, iso, image, size, _projectile) in (&entities, &isometries, &image_ids, &sizes, &projectiles).join() {
            canvas.render(&display, &mut target, &images[*image], &iso.0, size.0, false).unwrap();
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
        let (
            mut isometries,
            mut velocities,
            mut physics,
            mut world,
            bodies_map,
        ) = data;
        for (isometry, velocity, physics_component) in (&mut isometries, &mut velocities, &physics).join() {
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
    reader: ReaderId<Sound>
}

impl SoundSystem {
    pub fn new(reader: ReaderId<Sound>) -> Self {
        SoundSystem { reader: reader }
    }
}

impl<'a> System<'a> for SoundSystem {
    type SystemData = (
        ReadExpect<'a, ThreadPin<Sounds>>,
        WriteExpect<'a, ThreadPin<TimerSubsystem>>,
        Write<'a, EventChannel<Sound>>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            sounds,
            _timer,
            sounds_channel,
        ) = data;
        for s in sounds_channel.read(&mut self.reader) {
            sdl2::mixer::Channel::all().play(&sounds[*s], 0).unwrap();
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
            _asteroids,
            mut world,
        ) = data;
        for (physics_component) in (&physics).join() {
            let body = world.rigid_body_mut(physics_component.body_handle).unwrap();
            let mut velocity = *body.velocity();
            *velocity.as_vector_mut() *= DAMPING_FACTOR;
            body.set_velocity(velocity);
            body.activate();
        }
        for (isometry, velocity, physics_component, spin, _character) in (&mut isometries, &mut velocities, &physics, &spins, &character_markers).join() {
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
    reader: ReaderId<Keycode>,
}

impl ControlSystem {
    pub fn new(reader: ReaderId<Keycode>) -> Self {
        ControlSystem { reader: reader }
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
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities, 
            mut isometries, 
            mut velocities, 
            mut physics,
            mut spins, 
            mut images,
            mut guns,
            mut projectiles,
            mut geometries,
            mut lifetimes,
            mut sizes,
            character_markers, 
            keys_channel, 
            mouse_state,
            preloaded_images,
            mut sounds_channel,
            preloaded_sounds,
            mut world,
            mut bodies_map,
        ) = data;
        // TODO add dt in params
        let dt = 1f32 / 60f32;
        let mut character = None;
        for (entity, iso, vel, spin, _char_marker) in
            (&entities, &isometries, &mut velocities, &mut spins, &character_markers).join()
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
            let mut character_body = world.rigid_body(physics.get(character).unwrap().body_handle).unwrap();
            (*character_body.position(), *character_body.velocity())
        };
        if mouse_state.left {
            let gun = guns.get_mut(character).unwrap();
            if gun.shoot() {
                let isometry = *isometries.get(character).unwrap();
                let position = isometry.0.translation.vector;
                let direction = isometry.0 * Vector3::new(0f32, -1f32, 0f32);
                let velocity_rel = 0.5 * direction;
                let char_velocity = velocities.get(character).unwrap();
                let projectile_velocity = Velocity::new(char_velocity.0.x + velocity_rel.x, char_velocity.0.y + velocity_rel.y);
                let size = 0.1;
                sounds_channel.single_write(preloaded_sounds.shot);
                let bullet = entities
                    .build_entity()
                    .with(projectile_velocity.clone(), &mut velocities)
                    .with(isometry, &mut isometries)
                    .with(preloaded_images.projectile, &mut images)
                    .with(Spin::default(), &mut spins)
                    .with(Projectile{owner: character}, &mut projectiles)
                    .with(Geometry::Circle{radius: size}, &mut geometries)
                    .with(Lifetime::new(100u8), &mut lifetimes)
                    .with(Size(size), &mut sizes)
                    .build();

                let mut player_bullet_collision_groups = CollisionGroups::new();
                player_bullet_collision_groups.set_membership(&[CollisionId::PlayerBullet as usize]);
                player_bullet_collision_groups.set_whitelist(&[
                    CollisionId::Asteroid as usize,
                    CollisionId::EnemyShip as usize,]);
                player_bullet_collision_groups.set_blacklist(&[CollisionId::PlayerShip as usize]);

                let r = 1f32;
                let ball = ncollide2d::shape::Ball::new(r);
                let bullet_physics_component = PhysicsComponent::safe_insert(
                    &mut physics,
                    bullet,
                    ShapeHandle::new(ball),
                    Isometry2::new(Vector2::new(position.x, position.y), isometry.rotation()),
                    BodyStatus::Dynamic,
                    &mut world,
                    &mut bodies_map,
                    player_bullet_collision_groups,
                    0.4f32
                );
                let body = world.rigid_body_mut(bullet_physics_component.body_handle).unwrap();
                let mut velocity = *body.velocity();
                *velocity.as_vector_mut() = Vector3::new(projectile_velocity.0.x, projectile_velocity.0.y, 0f32);
                body.set_velocity(velocity);
            }
        }
        if mouse_state.right {
            let rotation = isometries.get(character).unwrap().0.rotation;
            let vel = velocities.get_mut(character).unwrap();
            let thrust = THRUST_FORCE * (rotation * Vector3::new(0.0, -1.0, 0.0));
            *character_velocity.as_vector_mut() += thrust;
        }
        let character_body = world.rigid_body_mut(physics.get(character).unwrap().body_handle).unwrap();
        character_body.set_velocity(character_velocity);
    }
}


pub struct InsertSystem {
    reader: ReaderId<InsertEvent>
}

impl InsertSystem {
    pub fn new(reader: ReaderId<InsertEvent>) -> Self {
        InsertSystem {
            reader: reader
        }
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
        WriteStorage<'a, Image>,
        WriteStorage<'a, Size>,
        WriteStorage<'a, Polygon>,
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
            mut images,
            mut sizes,
            mut polygons,
            mut stat,
            preloaded_images,
            mut world,
            mut bodies_map,
            insert_channel
        ) = data;
        for insert in insert_channel.read(&mut self.reader) {
            match insert {
                InsertEvent::Asteroid {
                    pos,
                    polygon,
                    light_shape,
                    spin,
                } => {
                    let physics_polygon = ncollide2d::shape::ConvexPolygon::try_from_points(
                        &polygon.points()
                    ).unwrap();
                    let mut asteroid = entities
                        .build_entity()
                        .with(*light_shape, &mut geometries)
                        .with(Isometry::new(pos.x, pos.y, 0f32), &mut isometries)
                        .with(Velocity::new(0f32, 0f32), &mut velocities)
                        .with(polygon.clone(), &mut polygons)
                        .with(AsteroidMarker::default(), &mut asteroid_markers)
                        .with(preloaded_images.asteroid, &mut images)
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
                        Isometry2::new(Vector2::new(pos.x, pos.y), 0f32),
                        BodyStatus::Dynamic,
                        &mut world,
                        &mut bodies_map,
                        asteroid_collision_groups,
                        10f32,
                    );
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
            mut physics,
            mut geometries,
            mut isometries,
            mut velocities,
            mut spins,
            mut guns, 
            mut lifetimes,
            mut asteroid_markers,
            mut images,
            mut sizes,
            mut polygons,
            mut stat,
            preloaded_images,
            mut world,
            mut bodies_map,
            mut insert_channel,
        ) = data;
        for gun in (&mut guns).join() {
            gun.update()
        }
        for (entity, lifetime) in (& entities, &mut lifetimes).join() {
            lifetime.update();
            if lifetime.delete() {
                entities.delete(entity).unwrap()
            }
        }
        let cnt = asteroid_markers.count();
        let add_cnt = if ASTEROIDS_NUMBER > cnt {ASTEROIDS_NUMBER - cnt} else {0}; 
        for _ in 0..add_cnt {
            stat.asteroids_number += 1;
            
            let mut rng = thread_rng();
            let size = rng.gen_range(0.4f32, 2f32);
            let r =  size;
            let asteroid_shape = Geometry::Circle{
                radius: r,
            };
            let poly = generate_convex_polygon(10, r);
            let spin = rng.gen_range(-1E-2, 1E-2);
            // let ball = ncollide2d::shape::Ball::new(r);
            insert_channel.single_write(
                InsertEvent::Asteroid {
                    pos: Point2::new(rng.gen_range(-10f32, 10f32), rng.gen_range(-10f32, 10f32)),
                    polygon: poly,
                    light_shape: asteroid_shape,
                    spin: spin,
                }
            );
        }
    }
}

#[derive(Default)]
pub struct CollisionSystem {
    colliding_start_events: Vec<(CollisionObjectHandle, CollisionObjectHandle)>,
    colliding_end_events: Vec<(CollisionObjectHandle, CollisionObjectHandle)>
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
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities, 
            isometries,
            physics, 
            asteroids,
            character_markers,
            ships,
            projectiles,
            mut polygons,
            mut world,
            bodies_map,
            mut insert_channel
        ) = data; 
        self.colliding_start_events.clear();
        self.colliding_end_events.clear();
        for event in world.contact_events() {
            match event {
                &ncollide2d::events::ContactEvent::Started(
                        collision_handle1,
                        collision_handle2,
                ) => {
                    self.colliding_start_events.push((collision_handle1, collision_handle2))
                }
                &ncollide2d::events::ContactEvent::Stopped(
                        collision_handle1, 
                        collision_handle2,
                ) => {
                    self.colliding_end_events.push((collision_handle1, collision_handle2))
                }
            }
        }
        for (handle1, handle2) in self.colliding_start_events.iter() {
            let (body_handle1, body_handle2) = {
                // get body handles
                let collider_world = world.collider_world_mut();
                (
                    collider_world
                        .collider_mut(*handle1)
                        .unwrap()
                        .body(),
                    collider_world
                        .collider_mut(*handle2)
                        .unwrap()
                        .body(),
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
                    let position = isometries.get(asteroid).unwrap().0.translation.vector;
                    let polygon = polygons.get(asteroid).unwrap();
                    let new_polygons = polygon.deconstruct();
                    if new_polygons.len() == 1 {

                    } else {
                        for poly in new_polygons.iter() {
                            let r = poly.min_r;
                            let asteroid_shape = Geometry::Circle{
                                radius: r,
                            };
                            let mut rng = thread_rng();
                            let insert_event = InsertEvent::Asteroid {
                                pos: Point2::new(position.x, position.y),
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
        }
    }
}


#[derive(Default)]
pub struct AISystem;

impl<'a> System<'a> for AISystem {
    type SystemData = (
        WriteStorage<'a, Isometry>,
        WriteStorage<'a, Velocity>,
        WriteStorage<'a, Spin>,
        WriteStorage<'a, EnemyMarker>,
        ReadStorage<'a, CharacterMarker>,
        Write<'a, Stat>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            isometries,
            mut velocities,
            mut spins,
            enemies,
            character_markers,
            _stat,
        ) = data;
        let _rng = thread_rng();
        let character_position = {
            let mut res = None;
            for (iso, _) in (&isometries, &character_markers).join() {
                res = Some(iso.0.translation.vector)
            }
            res.unwrap()
        };
        let dt = 1.0/60.0;
        for (iso, vel, spin, _enemy) in (&isometries, &mut velocities, &mut spins, &enemies).join() {
            let ship_torque = dt
                * calculate_player_ship_spin_for_aim(
                    Vector2::new(character_position.x, character_position.y)
                        - Vector2::new(iso.0.translation.vector.x, iso.0.translation.vector.y),
                    iso.rotation(),
                    spin.0,
                );
            spin.0 += ship_torque.max(-MAX_TORQUE).min(MAX_TORQUE);
            let speed = 0.1f32;
            let diff = character_position - iso.0.translation.vector;
            if diff.norm() > 4f32 {
                let dir = speed * (diff).normalize();
                *vel = Velocity::new(dir.x, dir.y);
            } else {
                let vel_vec = DAMPING_FACTOR * vel.0;
                *vel = Velocity::new(vel_vec.x, vel_vec.y);
            }
        }
    }
}