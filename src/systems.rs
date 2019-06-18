use std::cmp::Ordering::Equal;

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

use crate::components::*;
use crate::geometry::{LightningPolygon, EPS};
use crate::gfx::{GeometryData, BACKGROUND_SIZE};
use crate::sound::{PreloadedSounds};

const DAMPING_FACTOR: f32 = 0.95f32;
const THRUST_FORCE: f32 = 0.01f32;
const VELOCITY_MAX: f32 = 1f32;
const MAX_TORQUE: f32 = 10f32;
const LIGHT_RECTANGLE_SIZE: f32 = 20f32;

const ASTEROIDS_NUMBER: u8 = 10u8;

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
        ReadStorage<'a, CharacterMarker>,
        ReadStorage<'a, ShipMarker>,
        ReadStorage<'a, AsteroidMarker>,
        ReadStorage<'a, LightMarker>,
        ReadStorage<'a, Projectile>,
        ReadStorage<'a, Image>,
        ReadStorage<'a, Geometry>,
        ReadStorage<'a, Size>,
        WriteExpect<'a, SDLDisplay>,
        WriteExpect<'a, Canvas>,
        ReadExpect<'a, ThreadPin<Images>>,
        WriteExpect<'a, ThreadPin<ParticlesSystems>>,
        ReadExpect<'a, PreloadedImages>,
        ReadExpect<'a, PreloadedParticles>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            isometries, 
            velocities,
            character_markers, 
            ship_markers,
            asteroid_markers, 
            light_markers,
            projectiles,
            image_ids, 
            geometries,
            sizes,
            display, 
            mut canvas, 
            images,
            mut particles_systems,
            preloaded_images,
            preloaded_particles,
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
        // dbg!(&light_poly);
        let (positions, indices) = light_poly.get_triangles();
        // dbg!(&positions);
        // dbg!(&indices);
        let geom_data = GeometryData::new(&display, &positions, &indices);
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
                .render_geometry(&display, &mut target, &geom_data, &Isometry3::identity())
                .unwrap();
        }
        for (_entity, iso, image, size, _light) in (&entities, &isometries, &image_ids, &sizes, &light_markers).join() {
            let mut translation_vec =iso.0.translation.vector;
            translation_vec.z = canvas.get_z_shift();
            let isometry = Isometry3::new(translation_vec, Vector3::new(0f32, 0f32, 0f32));
            canvas.render(&display, &mut target, &images[*image], &isometry, size.0, true).unwrap();
        }
        for (_entity, iso, image, size, _asteroid) in (&entities, &isometries, &image_ids, &sizes, &asteroid_markers).join() {
            canvas.render(&display, &mut target, &images[*image], &iso.0, size.0, false).unwrap();
        }
        for (_entity, iso, image, size, _ship) in (&entities, &isometries, &image_ids, &sizes, &ship_markers).join() {
            canvas.render(&display, &mut target, &images[*image], &iso.0, size.0, false).unwrap();
        }
        for (_entity, iso, image, size, _projectile) in (&entities, &isometries, &image_ids, &sizes, &projectiles).join() {
            canvas.render(&display, &mut target, &images[*image], &iso.0, size.0, false).unwrap();
        }
        target.finish().unwrap();
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
        ReadStorage<'a, Spin>,
        ReadStorage<'a, AttachPosition>,
        ReadStorage<'a, CharacterMarker>,
        ReadStorage<'a, AsteroidMarker>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities, 
            mut isometries, 
            mut velocities, 
            spins, 
            attach_positions,
            character_markers,
            _asteroids,
        ) = data;
        // TODO add dt -- time delta for period
        for (mut isometry, velocity, spin) in (&mut isometries, &mut velocities, &spins).join() {
            isometry += velocity;
            isometry.add_spin(spin.0);
        }
        for (velocity, _char) in (&mut velocities, &character_markers).join() {
            velocity.0 *= DAMPING_FACTOR;
        }
        let mut attach_pairs = vec![];
        for (entity, _, attach) in (&entities, &mut isometries, &attach_positions).join() {
            attach_pairs.push((entity, attach.0));
        }
        for (entity, attach) in attach_pairs.iter() {
            let isometry = isometries.get(*attach).unwrap();
            isometries.get_mut(*entity).unwrap().0 = isometry.0;
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
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities, 
            mut isometries, 
            mut velocities, 
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
            // for key in keys_channel.read(&mut self.reader) {
            //     match key {
            //         Keycode::Left | Keycode::A => {
            //             vel.0.x = (vel.0.x - THRUST_FORCE).max(-VELOCITY_MAX);
            //         }
            //         Keycode::Right | Keycode::D => {
            //             vel.0.x = (vel.0.x + THRUST_FORCE).min(VELOCITY_MAX);
            //         }
            //         Keycode::Up | Keycode::W => {
            //             vel.0.y = (vel.0.y + THRUST_FORCE).max(-VELOCITY_MAX);
            //         }
            //         Keycode::Down | Keycode::S => {
            //             vel.0.y = (vel.0.y - THRUST_FORCE).min(VELOCITY_MAX);
            //         }
            //         _ => (),
            //     }
            // }
        }
        let character = character.unwrap();
        if mouse_state.left {
            let gun = guns.get_mut(character).unwrap();
            if gun.shoot() {
                let isometry = *isometries.get(character).unwrap();
                let direction = 0.25 * Vector2::new(
                    mouse_state.x - isometry.0.translation.x,
                    mouse_state.y - isometry.0.translation.y
                ).normalize();
                let char_velocity = velocities.get(character).unwrap();
                let projectile_velocity = Velocity::new(char_velocity.0.x + direction.x, char_velocity.0.y + direction.y);
                let size = 0.1;
                sounds_channel.single_write(preloaded_sounds.shot);
                let _bullet_entity = entities
                    .build_entity()
                    .with(projectile_velocity, &mut velocities)
                    .with(isometry, &mut isometries)
                    .with(preloaded_images.projectile, &mut images)
                    .with(Spin::default(), &mut spins)
                    .with(Projectile{owner: character}, &mut projectiles)
                    .with(Geometry::Circle{radius: size}, &mut geometries)
                    .with(Lifetime::new(100u8), &mut lifetimes)
                    .with(Size(size), &mut sizes)
                    .build();
            }
        }
        if mouse_state.right {
            let rotation = isometries.get(character).unwrap().0.rotation;
            let vel = velocities.get_mut(character).unwrap();
            let thrust = THRUST_FORCE * (rotation * Vector3::new(0.0, -1.0, 0.0));
            let thrust = Vector2::new(thrust.x, thrust.y);
            vel.0 += thrust;
        }
    }
}

#[derive(Default)]
pub struct GamePlaySystem;

impl<'a> System<'a> for GamePlaySystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Geometry>,
        WriteStorage<'a, Isometry>,
        WriteStorage<'a, Velocity>,
        WriteStorage<'a, Spin>,
        WriteStorage<'a, Gun>,
        WriteStorage<'a, Lifetime>,
        WriteStorage<'a, AsteroidMarker>,
        WriteStorage<'a, Image>,
        WriteStorage<'a, Size>,
        Write<'a, Stat>,
        WriteExpect<'a, PreloadedImages>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities, 
            mut geometries,
            mut isometries,
            mut velocities,
            mut spins,
            mut guns, 
            mut lifetimes,
            mut asteroid_markers,
            mut images,
            mut sizes,
            mut stat,
            preloaded_images,
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
        for _ in 0..ASTEROIDS_NUMBER - stat.asteroids_number {
            stat.asteroids_number += 1;
            
            let mut rng = thread_rng();
            let size = rng.gen_range(0.4f32, 2f32);
            let asteroid_shape = Geometry::Circle{
                radius: size * 0.7,
            };
            let _asteroid = entities
                .build_entity()
                .with(asteroid_shape, &mut geometries)
                .with(Isometry::new(rng.gen_range(-10f32, 10f32), rng.gen_range(-10f32, 10f32), 0f32), &mut isometries)
                .with(Velocity::new(0f32, 0f32), &mut velocities)
                .with(AsteroidMarker::default(), &mut asteroid_markers)
                .with(preloaded_images.asteroid, &mut images)
                .with(Spin(rng.gen_range(-1E-2, 1E-2)), &mut spins)
                .with(Size(size), &mut sizes)
                .build();
        }
    }
}

#[derive(Default)]
pub struct CollisionSystem;

impl<'a> System<'a> for CollisionSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Isometry>,
        WriteStorage<'a, Velocity>,
        WriteStorage<'a, Spin>,
        ReadStorage<'a, Geometry>,
        ReadStorage<'a, Projectile>,
        ReadStorage<'a, AsteroidMarker>,
        ReadStorage<'a, CharacterMarker>,
        ReadStorage<'a, ShipMarker>,
    );

    fn run(&mut self, data: Self::SystemData) {
        // mb store kd tree where save specs::Entity based on it's isometry component in future
        let (
            entities, 
            isometries, 
            mut velocities, 
            _spins,
            geometries,
            projectiles,
            asteroid_markers,
            _character_markers,
            ship_markers,
        ) = data;
        let mut collisions = vec![];
        for entity1 in entities.join() {
            for entity2 in entities.join() {
                if entity1 == entity2 || entity1.id() > entity2.id() {continue};
                match (geometries.get(entity1), geometries.get(entity2)) {
                    (Some(geom1), Some(geom2)) => {
                        let iso1 = isometries.get(entity1).unwrap();
                        let iso2 = isometries.get(entity2).unwrap();
                        match (geom1, geom2) {
                            (Geometry::Circle{radius:r1},
                             Geometry::Circle{radius: r2}) => {
                                if (iso1.0.translation.vector -
                                iso2.0.translation.vector).norm() < r1 + r2 {
                                    collisions.push((entity1, entity2))
                                    // dbg!("COLLISION");
                                }
                            }
                        }
                    }
                    _ => ()
                }
            }
        }
        // dbg!(collisions);
        for (entity1, entity2) in collisions.iter() {
            match (projectiles.get(*entity1), asteroid_markers.get(*entity2)) {
                (Some(projectile), Some(_)) => {
                    if projectile.owner != *entity2 {
                        entities.delete(*entity1).unwrap();
                    }
                }
                _ => ()
            }
            match (projectiles.get(*entity2), asteroid_markers.get(*entity1)) {
                (Some(projectile), Some(_)) => {
                    if projectile.owner != *entity1 {
                        entities.delete(*entity2).unwrap();
                    }
                }
                _ => ()
            }
            let iso1 = isometries.get(*entity1).unwrap().0.translation.vector;
            let iso2 = isometries.get(*entity2).unwrap().0.translation.vector;
            let center = (iso1 + iso2) / 2f32;
            if (iso1 - center).norm() < EPS {
                continue
            };
            let attack1 = 0.005 * (iso1 - center).normalize();
            let attack2 = 0.005 * (iso2 - center).normalize();

            match (projectiles.get(*entity1), ship_markers.get(*entity2),
                   ship_markers.get(*entity1), projectiles.get(*entity2)) {
                (Some(_), Some(_), _, _) |
                (_, _, Some(_), Some(_)) => {

                }
                _ => ()
            }

            // character_asteroid collision
            match (ship_markers.get(*entity1), asteroid_markers.get(*entity2),
                   asteroid_markers.get(*entity2), ship_markers.get(*entity1)) {
                (Some(_), Some(_), _, _) | 
                (_, _, Some(_), Some(_)) => {
                    match velocities.get_mut(*entity1) {
                        Some(velocity) => {
                            *velocity = Velocity::new(attack1.x, attack1.y);
                        }
                        None => ()
                    }
                    match velocities.get_mut(*entity2) {
                        Some(velocity) => {
                            *velocity = Velocity::new(attack2.x, attack2.y);
                        }
                        None => ()
                    }
                }
                _ => ()
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