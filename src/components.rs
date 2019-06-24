use std::ops::AddAssign;

pub use crate::geometry::Polygon;
pub use crate::physics::{BodiesMap, PhysicsComponent};
pub use crate::gfx::{ImageData};
pub use crate::sound::{SoundData};
use al::prelude::*;
use astro_lib as al;
use sdl2::mixer::Chunk;
use specs::prelude::*;
use specs_derive::Component;

use crate::gfx::{unproject_with_z, Canvas as SDLCanvas};
use crate::gfx_backend::SDL2Facade;

pub type SDLDisplay = ThreadPin<SDL2Facade>;
pub type Canvas<'a> = ThreadPin<SDLCanvas<'a>>;

#[derive(Default, Debug)]
pub struct Stat {
    pub asteroids_number: u8,
}

#[derive(Component, Debug, Clone, Copy)]
pub enum Geometry {
    Circle { radius: f32 },
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Size(pub f32);

/// Index of Images structure
#[derive(Component, Clone, Copy)]
pub struct Image(pub specs::Entity);

#[derive(Component, Clone, Copy)]
pub struct Sound(pub specs::Entity);

#[derive(Component, Clone, Copy)]
pub struct Particles(pub usize);

#[derive(Component, Clone, Copy)]
pub struct Lifes(pub usize);

#[derive(Component, Clone, Copy)]
pub struct Shields(pub usize);

// pub type Images = Collector<ImageData, Image>;

/// contains preloaded images ids
/// use it when you need to insert entity in system
pub struct PreloadedImages {
    pub projectile: specs::Entity,
    pub asteroid: specs::Entity,
    pub enemy: specs::Entity,
    pub background: specs::Entity,
}

pub struct PreloadedParticles {
    pub movement: specs::Entity,
}

#[derive(Default, Debug)]
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
    pub fn set_position(&mut self, x: i32, y: i32, observer: Point3, width_u: u32, height_u: u32) {
        let (width, height) = (width_u as f32, height_u as f32);
        // dpi already multiplyed
        let (x, y) = (x as f32, y as f32);
        let (x, y) = (2f32 * x / width - 1f32, 2f32 * y / height - 1f32);
        // with z=0f32 -- which is coordinate of our canvas in 3d space
        let point = unproject_with_z(observer, &Point2::new(x, -y), 0f32, width_u, height_u);
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

#[derive(Default, Component)]
#[storage(NullStorage)]
pub struct ShipMarker;

#[derive(Default, Component)]
#[storage(NullStorage)]
pub struct EnemyMarker;

#[derive(Default, Component)]
#[storage(NullStorage)]
pub struct AsteroidMarker;

#[derive(Default, Component)]
#[storage(NullStorage)]
pub struct LightMarker;

#[derive(Component)]
pub struct Projectile {
    pub owner: specs::Entity,
}

#[derive(Component)]
pub struct Lifetime {
    life_state: u8,
    life_time: u8,
}

impl Lifetime {
    pub fn new(live_time: u8) -> Self {
        Lifetime {
            life_state: 0u8,
            life_time: live_time,
        }
    }

    pub fn update(&mut self) {
        self.life_state = u8::min(self.life_time, self.life_state + 1u8);
    }

    pub fn delete(&self) -> bool {
        self.life_state >= self.life_time
    }
}

/// attach entity positions to some other entity position
#[derive(Component, Debug)]
pub struct AttachPosition(pub specs::Entity);

/// gun reloading status and time
#[derive(Component, Debug)]
pub struct Gun {
    recharge_state: u8,
    recharge_time: u8,
}

impl Gun {
    pub fn new(recharge_time: u8) -> Self {
        Gun {
            recharge_state: 0u8,
            recharge_time: recharge_time,
        }
    }

    pub fn update(&mut self) {
        self.recharge_state = u8::min(self.recharge_time, self.recharge_state + 1u8);
    }

    pub fn is_ready(&self) -> bool {
        self.recharge_state >= self.recharge_time
    }

    pub fn shoot(&mut self) -> bool {
        let result = self.is_ready();
        if result {
            self.recharge_state = 0u8
        };
        result
    }
}

/// translation + rotation
#[derive(Component, Debug, Clone, Copy)]
pub struct Isometry(pub Isometry3);

/// rotation speed
#[derive(Component, Default, Debug)]
pub struct Spin(pub f32);

impl Isometry {
    pub fn new(x: f32, y: f32, angle: f32) -> Self {
        Isometry(Isometry3::new(
            Vector3::new(x, y, 0f32),
            Vector3::new(0f32, 0f32, angle),
        ))
    }

    pub fn add_spin(&mut self, spin: f32) {
        let rotation = Rotation3::new(Vector3::new(0f32, 0f32, spin));
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

#[derive(Copy, Clone, Component, Debug)]
pub struct Velocity(pub Vector2);

impl AddAssign<Velocity> for &mut Velocity {
    fn add_assign(&mut self, other: Velocity) {
        self.0.x += other.0.x;
        self.0.y += other.0.y;
    }
}

impl Velocity {
    pub fn new(x: f32, y: f32) -> Self {
        Velocity(Vector2::new(x, y))
    }
}
