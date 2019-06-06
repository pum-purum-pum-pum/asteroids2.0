use astro_lib as al;
use al::prelude::*;
use al::types::*;
use specs_derive::{Component};
use specs::prelude::*;
use crate::gfx_backend::SDL2Facade;
use crate::gfx::Canvas as SDLCanvas;


pub type SDLDisplay = ThreadPin<SDL2Facade>;
pub type Canvas = ThreadPin<SDLCanvas>;


#[derive(Default, Component)]
#[storage(NullStorage)]
pub struct CharacterMarker;

#[derive(Component, Debug)]
pub struct Position(pub Point2);

impl Position {
    pub fn new(x: f32, y: f32) -> Self{
        Position(Point2::new(x, y))
    }
}

#[derive(Component, Debug)]
pub struct Velocity(pub Vector2);

impl Velocity {
    pub fn new(x: f32, y: f32) -> Self{
        Velocity(Vector2::new(x, y))
    }
}
