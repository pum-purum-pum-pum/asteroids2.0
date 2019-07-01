use al::prelude::*;
use astro_lib as al;
use rand::prelude::*;
use std::fs::File;
use std::io::{BufReader, Error as IOError};
use specs::prelude::*;
use specs_derive::Component;
use crate::components::{*};

fn check_in(x:f32, a: f32, b: f32) -> bool {
    x > a && x < b
}

#[derive(Default)]
pub struct IngameUI {
    hover_id: Option<usize>,
    pressed_id: Option<usize>,
    pub primitives: Vec<Primitive>
}

pub struct Button {
    position: Point2, // screen position
    width: f32,
    height: f32,
    color: Point3,
    with_projection: bool,
    pub image: Option<Image>,
    text: String
}

pub enum PrimitiveKind {
    Rectangle(Rectangle),
    Text(Text)
}

pub struct Primitive {
    pub kind: PrimitiveKind,
    pub with_projection: bool,
    pub image: Option<Image>,
}

impl Button {
    pub fn new(position: Point2, width: f32, height: f32, color: Point3, with_projection: bool, image: Option<Image>, text: String) -> Button {
        Button {
            position: position,
            width: width,
            height: height,
            color: color,
            with_projection: with_projection,
            image: image,
            text: text
        }
    }

    pub fn check(&self, mouse: &Mouse) -> bool {
        let mouse_position = Point2::new(mouse.o_x, mouse.o_y);
        mouse.left_released &&
        check_in(mouse_position.x, self.position.x, self.position.x + self.width) &&
        check_in(mouse_position.y, self.position.y, self.position.y + self.height)
    }

    pub fn get_geometry(&self) -> Primitive {
        Primitive {
            kind: PrimitiveKind::Rectangle(Rectangle{
                position: self.position, 
                width: self.width, 
                height: self.height,
                color: self.color
            }),
            with_projection: self.with_projection,
            image: self.image
        }
    }

    pub fn get_text_box(&self) -> Primitive {
        Primitive{
            kind: PrimitiveKind::Text(
                Text{
                    position: self.position,
                    text: self.text.clone(),
                },
            ),
            with_projection: self.with_projection,
            image: None
        }
    }

    pub fn place_and_check(
        &self, 
        ingame_ui: &mut IngameUI,
        mouse: &Mouse, 
    ) -> bool {
        ingame_ui.primitives.push(self.get_geometry());
        ingame_ui.primitives.push(self.get_text_box());
        self.check(mouse)
    }
}

pub struct Text {
    pub position: Point2,
    pub text: String
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