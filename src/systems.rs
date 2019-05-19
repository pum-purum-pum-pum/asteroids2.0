use astro_lib as al;
use al::prelude::*;
use specs::{Join};
use specs::prelude::*;

use crate::components::{Position, Velocity};

const DAMPING_FACTOR: f32 = 0.95f32;

pub struct KinematicSystem;

impl<'a> System<'a> for KinematicSystem {
    type SystemData = (
        WriteStorage<'a, Position>,
        WriteStorage<'a, Velocity>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut positions,
            mut velocities,
        ) = data;
        for (position, velocity) in (&mut positions, &mut velocities).join() {
            position.0 += velocity.0;
            velocity.0 *= DAMPING_FACTOR;
        }
    }
}