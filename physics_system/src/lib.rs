use components::*;
use log::info;
use common::*;

use std::time::{Instant, Duration};
use specs::prelude::*;
use specs::Join;
use nphysics2d::world::World;
use nphysics2d::object::{Body};
use nphysics2d::algebra::ForceType;
use nphysics2d::algebra::Force2;
use physics::*;
pub const MENU_VELOCITY: (f32, f32) = (0.0, 0.2);
pub const FRAME60: f32 = 1f32 / 60f32;

pub fn normalize_60frame(duration: Duration) -> f32 {
    duration.as_millis() as f32 / 1E3 / FRAME60
}

#[derive(Default, Clone)]
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
        WriteExpect<'a, TimeTracker>,
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
            app_state,
            mut time_tracker,
        ) = data;
        let time_scaler = normalize_60frame(time_tracker.update());
        world.set_timestep(PHYSICS_SIMULATION_TIME * time_scaler);
        let (character_position, character_prev_position) = {
            if let Some((character, isometry, _)) = (&entities, &isometries, &character_markers).join().next() {
                let body = world
                    .rigid_body(
                        physics
                            .get(character).unwrap()
                            .body_handle
                    ).unwrap();
                (*body.position(), *isometry)
            } else {
                (
                	Isometry2::new(Vector2::new(MENU_VELOCITY.0 ,MENU_VELOCITY.1), 0f32),
                	Isometry::new(0f32, 0f32, 0f32),
                )
            }
        };
        info!("asteroids: physics started");
        flame::start("physics");
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
            let force_factor = 0.006;
            for e1 in enemies_entities.iter() {
                for e2 in enemies_entities.iter() {
                    if e1 == e2 {
                        break
                    }
                    let phys1 = physics.get(*e1).unwrap();
                    let phys2 = physics.get(*e2).unwrap();
                    let body1 = world.rigid_body(phys1.body_handle).unwrap();
                    let body2 = world.rigid_body(phys2.body_handle).unwrap();
                    let position1 = body1.position().translation.vector;
                    let position2 = body2.position().translation.vector;
                    let distance = (position1 - position2).norm();
                    let center = (position1 + position2) / 2.0;
                    if chargings.get(*e1).is_some() || chargings.get(*e2).is_some() {
                        continue
                    }
                    let mut applyed = false;
                    if let Some(c1) = chains.get(*e1) {
                        applyed = true;
                        if c1.follow == *e2 {
                            let force1 = Force2::new(-1.0 * force_factor * (position1 - center).normalize(), 0.0);
                            world.rigid_body_mut(phys1.body_handle).unwrap()
                                .apply_force(0, &force1, ForceType::Force, true);
                        }
                    }
                    if let Some(c2) = chains.get(*e2) {
                        applyed = true;
                        if c2.follow == *e1 {
                            let force2 = Force2::new(-1.0 * force_factor * (position2 - center).normalize(), 0.0);
                            world.rigid_body_mut(phys2.body_handle).unwrap()
                                .apply_force(0, &force2, ForceType::Force, true);                            
                        }
                    }
                    if applyed {
                        continue
                    }
                    let (force1, force2, distance) = {
                        (
                            Force2::new(force_factor * (position1 - center).normalize(), 0.0), 
                            Force2::new(force_factor * (position2 - center).normalize(), 0.0),
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
            physics_isometry.translation.vector -= char_vec;
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
        info!("asteroids: physics ended");
    }
}
