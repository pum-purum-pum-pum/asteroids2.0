mod gui;
mod run;
mod systems;
#[cfg(test)]
mod test;

pub use common;
pub use derive_deref;
pub use fnv;
pub use gfx_h;
pub use nalgebra;
pub use ncollide2d;
pub use nphysics2d;
pub use rand;
pub use sdl2;
pub use shrev;
pub use specs;
pub use specs_derive;
pub use voronois;
#[cfg(any(target_os = "ios", target_os = "android", target_os = "emscripten"))]
#[macro_use]
extern crate log;
#[cfg(any(target_os = "ios", target_os = "android", target_os = "emscripten"))]
extern crate android_log;

// int SDL_main(int argc, char *argv[])
#[no_mangle]
pub extern "C" fn SDL_main(_argc: libc::c_int, _argv: *const *const libc::c_char) -> libc::c_int {
    run::run().unwrap();
    return 0;
}
