mod gui;
mod run;
mod setup;
mod systems;
#[cfg(test)]
mod test;

extern crate cfg_if;
extern crate slog;
extern crate slog_async;
extern crate slog_scope;
extern crate slog_stdlog;
extern crate slog_term;
pub use common;
pub use derive_deref;
pub use fnv;
pub use gfx_h;
pub use nalgebra;
pub use ncollide2d;
pub use nphysics2d;
pub use num_enum;
pub use rand;
pub use sdl2;
pub use shrev;
pub use specs;
pub use specs_derive;
pub use voronois;
pub use once_cell;

/// int SDL_main(int argc, char *argv[])
#[no_mangle]
pub extern "C" fn SDL_main(
    _argc: libc::c_int,
    _argv: *const *const libc::c_char,
) -> libc::c_int {
    main().unwrap();
    return 0;
}

pub fn main() -> Result<(), String> {
    run::run()
}
