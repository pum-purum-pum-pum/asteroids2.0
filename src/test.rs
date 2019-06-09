use crate::nalgebra::Rotation2;

#[test]
fn rotation() {
    let rot1 = Rotation2::new(1.5 * 3.14);
    let rot2 = Rotation2::new(0.5 * 3.14);
    dbg!((rot1.angle(), rot2.angle()));
}

#[test]
fn geom() {
    use astro_lib as al;
    use al::prelude::*;
    use crate::geometry::{*};

    let mut poly = LightningPolygon::new_rectangle(0f32, 0f32, 1f32, 1f32);
    dbg!(&poly);
    poly.clip_one(Geometry::Circle{
        radius: 1f32,
        position: Point2::new(0.75, 0.75)
    });
    dbg!(&poly);
}