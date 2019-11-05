use cfg_if::cfg_if;
use common::*;
use components::*;
use std::collections::HashMap;

cfg_if! {
    if #[cfg(any(target_os = "android"))] {
        use components::FINGER_NUMBER;
        /// Note: ofcourse works only with orthographics projection
        pub struct VecController {
            position: Point2, // screen position, center
            radius: f32,
            stick_radius: f32,
            _circle_image: AtlasImage,
            controller_geometry: Primitive,
        }

        impl VecController {
            pub fn new(position: Point2, radius: f32, stick_radius: f32, circle_image: AtlasImage) -> Self {
                let controller_geometry = Primitive {
                    kind: PrimitiveKind::Picture(Picture{
                        position: position - Vector2::new(radius, radius),
                        width: 2.0 * radius,
                        height: 2.0 * radius,
                        image: circle_image
                    }),
                    with_projection: false,
                };
                VecController {
                    position: position,
                    radius: radius,
                    stick_radius: stick_radius,
                    _circle_image: circle_image,
                    controller_geometry: controller_geometry,
                }
            }

            /// returns radius vector with lenght from 0 to 1 if updated
            pub fn set(&self, id: usize, ui: &mut UI,  touches: &Touches) -> Option<Vector2> {
                ui.primitives.push(self.controller_geometry.clone());
                // touches assumed to be FINGER_NUMBER sized array so we iterate over all possible touches
                for (touch_id, touch) in touches.iter().enumerate() {
                    let previously_attached =
                        ui.widget_finger[touch_id].is_some() &&
                        ui.widget_finger[touch_id].unwrap() == id;
                    if let Some(touch) = touch {
                        if self.is_in(touch) || previously_attached {
                            let mut new_pos = Point2::new(touch.x_o, touch.y_o);
                            let mut dir = new_pos - self.position;
                            if dir.norm() > self.radius {
                                dir = dir.normalize() * self.radius;
                            }
                            new_pos = self.position + dir;
                            // let new_pos = Point2::new(raw.x, raw.y);
                            ui.widget_finger[touch_id] = Some(id);
                            ui.primitives.push(
                                self.stick_geometry(new_pos)
                            );
                            return Some(self.get_rad(new_pos))
                        } else {
                            if previously_attached {
                                ui.widget_finger[touch_id] = None;
                            }
                        }
                    }
                    else {
                        if previously_attached {
                            ui.widget_finger[touch_id] = None;
                        }
                    }
                }
                ui.primitives.push(
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
                }
            }

            pub fn is_in(&self, touch: &Finger) -> bool {
                Vector2::new(
                    self.position.x - touch.x_o,
                    self.position.y - touch.y_o
                ).norm() < self.radius
            }
        }

    }
}

fn check_in(x: f32, a: f32, b: f32) -> bool {
    x > a && x < b
}

#[derive(Default)]
pub struct UI {
    // mouse controls
    hover: Option<usize>,
    selectors: HashMap<usize, Option<usize>>,
    // touch controls
    // for each finger we have id of pressed widget
    #[cfg(any(target_os = "android"))]
    widget_finger: [Option<usize>; FINGER_NUMBER],
    pub primitives: Vec<Primitive>,
    pub sounds: Vec<Sound>,
}

pub struct Selector {
    pub buttons: Vec<Button>,
    pub id: usize,
    pub mask: Option<Vec<bool>>,
}

impl Selector {
    pub fn place_and_check(&self, ui: &mut UI, mouse: &Mouse) -> Option<usize> {
        for (i, button) in self.buttons.iter().enumerate() {
            if button.place_and_check(ui, mouse) {
                let mask = if let Some(mask) = &self.mask {
                    mask[i]
                } else {
                    true
                };
                if mask {
                    ui.selectors.insert(self.id, Some(button.id));
                }
                return Some(button.id);
            }
        }
        None
    }
}

pub struct Button {
    position: Point2, // screen position
    width: f32,
    height: f32,
    color: Option<Point3>,
    with_projection: bool,
    pub image: Option<AtlasImage>,
    text: String,
    id: usize,
    hover_sound: Option<Sound>,
    click_sound: Option<Sound>,
}

#[derive(Clone)]
pub enum PrimitiveKind {
    Rectangle(Rectangle),
    Text(Text),
    Picture(Picture),
}

#[derive(Clone)]
pub struct Picture {
    pub position: Point2,
    pub width: f32,
    pub height: f32,
    pub image: AtlasImage,
}

impl Picture {
    pub fn get_gfx(&self) -> (Isometry3, Vec<Point2>, Vec<u16>) {
        let model = Isometry3::new(
            Vector3::new(self.position.x, self.position.y, 0f32),
            Vector3::new(0f32, 0f32, 0f32),
        );
        (
            model,
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(0.0, self.height),
                Point2::new(self.width, self.height),
                Point2::new(self.width, 0.0),
            ],
            vec![0u16, 1, 2, 2, 3, 0],
        )
    }
}

#[derive(Clone)]
pub struct Primitive {
    pub kind: PrimitiveKind,
    pub with_projection: bool,
}

impl Button {
    pub fn new(
        position: Point2,
        width: f32,
        height: f32,
        color: Option<Point3>,
        with_projection: bool,
        image: Option<AtlasImage>,
        text: String,
        id: usize,
        hover_sound: Option<Sound>,
        click_sound: Option<Sound>,
    ) -> Button {
        Button {
            position: position,
            width: width,
            height: height,
            color: color,
            with_projection: with_projection,
            image: image,
            text: text,
            id: id,
            hover_sound: hover_sound,
            click_sound: click_sound,
        }
    }

    pub fn check(&self, mouse: &Mouse, ui: &mut UI) -> bool {
        let mouse_position = Point2::new(mouse.o_x, mouse.o_y);
        let hover = check_in(
            mouse_position.x,
            self.position.x,
            self.position.x + self.width,
        ) && check_in(
            mouse_position.y,
            self.position.y,
            self.position.y + self.height,
        );
        if hover {
            if let Some(id) = ui.hover {
                if id != self.id {
                    if let Some(hover_sound) = self.hover_sound {
                        ui.sounds.push(hover_sound)
                    }
                }
            }
            ui.hover = Some(self.id);
        }
        mouse.left_released && hover
    }

    pub fn get_geometry(&self, ui: &UI) -> Vec<Primitive> {
        let mut res = vec![];
        let mut add_w = 0f32;
        let mut add_h = 0f32;
        let selected_ids: Vec<usize> =
            ui.selectors.iter().filter_map(|(_, i)| *i).collect();
        if selected_ids.contains(&self.id) {
            add_w += self.width * 0.2;
            add_h += self.height * 0.2;
        } else {
            if let Some(hover) = ui.hover {
                if hover == self.id {
                    add_w += self.width * 0.1;
                    add_h += self.height * 0.1;
                }
            }
        }
        if let Some(color) = self.color {
            let rectangle = Primitive {
                kind: PrimitiveKind::Rectangle(Rectangle {
                    position: self.position
                        - Vector2::new(add_w / 2.0, add_h / 2.0),
                    width: self.width,
                    height: self.height,
                    color: color,
                }),
                with_projection: self.with_projection,
            };
            res.push(rectangle);
        }
        if let Some(image) = self.image {
            let picture = Primitive {
                kind: PrimitiveKind::Picture(Picture {
                    position: self.position
                        - Vector2::new(add_w / 2.0, add_h / 2.0),
                    width: self.width + add_w,
                    height: self.height + add_h,
                    image: image,
                }),
                with_projection: self.with_projection,
            };
            res.push(picture);
        }
        res
    }

    pub fn get_text_box(&self) -> Primitive {
        // TODO FIX
        let crazy_position = Point2::new(
            self.position.x + self.width / 2.0,
            self.position.y + self.height / 2.0,
        );
        Primitive {
            kind: PrimitiveKind::Text(Text {
                position: crazy_position,
                text: self.text.clone(),
                color: (1.0, 1.0, 1.0, 1.0),
                font_size: 1.0
            }),
            with_projection: self.with_projection,
        }
    }

    pub fn place_and_check(&self, ui: &mut UI, mouse: &Mouse) -> bool {
        let check = self.check(mouse, ui);
        ui.primitives.extend(self.get_geometry(ui).into_iter());
        ui.primitives.push(self.get_text_box());
        if check {
            if let Some(click) = self.click_sound {
                ui.sounds.push(click);
            }
        }
        check
    }
}

#[derive(Clone)]
pub struct Text {
    pub position: Point2,
    pub color: (f32, f32, f32, f32),
    pub text: String,
    pub font_size: f32
}

#[derive(Clone)]
pub struct Rectangle {
    pub position: Point2, // screen position
    pub width: f32,
    pub height: f32,
    pub color: Point3,
}

impl Rectangle {
    pub fn get_gfx(&self) -> (Isometry3, Vec<Point2>, Vec<u16>) {
        let model = Isometry3::new(
            Vector3::new(self.position.x, self.position.y, 0f32),
            Vector3::new(0f32, 0f32, 0f32),
        );
        (
            model,
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(0.0, self.height),
                Point2::new(self.width, self.height),
                Point2::new(self.width, 0.0),
            ],
            vec![0u16, 1, 2, 2, 3, 0],
        )
    }
}
