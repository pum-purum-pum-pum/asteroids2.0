use al::prelude::*;
use astro_lib as al;
use rand::prelude::*;
use std::fs::File;
use std::io::{BufReader, Error as IOError};
use specs::prelude::*;
use specs_derive::Component;

fn check_in(x:f32, a: f32, b: f32) -> bool {
    x > a && x < b
}

#[derive(Default)]
pub struct IngameUI {
    pub primitives: Vec<Primitive>
}

pub struct Button {
    position: Point2, // screen position
    width: f32,
    height: f32,
    color: Point3,
    with_projection: bool
}

pub enum PrimitiveKind {
    Rectangle(Rectangle),
    Text(String)
}

pub struct Primitive {
    pub kind: PrimitiveKind,
    pub with_projection: bool,
}

impl Button {
    pub fn new(position: Point2, width: f32, height: f32, color: Point3, with_projection: bool) -> Button {
        Button {
            position: position,
            width: width,
            height: height,
            color: color,
            with_projection: with_projection
        }
    }

    pub fn check(&self, mouse: Point2) -> bool {
        check_in(mouse.x, self.position.x, self.position.x + self.width) &&
        check_in(mouse.y, self.position.y, self.position.y + self.height)
    }

    // pub fn get_geometry(&self) -> (Vec<Point2>, Vec<u16>) {
    //     (
    //         vec![
    //             Point2::new(0f32, 0f32), 
    //             Point2::new(0f32, self.height),
    //             Point2::new(self.width, self.height), 
    //             Point2::new(self.width, 0f32)
    //         ],
    //         vec![0u16, 1, 2, 2, 3, 0]
    //     )
    // }
    pub fn get_geometry(&self) -> Primitive {
        Primitive {
            kind: PrimitiveKind::Rectangle(Rectangle{
                position: self.position, 
                width: self.width, 
                height: self.height,
                color: self.color
            }),
            with_projection: self.with_projection
        }
    }

    pub fn place_and_check(
        &self, 
        ingame_ui: &mut IngameUI,
        mouse_screen_position: Point2, 
    ) -> bool {
        ingame_ui.primitives.push(self.get_geometry());
        self.check(mouse_screen_position)
    }
}

pub struct Rectangle {
    pub position: Point2, // screen position
    pub width: f32,
    pub height: f32,
    pub color: Point3,
}

impl Rectangle {
    pub fn get_geometry(&self) -> (Isometry3, Vec<Point2>, Vec<u16>) {
        let model = Isometry3::new(Vector3::new(self.position.x, self.position.y, 0f32), Vector3::new(0f32, 0f32 ,0f32));
        (
            model,
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(0.0, self.height),
                Point2::new(self.width, self.height), 
                Point2::new(self.width, 0.0)
            ],
            vec![0u16, 1, 2, 2, 3, 0]
        )
    }
}