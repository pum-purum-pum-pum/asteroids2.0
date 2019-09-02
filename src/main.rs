mod components;
mod geometry;
mod physics;
mod sound;
mod systems;
#[cfg(test)]
mod test;
mod gui;
mod run;
extern crate cfg_if;

pub use common;
pub use gfx_h;
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
pub use vorod;

// int SDL_main(int argc, char *argv[])
#[no_mangle]
pub extern fn SDL_main(_argc: libc::c_int, _argv: *const *const libc::c_char) -> libc::c_int {
  main().unwrap();
  return 0;
}

pub fn main() -> Result<(), String> {
    run::run()
}
