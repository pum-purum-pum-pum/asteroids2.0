use crate::types::{Point2, Vector2};
use sdl2::render::{Canvas};
use sdl2::rect::Rect;

pub fn draw_image(
    canvas: &mut Canvas<sdl2::video::Window>, 
    left_bot: Point2,
    size: Vector2,
    image: &sdl2::render::Texture<'_>,
) -> Result<(), String>{
    canvas.copy(image, None, Some(Rect::new(left_bot.x.round() as i32, left_bot.y.round() as i32, size.x.round() as u32, size.y.round() as u32)))?;
    Ok(())
}