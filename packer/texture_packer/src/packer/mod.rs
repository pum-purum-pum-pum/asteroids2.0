use frame::Frame;
use texture::{Pixel, Texture};

pub use self::skyline_packer::SkylinePacker;

mod skyline_packer;

pub trait Packer {
    type Pixel: Pixel;

    fn pack(
        &mut self,
        key: String,
        texture: &Texture<Pixel = Self::Pixel>,
    ) -> Option<Frame>;
    fn can_pack(&self, texture: &Texture<Pixel = Self::Pixel>) -> bool;
}
