use std::time::Duration;
use std::path::Path;

use astro_lib as al;
use astro_lib::prelude::*;
use al::gfx::{draw_image};
use al::types::{*};
mod components;
mod systems;

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::image::{LoadTexture, InitFlag};

#[no_mangle]
pub extern fn main() -> Result<(), String> {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
 
    let window = video_subsystem.window("rust-sdl2 demo", 800, 600)
        .opengl()
        .position_centered()
        .build()
        .unwrap();
    let gl_context = window.gl_create_context().unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    
    canvas.set_draw_color(Color::RGB(0, 255, 255));
    canvas.clear();
    canvas.present();
    let image_path = Path::new("assets/player.png");
    
    let texture_creator : TextureCreator<_> = canvas.texture_creator();
    let texture = texture_creator.load_texture(image_path)?;
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut i = 0;
    'running: loop {
        i = (i + 1) % 255;
        canvas.set_draw_color(Color::RGB(i, 64, 255 - i));
        canvas.clear();
        draw_image(&mut canvas, Point2::new(0f32, 0f32), Vector2::new(100f32, 100f32), &texture)?;
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running;
                },
                _ => {}
            }
        }
        // The rest of the game loop goes here...

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
    Ok(())
}