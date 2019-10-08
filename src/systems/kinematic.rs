use super::*;
use log::info;

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
        info!("asteroids: kinematic system started");
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
            match isometries.get(*attach) {
                Some(isometry) => {
                    let iso = isometry;
                    isometries.get_mut(*entity).unwrap().0.translation.vector =
                        iso.0.translation.vector;
                }
                None => {
                    entities.delete(*entity).unwrap();
                }
            }
        }
        info!("asteroids: kinematic system ended");
    }
}
