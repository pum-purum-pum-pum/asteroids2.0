use common::*;
use ncollide2d::transformation::convex_hull_idx;
// use ncollide2d::query::closest_points_line_line_parameters;
use rand::prelude::*;
use specs::prelude::*;
use specs_derive::Component;
use voronois::destruction;

pub const EPS: f32 = 1E-3;
pub const SHADOW_LENGTH: f32 = 100f32;

#[derive(Component, Debug, Clone)]
pub enum Geometry {
    Circle { radius: f32 },
    Polygon(Polygon),
}

#[derive(Component, Debug, Clone, Copy)]
pub struct BlockSegment {
    pub point1: Point2,
    pub point2: Point2,
}

pub struct NebulaGrid {
    pub grid: Grid<bool>,
}

impl NebulaGrid {
    pub fn new(n: usize, rw: f32, rh: f32, rbw: f32, rbh: f32) -> Self {
        let grid = Grid::new(n, rw, rh, rbw, rbh);
        NebulaGrid { grid: grid }
    }
}

pub struct PlanetGrid {
    pub grid: Grid<bool>,
}

impl PlanetGrid {
    pub fn new(n: usize, rw: f32, rh: f32, rbw: f32, rbh: f32) -> Self {
        let grid = Grid::new(n, rw, rh, rbw, rbh);
        PlanetGrid { grid: grid }
    }
}

pub struct StarsGrid {
    pub grid: Grid<bool>,
}

impl StarsGrid {
    pub fn new(n: usize, rw: f32, rh: f32, rbw: f32, rbh: f32) -> Self {
        let grid = Grid::new(n, rw, rh, rbw, rbh);
        StarsGrid { grid: grid }
    }
}

pub struct FogGrid {
    pub grid: Grid<bool>,
}

impl FogGrid {
    pub fn new(n: usize, rw: f32, rh: f32, rbw: f32, rbh: f32) -> Self {
        let grid = Grid::new(n, rw, rh, rbw, rbh);
        FogGrid { grid: grid }
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

impl<T> Grid<T>
where
    T: Default + Clone,
{
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
            size: size,
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
        (self.n as i32
            + ((self.rw + x - self.x) / (2.0 * self.rw)).floor() as i32)
            as usize
    }

    pub fn get_row(&self, y: f32) -> usize {
        (self.n as i32
            + ((self.rh + y - self.y) / (2.0 * self.rh)).floor() as i32)
            as usize
    }

    pub fn get_rectangle(
        &self,
        row: usize,
        col: usize,
    ) -> ((f32, f32), (f32, f32)) {
        let point = self.get_cell_point(row, col);
        return (
            (point.x - self.rw + self.rbw, point.x + self.rw - self.rbw),
            (point.y - self.rh + self.rbh, point.y + self.rh - self.rbh),
        );
    }

    pub fn get_cell_point(&self, row: usize, column: usize) -> Point2 {
        let (row, column) =
            (row as f32 - self.n as f32, column as f32 - self.n as f32);
        Point2::new(
            self.x + column * 2.0 * self.rw,
            self.y + row * 2.0 * self.rh,
        )
    }

    pub fn get_cell_value(&self, row: usize, column: usize) -> &T {
        &self.bricks[row * self.size + column]
    }

    pub fn reset(&mut self) {
        self.bricks = vec![T::default(); self.bricks.len()];
    }

    pub fn update(&mut self, point: Point2, value: T) -> Result<(), ()> {
        if (point.x - self.x).abs() < self.max_w
            && (point.y - self.y).abs() < self.max_h
        {
            let id =
                self.size * self.get_row(point.y) + self.get_column(point.x);
            self.bricks[id] = value;
            return Ok(());
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
pub fn get_tangent(
    circle: Point2,
    r: f32,
    point: Point2,
) -> (Option<Point2>, Option<Point2>) {
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

impl Triangulation {
    pub fn new() -> Self {
        Self {
            points: vec![],
            indicies: vec![],
        }
    }

    pub fn apply(&mut self, isometry: Isometry2) {
        for p in self.points.iter_mut() {
            *p = isometry * *p;
        }
    }

    pub fn translate(&mut self, shift: Vector2) {
        for p in self.points.iter_mut() {
            *p += shift;
        }
    }

    pub fn extend(&mut self, triangulation: Triangulation) {
        self.points.extend(triangulation.points);
        let id_shift = self.points.len() as u16;
        self.indicies
            .extend(triangulation.indicies.iter().map(|x| *x + id_shift));
    }
}

pub trait TriangulateFromCenter {
    fn points(&self) -> &[Point2];

    fn center(&self) -> Point2;

    fn triangulate(&self) -> Triangulation {
        let mut points = vec![];
        points.push(self.center());
        points.extend(self.points().iter());
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
    pub max_r: f32,
    pub width: f32,
    pub height: f32,
}

impl Polygon {
    pub fn into_rounded(self, smooth_points: usize) -> Self {
        let mut res = vec![];
        for i in 0..self.points.len() {
            let prev = self.points[if i == 0 {
                self.points.len() - 1
            } else {
                (i - 1) % self.points.len()
            }];
            let p = self.points[i];
            let next = self.points[(i + 1) % self.points.len()];
            let edge_vec1 = p.coords - prev.coords;
            let edge_vec2 = next.coords - p.coords;
            let segment1 = Segment::new(prev, prev + edge_vec1);
            let inside_vec =
                (-edge_vec1.normalize() + edge_vec2.normalize()) / 2.0;
            let d = 1.0
                * inside_vec
                    .dot(&edge_vec1)
                    .abs()
                    .min(inside_vec.dot(&edge_vec2).abs());
            let inside_vec = d * inside_vec;
            let o = p + inside_vec;
            let mut h1 = Vector2::new(-edge_vec1.y, edge_vec1.x).normalize();
            let mut dbg_flag = false;
            {
                // try different direction of perpendicular
                let ray1 = Ray::new(o, h1);
                let ray2 = Ray::new(o, -h1);
                // dbg!((&ray1, &segment1));
                let toi1 =
                    segment1.toi_with_ray(&Isometry2::identity(), &ray1, true);
                let toi2 =
                    segment1.toi_with_ray(&Isometry2::identity(), &ray2, true);
                match (toi1, toi2) {
                    (Some(toi), _) => {
                        h1 = h1 * toi;
                        dbg_flag = true;
                    }
                    (_, Some(toi)) => {
                        h1 = -h1 * toi;
                        dbg_flag = true;
                    }
                    _ => (),
                }
            };
            if dbg_flag {
                let angle = edge_vec1.angle(&edge_vec2);
                // if smooth_points == 1 {
                //     res.push(o + Rotation2::new(-angle / 2.0) * h1);
                //     continue;
                // }
                let rotation = Rotation2::new(-angle / (smooth_points as f32));
                for _ in 0..=smooth_points {
                    res.push(o + h1);
                    h1 = rotation * h1;
                }
            } else {
                res.push(p)
            }
        }
        Self::new(res)
    }

    pub fn new(mut points: Vec<Point2>) -> Self {
        let w = 1.0 / (points.len() as f32);
        let mut center = Point2::new(0f32, 0f32);
        let mut min_x = 100f32;
        let mut max_x = 100f32;
        let mut min_y = 100f32;
        let mut max_y = 100f32;
        for p in points.iter() {
            center.x += w * p.x;
            center.y += w * p.y;
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.y);
            max_y = max_y.max(p.y);
        }
        let width = max_x - min_x;
        let height = max_y - min_y;
        let mut min_r = 10f32;
        let mut max_r = 0f32;
        for p in points.iter() {
            min_r = min_r.min((p - center).norm());
            max_r = max_r.max((p - center).norm());
        }
        if (points[0].coords - center.coords)
            .perp(&(points[1].coords - center.coords))
            > 0.0
        {
            points.reverse();
        }
        Polygon {
            points: points,
            mass_center: center,
            min_r,
            max_r,
            width: width,
            height: height,
        }
    }

    pub fn centralize(&mut self, rot: Rotation2<f32>) {
        for p in self.points.iter_mut() {
            *p = rot * *p;
            p.x -= self.mass_center.x;
            p.y -= self.mass_center.y;
        }
        self.mass_center = Point2::new(0f32, 0f32);
    }

    pub fn deconstruct(&self, bullet: Point2, sites: usize) -> Vec<Polygon> {
        if self.min_r < 0.8 {
            return vec![];
        }
        let mut transofrmed_points = self.points.clone();
        let w_div = self.width + 0.05;
        let h_div = self.height + 0.05;
        for p in transofrmed_points.iter_mut() {
            p.x += self.width / 2.0;
            p.x /= w_div;
            p.y += self.height / 2.0;
            p.y /= h_div;
        }
        let mut bullet =
            bullet + Vector2::new(self.width / 2.0, self.height / 2.0);
        bullet.x /= w_div;
        bullet.y /= h_div;
        let bullet = Point2::from(5.0 * bullet.coords);
        let (polys, _, _) = destruction(&transofrmed_points, bullet, sites);
        let mut res = vec![];
        for poly in polys.iter() {
            let mut poly = poly.clone();
            for p in poly.iter_mut() {
                p.x *= w_div;
                p.x -= self.width / 2.0;
                p.y *= h_div;
                p.y -= self.height / 2.0;
            }
            res.push(Polygon::new(poly.to_vec()));
        }
        res
        // polys.iter().map(|poly |Polygon::new(poly.to_vec())).collect()
        // let mut res = vec![];
        // if self.points.len() == 3 {
        //     return vec![self.clone()];
        // }
        // // dummy destruct for now
        // let triangulation = self.triangulate();
        // let points = triangulation.points;
        // let indicies = triangulation.indicies;
        // let mut i = 0usize;
        // while i < indicies.len() {
        //     res.push(Polygon::new(vec![
        //         points[indicies[i] as usize],
        //         points[indicies[i + 1] as usize],
        //         points[indicies[i + 2] as usize],
        //     ]));
        //     i += 3;
        // }
        // res
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
    let points = &poly.points;
    let rotation = Rotation2::rotation_between(
        &(points[0].coords + position.coords),
        &Vector2::x_axis(),
    );
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
    }
    BlockSegment { point1, point2 }
}

pub fn shadow_geometry(
    center: Point2,
    geom: Geometry,
    position: Point2,
    rotation: Rotation2<f32>,
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
                Some(BlockSegment {
                    point1: shape_point1,
                    point2: shape_point2,
                })
            } else {
                None
            }
        }
        Geometry::Polygon(mut block_polygon) => {
            let points: Vec<Point2> =
                block_polygon.points.iter().map(|x| rotation * x).collect();
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
            segment.point2 + SHADOW_LENGTH * dir2,
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
