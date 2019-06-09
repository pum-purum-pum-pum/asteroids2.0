use astro_lib as al;
use al::prelude::*;

use specs::{Join};
use specs::prelude::*;
use shrev::EventChannel;
use sdl2::keyboard::Keycode;
use glium::Surface;
use glium;
use nalgebra::{Isometry3, Vector3};

use crate::components::{*};
use crate::gfx::{ImageData};

#[derive(Debug)]
pub enum Geometry {
    Circle{
        radius: f32,
        position: Point2,
    },
}

/// Polygon for light rendering(just render light on this rctngl)
/// coordinates are in world 3d space
/// Orientation is clockwise
#[derive(Debug)]
pub struct LightningPolygon {
    points: Vec<Point2>,
    center: Point2 // position of the light
}

impl LightningPolygon {
    pub fn new_rectangle(x_min: f32, y_min: f32, x_max: f32, y_max: f32) -> Self {
        // by default we have one big rectangle with no clipping(shadows)
        LightningPolygon {
            points: vec![
                Point2::new(x_min, y_min),
                Point2::new(x_min, y_max),
                Point2::new(x_max, y_max),
                Point2::new(x_max, y_min)
            ],
            center: Point2::new((x_min + x_max) / 2f32, (y_min + y_max) / 2f32)
        }
    }

    pub fn new_clipped_rectangle(
        x_min: f32, y_min: f32, x_max: f32, y_max: f32, 
        shapes: &[Geometry],
    ) -> Self {
        let mut poly = LightningPolygon::new_rectangle(x_min, y_min, x_max, y_max);

        poly
    }

    pub fn update_center(&mut self, point: Point2) {
        self.center = point;
    }

    pub fn get_triangles(&mut self) -> (Vec<Point2>, Vec<u16>) {
        let mut points = vec![];
        points.push(self.center);
        for i in 0..self.points.len() {
            points.push(self.points[i].clone());
        };
        let mut indicies = vec![];
        for i in 1..self.points.len() {
            indicies.push(0u16);
            indicies.push(i as u16);
            indicies.push(((i + 1) % self.points.len()) as u16);
        };
        (points, indicies)
    }


    pub fn clip_one(&mut self, geom: Geometry) {
        // PLAN
        // Write clipping with segment
        // then write other primitives with that
        // create rays from center to shape borders
        match geom {
            Geometry::Circle {
                radius,
                position
            } => {
                let (dir1, dir2) = {
                    // kludge for test how it's working
                    let rotation1 = Rotation2::new(0.3f32);
                    let rotation2 = Rotation2::new(-0.3f32);
                    let dir = Vector2::new(
                        position.x - self.center.x,
                        position.y - self.center.y
                    );
                    (rotation1 * dir, rotation2 * dir)
                };
                let shape_point1 = self.center + dir1;
                let shape_point2 = self.center + dir2;
                dbg!((&dir1, &dir2));
                let ray1 = Ray::new(self.center, dir1);
                let ray2 = Ray::new(self.center, dir2);
                // find two points where rays intersect
                let (point1, pid1, point2, pid2) = {
                    // pid1 -- first point of the edge
                    // pid2 -- SECOND point of the edge
                    let mut pi_result1 = None;
                    let mut pi_result2 = None;
                    let mut point_id1 = None;
                    let mut point_id2 = None;
                    for i in 0..self.points.len() {
                        let p1 = self.points[i];
                        let p2 = self.points[(i + 1) % self.points.len()];
                        let segment = Segment::new(p1, p2);
                        let point_intersect1 = segment.toi_with_ray(&Isometry2::identity(), &ray1, true);
                        let point_intersect2 = segment.toi_with_ray(&Isometry2::identity(), &ray2, true);
                        match point_intersect1 {
                            Some(pi) => {
                                pi_result1 = Some(ray1.point_at(pi));
                                point_id1 = Some(i);
                            }
                            None => ()
                        }
                        match point_intersect2 {
                            Some(pi) => {
                                pi_result2 = Some(ray2.point_at(pi));
                                point_id2 = Some(i + 1);
                            }
                            None => ()
                        }
                    }
                    (pi_result1.unwrap(), point_id1.unwrap(), pi_result2.unwrap(), point_id2.unwrap())
                };
                dbg!((pid1, pid2));
                assert!(pid1 <= pid2 || pid1 < self.points.len() && pid2 == 0);
                // insert these two new points in polygon
                self.points.insert(pid1 + 1, point1);
                self.points.insert(pid1 + 2, shape_point1);
                self.points.insert(pid1 + 3, shape_point2);
                self.points.insert(pid2 + 3, point2);
                // so our new points are in pid1 + 1 and pid2 + 1
                // remove all points in polygon between them
                dbg!((pid1 + 3, pid2 + 3));
                let mut index = -1i32;
                self.points.retain(|_| {index+=1; !(index > pid1 as i32 + 3 && index < pid2 as i32 + 3)});
                
            }
        }
        // for p in self.points.iter() {

        // }
    }
}