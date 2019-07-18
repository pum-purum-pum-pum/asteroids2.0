use crate::types::{*};
use rand::prelude::*;
use std::fs::File;
use std::io::{BufReader, Error as IOError};
use specs::prelude::*;
use specs_derive::Component;
use crate::components::{*};
use crate::run::FINGER_NUMBER;

fn check_in(x:f32, a: f32, b: f32) -> bool {
    x > a && x < b
}

#[derive(Default)]
pub struct IngameUI {
    // mouse controls
    hover_id: Option<usize>,
    pressed_id: Option<usize>,
    // touch controls
    // for each finger we have id of pressed widget
    widget_finger: [Option<usize>; FINGER_NUMBER],
    pub primitives: Vec<Primitive>,
}


/// Note: ofcourse works only with orthographics projection
pub struct VecController {
    position: Point2, // screen position, center
    radius: f32,
    stick_radius: f32,
    circle_image: Image,
    controller_geometry: Primitive,
}

impl VecController {
    pub fn new(position: Point2, radius: f32, stick_radius: f32, circle_image: Image) -> Self {
        let controller_geometry = Primitive {
            kind: PrimitiveKind::Rectangle(Rectangle{
                position: position - Vector2::new(radius, radius), 
                width: 2.0 * radius, 
                height: 2.0 * radius,
                color: Point3::new(1.0, 1.0, 1.0)
            }),
            with_projection: false,
            image: Some(circle_image)
        };
        VecController {
            position: position,
            radius: radius,
            stick_radius: stick_radius,
            circle_image: circle_image,
            controller_geometry: controller_geometry,
        }
    }

    /// returns radius vector with lenght from 0 to 1 if updated
    pub fn set(&self, id: usize, ingame_ui: &mut IngameUI,  touches: &Touches) -> Option<Vector2> {
        ingame_ui.primitives.push(self.controller_geometry.clone());
        for (touch_id, touch) in touches.iter().enumerate() {
            let previously_attached = 
                ingame_ui.widget_finger[touch_id].is_some() && 
                ingame_ui.widget_finger[touch_id].unwrap() == id;
            let mut interacted = false;
            match touch {
                Some(touch) => {
                    dbg!(ingame_ui.widget_finger[touch_id], id);
                    if self.is_in(touch) || previously_attached {
                        interacted = true;
                        let mut new_pos = Point2::new(touch.x_o, touch.y_o);
                        let mut dir = (new_pos - self.position);
                        if dir.norm() > self.radius {
                            dir = dir.normalize() * self.radius;
                        }
                        new_pos = self.position + dir;
                        // let new_pos = Point2::new(raw.x, raw.y);
                        ingame_ui.widget_finger[touch_id] = Some(id);
                        ingame_ui.primitives.push(
                            self.stick_geometry(new_pos)
                        );
                        return Some(self.get_rad(new_pos))
                    };
                }
                _ => ()
            }
            if !interacted && previously_attached  {
                ingame_ui.widget_finger[touch_id] = None;
            }
        }
        ingame_ui.primitives.push(
            self.stick_geometry(self.position)
        );
        None
    }

    fn get_rad(&self, finger_position: Point2) -> Vector2 {
        (finger_position - self.position) / self.radius
    }

    pub fn stick_geometry(&self, new_pos: Point2) -> Primitive {
        Primitive {
            kind: PrimitiveKind::Rectangle(Rectangle{
                position: new_pos, 
                width: self.stick_radius, 
                height: self.stick_radius,
                color: Point3::new(1.0, 1.0, 1.0)
            }),
            with_projection: false,
            image: None
        }
    }

    pub fn is_in(&self, touch: &Finger) -> bool {
        Vector2::new(
            self.position.x - touch.x_o, 
            self.position.y - touch.y_o
        ).norm() < self.radius
    }
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

#[derive(Clone)]
pub enum PrimitiveKind {
    Rectangle(Rectangle),
    Text(Text)
}

#[derive(Clone)]
pub struct Primitive {
    pub kind: PrimitiveKind,
    pub with_projection: bool,
    pub image: Option<Image>,
}

impl Button {
    pub fn new(
        position: Point2, 
        width: f32, 
        height: f32, 
        color: Point3, 
        with_projection: bool, 
        image: Option<Image>, 
        text: String
    ) -> Button {
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

#[derive(Clone)]
pub struct Text {
    pub position: Point2,
    pub text: String
}

#[derive(Clone)]
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