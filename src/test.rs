use crate::nalgebra::Rotation2;

#[test]
fn rotation() {
    let rot1 = Rotation2::new(1.5 * 3.14);
    let rot2 = Rotation2::new(0.5 * 3.14);
    dbg!((rot1.angle(), rot2.angle()));
}

#[test]
fn geom() {
    use crate::geometry::*;
    use crate::components::*;
    use al::prelude::*;
    use astro_lib as al;

    let mut poly = LightningPolygon::new_rectangle(-5f32, 5f32, -5f32, 5f32, Point2::new(0.5, 0.5));
    dbg!(&poly);
    poly.clip_one(
        Geometry::Circle {
            radius: 1f32,
        },
        Point2::new(0f32, 4f32)
    );
    dbg!(&poly);
}
