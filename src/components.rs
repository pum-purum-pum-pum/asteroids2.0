use std::ops::{AddAssign, MulAssign};
use astro_lib as al;
use al::prelude::*;
use al::types::*;
use specs_derive::{Component};
use specs::prelude::*;
use crate::gfx_backend::SDL2Facade;
use crate::gfx::{Canvas as SDLCanvas, unproject_with_z};


pub type SDLDisplay = ThreadPin<SDL2Facade>;
pub type Canvas = ThreadPin<SDLCanvas>;

#[derive(Default)]
pub struct Mouse {
    pub x: f32,
    pub y: f32,
    pub left: bool,
    pub right: bool,
    pub wdpi: f32,
    pub hdpi: f32,
}

impl Mouse {
    /// get system location of mouse and then unproject it into canvas coordinates
    pub fn set_position(
        &mut self, 
        x: i32, 
        y: i32, 
        observer: &Isometry3,
        width_u: u32,
        height_u: u32,
    ) {
        let (width, height) = (width_u as f32, height_u as f32);
        // dpi already multiplyed
        let (x, y) = (x as f32, y as f32);
        dbg!(("before norm", x, y));
        let (x, y) = (
            2f32 * x / width - 1f32,
            2f32 * y / height - 1f32,
        );
        // with z=0f32 -- which is coordinate of our canvas in 3d space
        let point = unproject_with_z(
            observer,
            &Point2::new(x, y),
            0f32,
            width_u,
            height_u
        );
        self.x = point.x;
        self.y = point.y;
    }

    pub fn set_left(&mut self, is_left: bool) {
        self.left = is_left;
    }

    pub fn set_right(&mut self, is_right: bool) {
        self.right = is_right;
    }
}

#[derive(Default, Component)]
#[storage(NullStorage)]
pub struct CharacterMarker;

#[derive(Component, Debug)]
pub struct Isometry(pub Isometry3);

#[derive(Component, Default, Debug)]
pub struct Spin(pub f32);

impl Isometry {
    pub fn new(x: f32, y: f32, angle: f32) -> Self{
        Isometry(Isometry3::new(
            Vector3::new(x, y, 0f32),
            Vector3::new(0f32, 0f32, angle),
        ))
    }

    pub fn add_spin(&mut self, spin: f32) {
        let rotation = Rotation3::new(
            Vector3::new(0f32, 0f32, spin)
        );
        self.0.rotation *= rotation;
    }

    /// return clockwise angle in XY plane
    pub fn rotation(&self) -> f32 {
        // self.0.rotation.angle()
        self.0.rotation.euler_angles().2
    }
}

impl AddAssign<&Velocity> for &mut Isometry {
    fn add_assign(&mut self, other: &Velocity) {
        self.0.translation.vector.x += other.0.x;
        self.0.translation.vector.y += other.0.y;
    }
}

#[derive(Component, Debug)]
pub struct Velocity(pub Vector2);

impl Velocity {
    pub fn new(x: f32, y: f32) -> Self{
        Velocity(Vector2::new(x, y))
    }
}
