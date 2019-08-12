use crate::components::{Geometry, BlockSegment};
use crate::types::{*};
use ncollide2d::transformation::convex_hull_idx;
use rand::prelude::*;
use specs::prelude::*;
use specs_derive::Component;

pub const EPS: f32 = 1E-3;
pub const SHADOW_LENGTH: f32 = 100f32;

pub struct NebulaGrid {
    pub grid: Grid<bool>
}

impl NebulaGrid {
    pub fn new(n: usize, rw: f32, rh: f32, rbw: f32, rbh: f32) -> Self {
        let grid = Grid::new(n, rw, rh, rbw, rbh);
        NebulaGrid {
            grid: grid
        }
    }
}

pub struct PlanetGrid {
    pub grid: Grid<bool>
}

impl PlanetGrid {
    pub fn new(n: usize, rw: f32, rh: f32, rbw: f32, rbh: f32) -> Self {
        let grid = Grid::new(n, rw, rh, rbw, rbh);
        PlanetGrid {
            grid: grid
        }
    }
}


pub struct Grid<T> {
    bricks: Vec<T>,
    x: f32, 
    y: f32,
    rw: f32,
    rh: f32,
    rbw: f32,
    rbh: f32,
    pub max_w: f32, 
    pub max_h: f32,
    pub n: usize,
    pub size: usize,
}

impl<T> Grid<T> where T: Default + Clone {
    pub fn new(n: usize, rw: f32, rh: f32, rbw: f32, rbh: f32) -> Self {
        let size = 2 * n + 1;
        let bricks = vec![T::default(); size * size];
        Self {
            bricks: bricks,
            x: 0.0,
            y: 0.0,
            rw: rw,
            rh: rh,
            rbw: rbw,
            rbh: rbh,
            max_w: rw + 2.0 * n as f32 * rw,
            max_h: rh + 2.0 * n as f32 * rh,
            n: n,
            size: size
        }
    }

    pub fn shift(&mut self, x: f32, y: f32) {
        self.x += x;
        self.y += y;
        // shift grid by one cell if necessary
        if self.x > self.rw + EPS {
            self.x = -self.rw
        }
        if self.x < -self.rw - EPS {
            self.x = self.rw
        }
        if self.y > self.rh + EPS {
            self.y = -self.rh
        }
        if self.y < -self.rh - EPS {
            self.y = self.rh
        }
        self.bricks = vec![T::default(); self.bricks.len()];
    }

    pub fn get_column(&self, x: f32) -> usize {
        (self.n as i32 + ((self.rw + x - self.x) / (2.0 * self.rw)).floor() as i32) as usize
    }

    pub fn get_row(&self, y: f32) -> usize {
        // assert_debug!();
        (self.n as i32 + ((self.rh + y - self.y) / (2.0 * self.rh)).floor() as i32) as usize
    }

    pub fn get(&self, point: Point2) -> &T {
        let id = self.size * self.get_row(point.y) + 
            self.get_column(point.x);
        &self.bricks[id]
    }

    pub fn get_rectangle(&self, row: usize, col: usize) -> ((f32, f32), (f32, f32)) {        
        let point = self.get_cell_point(row, col);
        return (
            (
                point.x - self.rw + self.rbw, 
                point.x + self.rw - self.rbw
            ), 
            (
                point.y - self.rh + self.rbh,
                point.y + self.rh - self.rbh
            )
        )
    }

    pub fn get_ids(&self, id: usize) -> (usize, usize) {
        let row = id / self.size;
        let col = id % self.size;
        (row, col)        
    }

    pub fn get_cell_point(&self, row: usize, column: usize) -> Point2 {
        let (row, column) = (row as f32 - self.n as f32, column as f32 - self.n as f32);
        Point2::new(
            self.x + column * 2.0 * self.rw, 
            self.y + row * 2.0 * self.rh
        )
    }

    pub fn get_cell_value(&self, row: usize, column: usize) -> &T {
        &self.bricks[row * self.size + column]
    }

    pub fn reset(&mut self) {
        self.bricks = vec![T::default(); self.bricks.len()];
    }

    pub fn update(&mut self, point: Point2, value: T) -> Result<(), ()> {
        if (point.x - self.x).abs() < self.max_w && (point.y - self.y).abs() < self.max_h {
            let id = self.size * self.get_row(point.y) + 
            self.get_column(point.x);
            self.bricks[id] = value;
            return Ok(())
        }
        Err(())
    }
}

pub fn generate_convex_polygon(samples_num: usize, size: f32) -> Polygon {
    let mut rng = thread_rng();
    let mut points = vec![];
    for _ in 0..samples_num {
        let x = rng.gen_range(-size, size);
        // sample from circle
        let chord = (size * size - x * x).sqrt();
        let y = rng.gen_range(-chord, chord);
        points.push(Point2::new(x, y));
    }
    let ids = convex_hull_idx(&points);
    // TODO opt: inplace
    let points = {
        let mut res = vec![];
        for &i in ids.iter() {
            res.push(points[i])
        }
        res
    };
    Polygon::new(points)
}

// @vlad TODO refactor (it's copy paste from stack overflow)
/// get tangent to circle from point
pub fn get_tangent(circle: Point2, r: f32, point: Point2) -> (Option<Point2>, Option<Point2>) {
    let npoint = (point - circle) / r;
    let xy = npoint.norm_squared();
    if xy - 1.0 <= EPS {
        return (None, None);
    }
    let mut discr = npoint.y * (xy - 1f32).sqrt();
    let tx0 = (npoint.x - discr) / xy;
    let tx1 = (npoint.x + discr) / xy;
    let (yt0, yt1) = if npoint.y != 0f32 {
        (
            circle.y + r * (1f32 - tx0 * npoint.x) / npoint.y,
            circle.y + r * (1f32 - tx1 * npoint.x) / npoint.y,
        )
    } else {
        discr = r * (1f32 - tx0 * tx0).sqrt();
        (circle.y + discr, circle.y - discr)
    };
    let xt0 = circle.x + r * tx0;
    let xt1 = circle.x + r * tx1;
    (Some(Point2::new(xt0, yt0)), Some(Point2::new(xt1, yt1)))
}

#[derive(Debug)]
pub struct Triangulation {
    pub points: Vec<Point2>,
    pub indicies: Vec<u16>,
}

pub trait TriangulateFromCenter {
    fn points(&self) -> &[Point2];

    fn center(&self) -> Point2;

    fn triangulate(&self) -> Triangulation {
        let mut points = vec![];
        points.push(self.center());
        for i in 0..self.points().len() {
            points.push(self.points()[i].clone());
        }
        let mut indicies = vec![];
        for i in 1..points.len() {
            indicies.push(0u16);
            indicies.push(i as u16);
            let mut si = i as u16 + 1u16;
            if si == points.len() as u16 {
                si = 1u16
            };
            indicies.push(si);
        }
        Triangulation {
            points: points,
            indicies: indicies,
        }
    }
}

#[derive(Debug, Component, Clone)]
pub struct Polygon {
    pub points: Vec<Point2>,
    mass_center: Point2,
    pub min_r: f32,
}

impl Polygon {
    // pub fn get_block_polygon(&self) -> BlockPolygon {
    //     let mut rng = thread_rng();
    //     let mut segments = vec![];
    //     for i in 0..self.points.len() {
    //         let p1 = self.points[i];
    //         let p2 = self.points[(i + 1) % self.points.len()];
    //         // TODO: it's kludge for clipping algo which has some bug when working with collision points :)
    //         // FIX ME
    //         let noise = Vector2::new(rng.gen_range(0.02, 0.05), rng.gen_range(0.02, 0.05));
    //         let block_segment = BlockSegment {
    //             point1: p1 - rvec2(self.mass_center) + noise,
    //             point2: p2 - rvec2(self.mass_center) + noise
    //         };
    //         segments.push(block_segment);
    //     }
    //     BlockPolygon {
    //         segments: segments
    //     }
    // }

    pub fn new(points: Vec<Point2>) -> Self {
        let w = 1.0 / (points.len() as f32);
        let mut center = Point2::new(0f32, 0f32);
        for p in points.iter() {
            center.x += w * p.x;
            center.y += w * p.y;
        }
        let mut min_r = 10f32;
        for p in points.iter() {
            min_r = min_r.min((p - center).norm())
        }
        Polygon {
            points: points,
            mass_center: Point2::new(0f32, 0f32),
            min_r,
        }
    }

    pub fn deconstruct(&self) -> Vec<Polygon> {
        let mut res = vec![];
        if self.points.len() == 3 {
            return vec![self.clone()];
        }
        // dummy destruct for now
        let triangulation = self.triangulate();
        let points = triangulation.points;
        let indicies = triangulation.indicies;
        let mut i = 0usize;
        while i < indicies.len() {
            res.push(Polygon::new(vec![
                points[indicies[i] as usize],
                points[indicies[i + 1] as usize],
                points[indicies[i + 2] as usize],
            ]));
            i += 3;
        }
        res
    }
}

impl TriangulateFromCenter for Polygon {
    fn points(&self) -> &[Point2] {
        &self.points
    }

    fn center(&self) -> Point2 {
        self.mass_center
    }
}

/// Polygon for light rendering(just render light on this rctngl)
/// coordinates are in world 3d space
/// Orientation is clockwise
#[derive(Debug)]
pub struct LightningPolygon {
    pub points: Vec<Point2>,
    x_min: f32,
    y_min: f32,
    x_max: f32,
    y_max: f32,
    pub center: Point2, // position of the light
}

impl TriangulateFromCenter for LightningPolygon {
    fn points(&self) -> &[Point2] {
        &self.points
    }

    fn center(&self) -> Point2 {
        self.center
    }
}

pub fn cross(a: Vector2, b: Vector2) -> f32 {
    a.x * b.y - a.y * b.x
}

fn x_angle(vec: Vector2) -> f32 {
    let a = vec.y.atan2(vec.x);
    // if a < 0.0 {
    //     2.0 * std::f32::consts::PI + a
    // } else {
    //     a
    // }
    a
}

pub fn poly_to_segment(poly: Polygon, position: Point2) -> BlockSegment {
    let points= &poly.points;
    let rotation = Rotation2::rotation_between(&(points[0].coords + position.coords), &Vector2::x_axis());
    let mut point1 = points[0];
    let mut angle1 = x_angle(rotation * (point1.coords + position.coords));
    let mut point2 = points[0];
    let mut angle2 = x_angle(rotation * (point2.coords + position.coords));
    for point in points.iter() {
        let cur = rotation * (point.coords + position.coords);
        let angle = x_angle(cur);
        if angle < angle1 {
            angle1 = angle;
            point1 = *point;
        };
        if angle > angle2 {
            angle2 = angle;
            point2 = *point;
        }
    };
    BlockSegment {
        point1,
        point2
    }
}

impl LightningPolygon {
    pub fn new_rectangle(x_min: f32, y_min: f32, x_max: f32, y_max: f32, center: Point2) -> Self {
        // by default we have one big rectangle with no clipping(shadows)
        LightningPolygon {
            points: vec![
                Point2::new(x_min, y_min),
                Point2::new(x_min, y_max),
                Point2::new(x_max, y_max),
                Point2::new(x_max, y_min),
            ],
            x_min,
            y_min,
            x_max,
            y_max,
            center: center,
        }
    }

    fn clip_segment(
        &mut self, 
        BlockSegment{ point1: mut shape_point1, point2: mut shape_point2}: BlockSegment, 
    ) {
        if cross(shape_point1.coords, shape_point2.coords) > 0f32 {
            std::mem::swap(&mut shape_point1, &mut shape_point2)
        }
        let dir1 = shape_point1.coords - self.center.coords;
        let dir2 = shape_point2.coords - self.center.coords;
        let ray1 = Ray::new(self.center, dir1);
        let ray2 = Ray::new(self.center, dir2);
        let (point1, pid1, point2, pid2) = {
            // pid1 -- first point of the edge
            // pid2 -- first point of the edge
            let mut pi_result1 = None;
            let mut pi_result2 = None;
            let mut point_id1 = None;
            let mut point_id2 = None;
            for i in 0..self.points.len() {
                let j = (i + 1) % self.points.len();
                let p1 = self.points[i];
                let p2 = self.points[j];
                let segment = Segment::new(p1, p2);
                let point_intersect1 =
                    segment.toi_with_ray(&Isometry2::identity(), &ray1, true);
                let point_intersect2 =
                    segment.toi_with_ray(&Isometry2::identity(), &ray2, true);
                match point_intersect1 {
                    Some(pi) => {
                        pi_result1 = Some(ray1.point_at(pi));
                        point_id1 = Some(i);
                    }
                    None => (),
                }
                match point_intersect2 {
                    Some(pi) => {
                        pi_result2 = Some(ray2.point_at(pi));
                        point_id2 = Some(j);
                    }
                    None => (),
                }
            }
            if pi_result1.is_none() || pi_result2.is_none() {
                return;
            };
            (
                pi_result1.unwrap(),
                point_id1.unwrap(),
                pi_result2.unwrap(),
                point_id2.unwrap(),
            )
        };
        if pid1 <= pid2 {
            self.points.insert(pid1 + 1, point1);
            self.points.insert(pid1 + 2, shape_point1);
            self.points.insert(pid1 + 3, shape_point2);
            self.points.insert((pid2 + 3) % self.points.len(), point2);
            // remove all points in polygon between them
            let mut index = -1i32;
            self.points.retain(|_| {
                index += 1;
                !(index > pid1 as i32 + 3 && index < pid2 as i32 + 3)
            });
        } else {
            self.points.insert(pid2, point2);
            self.points.insert(pid1 + 2, point1);
            self.points.insert(pid1 + 3, shape_point1);
            self.points.insert(pid1 + 4, shape_point2);
            let mut index = -1i32;
            self.points.retain(|_| {
                index += 1;
                !(index < pid2 as i32 || index > pid1 as i32 + 4)
            });
        }
    }

    // slow-ugly-simple version of polygon clipping
    pub fn clip_one(&mut self, geom: Geometry, position: Point2) {
        // PLAN
        // Write clipping with segment
        // then write other primitives with that
        // create rays from center to shape borders

        match geom {
            Geometry::Circle { radius } => {
                let (dir1, dir2) = match get_tangent(position, radius, self.center) {
                    (Some(p1), Some(p2)) => (
                        Vector2::new(p2.x - self.center.x, p2.y - self.center.y),
                        Vector2::new(p1.x - self.center.x, p1.y - self.center.y),
                    ),
                    _ => return,
                };
                let shape_point1 = self.center + dir1;
                let shape_point2 = self.center + dir2;
                self.clip_segment(
                    BlockSegment{point1: shape_point1, point2: shape_point2}, 
                );
            }
            Geometry::Segment(block_segment) => {
                self.clip_segment(
                    block_segment,
                )
            }
            Geometry::Polygon(block_polygon) => {
                let BlockSegment {
                    point1,
                    point2
                } = poly_to_segment(block_polygon, position);
                self.clip_segment(
                    BlockSegment {
                        point1: point1 + position.coords,
                        point2: point2 + position.coords
                    },
                )
                // for segment in block_polygon.segments.iter() {
                //     self.clip_segment(
                //         BlockSegment{
                //             point1: segment.point1 + rvec2(position),
                //             point2: segment.point2 + rvec2(position)
                //         }, 
                //     );
                // }
            }
        }
    }
}


pub fn shadow_geometry(
    center: Point2, geom: Geometry, position: Point2, rotation: Rotation2<f32>
) -> Option<Triangulation> {
    let segment = match geom {
        Geometry::Circle { radius } => {
            let dirs = match get_tangent(position, radius, center) {
                (Some(p1), Some(p2)) => Some((
                    Vector2::new(p2.x - center.x, p2.y - center.y),
                    Vector2::new(p1.x - center.x, p1.y - center.y),
                )),
                _ => None, // TODO handle this or what?
            };
            if let Some((dir1, dir2)) = dirs {
                let shape_point1 = center + dir1;
                let shape_point2 = center + dir2;
                Some(BlockSegment{point1: shape_point1, point2: shape_point2})
            } else {
                None
            }
        }
        Geometry::Segment(block_segment) => {
            Some(block_segment)
        }
        Geometry::Polygon(mut block_polygon) => {
            let points: Vec<Point2> = block_polygon.points.iter().map(|x| rotation * x).collect();
            block_polygon.points = points;
            Some(poly_to_segment(block_polygon, position))
        }
    };
    if let Some(segment) = segment {
        let dir1 = segment.point1.coords + position.coords - center.coords;
        let dir2 = segment.point2.coords + position.coords - center.coords;
        let points = vec![
            segment.point1,
            segment.point2,
            segment.point1 + SHADOW_LENGTH * dir1,
            segment.point2 + SHADOW_LENGTH * dir2
        ];
        let indicies = vec![0, 2, 3, 0, 3, 1];
        Some(Triangulation {
            points: points,
            indicies: indicies,
        })
    } else {
        None
    }
}