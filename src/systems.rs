use al::prelude::*;
use astro_lib as al;
use rand::prelude::*;

use glium;
use glium::Surface;
use sdl2::keyboard::Keycode;
use shrev::EventChannel;
use specs::prelude::*;
use specs::Join;

use crate::components::*;
use crate::geometry::{LightningPolygon, EPS};
use crate::gfx::{GeometryData};

const DAMPING_FACTOR: f32 = 0.95f32;
const THRUST_FORCE: f32 = 0.01f32;
const VELOCITY_MAX: f32 = 1f32;
const MAX_TORQUE: f32 = 10f32;

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

    (angle_diff * 100.0 - speed * 55.0)
}

#[derive(Default)]
pub struct RenderingSystem;

impl<'a> System<'a> for RenderingSystem {
    type SystemData = (
        ReadStorage<'a, Isometry>,
        ReadStorage<'a, CharacterMarker>,
        ReadStorage<'a, AsteroidMarker>,
        ReadStorage<'a, Image>,
        ReadStorage<'a, Geometry>,
        ReadStorage<'a, Size>,
        WriteExpect<'a, SDLDisplay>,
        WriteExpect<'a, Canvas>,
        ReadExpect<'a, ThreadPin<Images>>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            isometries, 
            character_markers, 
            asteroid_markers, 
            image_ids, 
            geometries,
            sizes,
            display, 
            mut canvas, 
            images
        ) = data;
        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        target.clear_stencil(0i32);
        let char_pos = {
            let mut opt_iso = None;
            for (iso, _) in (&isometries, &character_markers).join() {
                canvas.update_observer(Point2::new(
                    iso.0.translation.vector.x,
                    iso.0.translation.vector.y,
                ));
                opt_iso = Some(iso)
            }
            opt_iso.unwrap().0.translation.vector
        };
        // @vlad TODO rewrite it with screen borders
        let rectangle = (
            char_pos.x - 10f32,
            char_pos.y - 10f32,
            char_pos.x + 10f32,
            char_pos.y + 10f32,
        );
        let mut light_poly = LightningPolygon::new_rectangle(
            rectangle.0,
            rectangle.1,
            rectangle.2,
            rectangle.3,
            Point2::new(char_pos.x, char_pos.y),
        );
        // UNCOMMENT TO ADD LIGHTS
        for (iso, geom, _) in (&isometries, &geometries, &asteroid_markers).join() {
            let pos = Point2::new(iso.0.translation.vector.x, iso.0.translation.vector.y);
            if pos.x > rectangle.0 && pos.x < rectangle.2 && pos.y > rectangle.1 && pos.y < rectangle.3 {
                light_poly.clip_one(*geom, pos);
            }
        }
        // dbg!(&light_poly);
        let (positions, indices) = light_poly.get_triangles();
        // dbg!(&positions);
        // dbg!(&indices);
        let geom_data = GeometryData::new(&display, &positions, &indices);
        for (_iso, _char_marker) in (&isometries, &character_markers).join() {
            canvas
                .render_geometry(&display, &mut target, &geom_data, &Isometry3::identity())
                .unwrap();
        }
        for (iso, image, size) in (&isometries, &image_ids, &sizes).join() {
            canvas.render(&display, &mut target, &images[*image], &iso.0, size.0).unwrap();
        }
        target.finish().unwrap();
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
            asteroids,
        ) = data;
        // TODO add dt -- time delta for period
        for (mut isometry, velocity, spin) in (&mut isometries, &mut velocities, &spins).join() {
            isometry += velocity;
            isometry.add_spin(spin.0);
        }
        for (isometry, asteroid) in (&isometries, &asteroids).join() {
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
            for key in keys_channel.read(&mut self.reader) {
                match key {
                    Keycode::Left | Keycode::A => {
                        vel.0.x = (vel.0.x - THRUST_FORCE).max(-VELOCITY_MAX);
                    }
                    Keycode::Right | Keycode::D => {
                        vel.0.x = (vel.0.x + THRUST_FORCE).min(VELOCITY_MAX);
                    }
                    Keycode::Up | Keycode::W => {
                        vel.0.y = (vel.0.y + THRUST_FORCE).max(-VELOCITY_MAX);
                    }
                    Keycode::Down | Keycode::S => {
                        vel.0.y = (vel.0.y - THRUST_FORCE).min(VELOCITY_MAX);
                    }
                    _ => (),
                }
            }
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
            mut preloaded_images,
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
                radius: size,
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
    );

    fn run(&mut self, data: Self::SystemData) {
        /// mb store kd tree where save specs::Entity based on it's isometry component in future
        let (
            mut entities, 
            mut isometries, 
            mut velocities, 
            mut spins,
            geometries,
            projectiles,
            asteroid_markers,
            character_markers,
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
                            _ => ()
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

            // character_asteroid 
            match (character_markers.get(*entity1), asteroid_markers.get(*entity2)) {
                (Some(_), Some(_)) => (),
                _ => continue
            }
            match (asteroid_markers.get(*entity2), character_markers.get(*entity1)) {
                (Some(_), Some(_)) => (),
                _ => continue
            }
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
        // for (iso1, vel1, spin1) in (&mut isometries, &mut velocities, &mut spins).join() {

        // }
    }
}
