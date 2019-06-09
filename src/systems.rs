use astro_lib as al;
use al::prelude::*;

use specs::{Join};
use specs::prelude::*;
use shrev::EventChannel;
use sdl2::keyboard::Keycode;
use glium::Surface;
use glium;

use crate::components::{*};
use crate::gfx::{ImageData, GeometryData};
use crate::geometry::LightningPolygon;

const DAMPING_FACTOR: f32 = 0.95f32;
const THRUST_FORCE: f32 = 0.01f32;
const VELOCITY_MAX: f32 = 1f32;
// const MAX_ROTATION_SPEED: f32 = 0.1f32;
const MAX_TORQUE: f32 = 10f32;

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
        -(-aim.x).atan2(aim.y)
    };

    let angle_diff = angle_shortest_dist(rotation, target_rot);

    (angle_diff * 100.0 - speed * 55.0)
}


#[derive(Default)]
pub struct RenderingSystem;

type Image = ThreadPin<ImageData>;

impl<'a> System<'a> for RenderingSystem {
    type SystemData = (
        ReadStorage<'a, Isometry>,
        ReadStorage<'a, CharacterMarker>,
        ReadStorage<'a, Image>,

        WriteExpect<'a, SDLDisplay>,
        WriteExpect<'a, Canvas>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            isometries,
            character_markers,
            image_data,
            display,
            mut canvas,
        ) = data;
        let mut target = display.draw();
            target.clear_color(0.0, 0.0, 0.0, 1.0);
        for (iso, image) in (&isometries, &image_data).join() {
            canvas.render(
                &display, 
                &mut target, 
                image, 
                &iso.0,
            ).unwrap();
        }
        for (iso, _) in (&isometries, &character_markers).join() {
            canvas.update_observer(Point2::new(iso.0.translation.vector.x, iso.0.translation.vector.y));
        }
        let mut light_poly = LightningPolygon::new_rectangle(0f32, 0f32, 1f32, 1f32);
        let (positions, indices) = light_poly.get_triangles();
        let geom_data = GeometryData::new(&display, &positions, &indices);
        for (iso, _) in (&isometries, &character_markers).join() {
            canvas.render_geometry(
                &display,
                &mut target, 
                &geom_data, 
                &iso.0
            ).unwrap();
        }
        target.finish().unwrap();
    }
}

pub struct KinematicSystem;

impl<'a> System<'a> for KinematicSystem {
    type SystemData = (
        WriteStorage<'a, Isometry>,
        WriteStorage<'a, Velocity>,
        ReadStorage<'a, Spin>,
    );


    fn run(&mut self, data: Self::SystemData) {
        let (
            mut isometries,
            mut velocities,
            spins,
        ) = data;
        // TODO add dt -- time delta for period
        for (
            mut isometry, 
            velocity, 
            spin
        ) in (
                &mut isometries, 
                &mut velocities, 
                &spins
        ).join() {
            isometry += velocity;
            isometry.add_spin(spin.0);
            velocity.0 *= DAMPING_FACTOR;
        }
    }
}

pub struct ControlSystem {
    reader: ReaderId<Keycode>,
}

impl ControlSystem {
    pub fn new(reader: ReaderId<Keycode>) -> Self {
        ControlSystem{ reader: reader }
    }
}

impl<'a> System<'a> for ControlSystem {
    type SystemData = (
        WriteStorage<'a, Isometry>,
        WriteStorage<'a, Velocity>,
        WriteStorage<'a, Spin>,
        ReadStorage<'a, CharacterMarker>,
        Read<'a, EventChannel<Keycode>>,
        Read<'a, Mouse>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            isometries, 
            mut velocities, 
            mut spins,
            character_markers, 
            keys_channel,
            mouse_state,
        ) = data;
        // TODO add dt in params
        let dt = 1f32 / 60f32;
        for (iso, vel, spin, _) in (&isometries, &mut velocities, &mut spins, &character_markers).join(){
            let player_torque = dt * calculate_player_ship_spin_for_aim(
                Vector2::new(mouse_state.x, mouse_state.y) -
                Vector2::new(iso.0.translation.vector.x, iso.0.translation.vector.y),
                iso.rotation(), 
                spin.0
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
                    Keycode::Up | Keycode::W =>  {
                        vel.0.y = (vel.0.y + THRUST_FORCE).max(-VELOCITY_MAX);
                    }
                    Keycode::Down | Keycode::S => {
                        vel.0.y = (vel.0.y - THRUST_FORCE).min(VELOCITY_MAX);
                    }
                    _ => ()
                }
            }
        }
    }
}