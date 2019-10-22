use red;

use common::*;
use gfx_h::Canvas;
use rand::prelude::*;
use red::glow::RenderLoop;
use red::{glow, Frame, GL};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;

mod plot;
use plot::{render_plot, TeleGraph};

fn main() -> Result<(), String> {
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();
    let (_ddpi, _hdpi, _vdpi) = video.display_dpi(0i32)?;
    let gl_attr = video.gl_attr();
    gl_attr.set_context_version(3, 3);
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    let (window_w, window_h) = (1920u32, 1080);
    let viewport = red::Viewport::for_window(window_w as i32, window_h as i32);
    let window = video
        .window("tele-graph", window_w, window_h)
        // .fullscreen()
        .opengl()
        .resizable()
        .build()
        .unwrap();
    let _gl_context = window.gl_create_context().unwrap();
    let render_loop =
        glow::native::RenderLoop::<sdl2::video::Window>::from_sdl_window(
            window,
        );
    let context = glow::native::Context::from_loader_function(|s| {
        video.gl_get_proc_address(s) as *const _
    });
    let context = GL::new(context);
    let glsl_version = "#version 330";
    let canvas = Canvas::new(&context, "", &glsl_version).unwrap();
    let mut frame = Frame::new(&context);
    let mut event_loop = sdl_context.event_pump().unwrap();
    let mut telegraph = TeleGraph::new(Duration::from_secs(10));
    let (w, h) = (16f32, 9f32);
    telegraph.set_color("plot a".to_string(), Point3::new(1.0, 1.0, 1.0));
    telegraph.set_color("plot b".to_string(), Point3::new(0.0, 1.0, 0.0));
    render_loop.run(move |running: &mut bool| {
        frame.set_clear_color(0.015, 0.004, 0.0, 1.0);
        frame.clear_color_and_stencil();
        let mut rng = rand::thread_rng();
        telegraph.update();
        if rng.gen_range(0.0, 1.0) < 0.06 {
            telegraph.insert("plot a".to_string(), rng.gen_range(0.0, 1.0));
            telegraph.insert("plot b".to_string(), rng.gen_range(0.0, 1.0));
        }
        for name in telegraph.iter_names() {
            if let Some(plot) = telegraph.iter(name.to_string()) {
                render_plot(
                    plot.0, plot.1, w, h, &context, &viewport, &canvas,
                    &mut frame,
                );
            }
        }
        for event in event_loop.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => *running = false,
                _ => {}
            }
        }

        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
        // The rest of the game loop goes here...
    });
    Ok(())
}
