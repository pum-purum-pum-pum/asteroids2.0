use ncollide2d::shape::ShapeHandle;
use ncollide2d::world::CollisionGroups;
use nphysics2d::object::BodyStatus;
use nphysics2d::world::World;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use shrev::EventChannel;
use specs::prelude::*;
use specs::World as SpecsWorld;
use red::{self, GL, Frame, DrawType, glow};
use red::glow::RenderLoop;
use std::panic;

mod components;
mod geometry;
mod gfx;
mod physics;
mod sound;
mod systems;
#[cfg(test)]
mod test;
mod gui;
mod types;
mod run;

use backtrace;
#[macro_use]
pub use specs_derive;
pub use specs;
pub use sdl2;
pub use shrev;
pub use rand;
pub use fnv;
pub use derive_deref;
pub use nalgebra;
pub use nphysics2d;
pub use ncollide2d;
use crate::types::{*};
use components::*;
use gfx::{Canvas};
use physics::{safe_maintain, CollisionId, PHYSICS_SIMULATION_TIME};
use sound::init_sound;
use systems::{
    AISystem, CollisionSystem, ControlSystem, GamePlaySystem, InsertSystem,
    KinematicSystem, PhysicsSystem, RenderingSystem, SoundSystem, MenuRenderingSystem,
    GUISystem,
};
use gui::{IngameUI, Primitive};
#[cfg(any(target_os = "ios", target_os = "android", target_os = "emscripten"))]
#[macro_use] extern crate log;
#[cfg(any(target_os = "ios", target_os = "android", target_os = "emscripten"))]
extern crate android_log;

// int SDL_main(int argc, char *argv[])
#[no_mangle]
pub extern fn SDL_main(_argc: libc::c_int, _argv: *const *const libc::c_char) -> libc::c_int {
  run::run().unwrap();
  return 0;
}
