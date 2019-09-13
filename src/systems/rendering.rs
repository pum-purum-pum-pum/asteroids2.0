use gfx_h::{TextData, RenderMode};
use gfx_h::effects::MenuParticles;
use std::collections::{HashMap};
use telemetry::{TeleGraph, render_plot};
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;
pub use crate::gui::{Button, Rectangle, Picture, Selector};

// use flame;

use super::*;
#[cfg(any(target_os = "android"))]
use crate::gui::VecController;
use glyph_brush::{Section, rusttype::Scale};
use crate::geometry::{shadow_geometry};

const BUTTON_SCALE: f32 = 1.2;

use gfx_h::unproject_with_z;
fn visible(
    canvas: &Canvas,
    iso: &Isometry3,
    dims: (i32, i32),

) -> bool{
    let unprojected = unproject_with_z(canvas.observer(), &Point2::new(1.0, 1.0), iso.translation.vector.z, dims.0 as u32, dims.1 as u32, canvas.z_far);
    let corner_rvec = Vector2::new(unprojected.x, unprojected.y);
    let object_rvec = Vector2::new(iso.translation.vector.x, iso.translation.vector.y);
    object_rvec.norm() < corner_rvec.norm()
}


#[derive(Clone, Copy, Debug, TryFromPrimitive)]
#[repr(usize)]
pub enum Widgets {
    BackMenu,
    LazerGun, 
    BlasterGun, 
    ShotGun,
    BasicShip,
    HeavyShip,
    LockedBasicShip,
    LockedHeavyShip,
    ScoreTable,
    Play,
    Upgrade1,
    Upgrade2,
    Upgrade,
    Done,
    WeaponSelector,
    ShipsSelector,
}

pub fn render_primitives<'a>(
    mouse: &Read<'a, Mouse>,
    reader: &mut ReaderId<Primitive>,
    frame: &mut red::Frame,
    image_datas: &ReadStorage<'a, ThreadPin<ImageData>>,
    gl: &ReadExpect<'a, ThreadPin<red::GL>>,
    canvas: &mut WriteExpect<'a, Canvas>,
    viewport: &ReadExpect<'a, red::Viewport>,
    primitives_channel: &mut Write<'a, EventChannel<Primitive>>,
    text_data: &mut WriteExpect<'a, ThreadPin<TextData<'static>>>,
) {
    let dims = viewport.dimensions();
    let (w, h) = (dims.0 as f32, dims.1 as f32);
    let scale = Scale::uniform(((w * w + h * h).sqrt() / 11000.0 * mouse.hdpi as f32).round());
    for primitive in primitives_channel.read(reader) {
        match primitive {
            Primitive {
                kind: PrimitiveKind::Picture(picture),
                with_projection,
            } => {
                let (model, _points, _indicies) = picture.get_gfx();
                canvas
                    .render_primitive_texture(
                        &gl, 
                        &viewport,
                        frame, 
                        image_datas.get(picture.image.0).unwrap(),
                        &model, 
                        *with_projection, 
                        (picture.width, picture.height)
                        );
            },
            Primitive {
                kind: PrimitiveKind::Rectangle(rectangle),
                with_projection,
            } => {
                let (model, points, indicies) = rectangle.get_gfx();
                let geom_data =
                    GeometryData::new(&gl, &points, &indicies).unwrap();
                let fill_color = rectangle.color;
                canvas.render_primitive(
                    &gl,
                    &viewport,
                    frame,
                    &geom_data,
                    &model,
                    (fill_color.x, fill_color.y, fill_color.z),
                    *with_projection,
                    RenderMode::Draw
                );
            }
            Primitive {
                kind: PrimitiveKind::Text(text),
                with_projection: _,
            } => {
                use glyph_brush::{Layout, HorizontalAlign, VerticalAlign};
                text_data.glyph_brush.queue(Section {
                    text: &text.text,
                    scale,
                    screen_position: (text.position.x, text.position.y),
                    // bounds: (w /3.15, h),
                    color: [1.0, 1.0, 1.0, 1.0],
                    layout: Layout::default()
                        .h_align(HorizontalAlign::Center)
                        .v_align(VerticalAlign::Center),
                    ..Section::default()
                });
            }
        }
    }
    canvas.render_text(
        text_data,
        &viewport,
        frame
    );
}


pub struct RenderingSystem {
    reader: ReaderId<Primitive>,
}

impl RenderingSystem {
    pub fn new(reader: ReaderId<Primitive>) -> Self{
        RenderingSystem {
            reader: reader
        }
    }
}

impl<'a> System<'a> for RenderingSystem {
    type SystemData = (
        (
            Entities<'a>,
            ReadStorage<'a, Isometry>,
            ReadStorage<'a, Velocity>,
            ReadStorage<'a, PhysicsComponent>,
            ReadStorage<'a, CharacterMarker>,
            ReadStorage<'a, ShipMarker>,
            ReadStorage<'a, AsteroidMarker>,
            ReadStorage<'a, LightMarker>,
            ReadStorage<'a, StarsMarker>,
            ReadStorage<'a, NebulaMarker>,
            ReadStorage<'a, PlanetMarker>,
            ReadStorage<'a, BigStarMarker>,
            ReadStorage<'a, Projectile>,
            ReadStorage<'a, ThreadPin<ImageData>>,
            ReadStorage<'a, Image>,
            WriteStorage<'a, Animation>,
            ReadStorage<'a, Size>,
            ReadStorage<'a, Polygon>,
            ReadStorage<'a, Geometry>,
            ReadStorage<'a, CollectableMarker>,
            WriteStorage<'a, ThreadPin<ParticlesData>>,
            ReadStorage<'a, MultyLazer>,
            ReadStorage<'a, Chain>,
            ReadStorage<'a, Rift>,
            ReadStorage<'a, ThreadPin<GeometryData>>,
        ),
        WriteExpect<'a, TeleGraph>,
        Read<'a, Mouse>,
        ReadExpect<'a, ThreadPin<red::GL>>,
        ReadExpect<'a, red::Viewport>,
        WriteExpect<'a, Canvas>,
        ReadExpect<'a, PreloadedParticles>,
        Read<'a, World<f32>>,
        Write<'a, EventChannel<Primitive>>,
        Write<'a, UI>,
        WriteExpect<'a, ThreadPin<TextData<'static>>>,
        WriteExpect<'a, GlobalParams>,
        ReadExpect<'a, DevInfo>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            (
                entities,
                isometries,
                velocities,
                physics,
                character_markers,
                ship_markers,
                asteroid_markers,
                light_markers,
                stars,
                nebulas,
                planets,
                _big_star_markers,
                projectiles,
                image_datas,
                image_ids,
                mut animations,
                sizes,
                polygons,
                geometries,
                collectables,
                mut particles_datas,
                multy_lazers,
                _chains,
                rifts,
                geom_datas
            ),
            mut telegraph,
            mouse,
            gl,
            viewport,
            mut canvas,
            preloaded_particles,
            _world,
            mut primitives_channel,
            mut ui,
            mut text_data,
            mut global_params,
            dev_info
        ) = data;
        let dims = viewport.dimensions();
        let char_pos = if let Some((iso, vel, _)) = (&isometries, &velocities, &character_markers).join().next() {
            canvas.update_observer(
                Point2::new(iso.0.translation.vector.x, iso.0.translation.vector.y),
                vel.0.norm() / VELOCITY_MAX,
                Vector2::new(mouse.x01, mouse.y01).normalize()
            );
            iso.0.translation.vector
        } else {
            return
        };
        flame::start("rendering");
        flame::start("clear");
        let mut frame = red::Frame::new(&gl);
        global_params.update();
        frame.set_clear_color(global_params.red.min(1.0), 0.004, 0.0, 1.0);
        // frame.set_clear_color(0.15, 0.004, 0.0, 1.0);
        frame.set_clear_stencil(0);
        // frame.clear_color_and_stencil();
        flame::start("color");
        frame.clear_color();
        flame::end("color");
        flame::start("stencil");
        frame.clear_stencil();
        flame::end("stencil");
        telegraph.update();
        flame::end("clear");
        flame::start("shadow rendering");
        for (_entity, iso, geom, _) in (&entities, &isometries, &geometries, &asteroid_markers).join() {
            if visible(&*canvas, &iso.0, dims) {
                let pos = Point2::new(iso.0.translation.vector.x, iso.0.translation.vector.y);
                // light_poly.clip_one((*geom).clone(), pos);
                let rotation = iso.0.rotation.euler_angles().2;
                let rotation = Rotation2::new(rotation);
                let shadow_triangulation = shadow_geometry(
                    Point2::new(char_pos.x, char_pos.y),
                    (*geom).clone(),
                    pos,
                    rotation
                );
                if let Some(shadow_triangulation) = shadow_triangulation {
                    let geometry_data =  GeometryData::new(
                        &gl, 
                        &shadow_triangulation.points, 
                        &shadow_triangulation.indicies)
                    .unwrap();
                    let iso = Isometry3::new(iso.0.translation.vector, Vector3::new(0f32, 0f32, 0f32));
                    // draw shadows
                    canvas
                        .render_geometry(
                            &gl, &viewport,
                            &mut frame,
                            &geometry_data,
                            &iso,
                            RenderMode::StencilWrite,
                            Point3::new(0f32, 0f32, 0f32)
                        );
                }
            }
        }
        flame::end("shadow rendering");

        flame::start("background rendering");
        for (_entity, iso, image, size, _stars) in
            (&entities, &isometries, &image_ids, &sizes, &stars).join() {
            if visible(&*canvas, &iso.0, dims) {
                let image_data = image_datas.get(image.0).unwrap();
                canvas
                    .render(
                            &gl,
                            &viewport,
                            &mut frame,
                            &image_data,
                            &iso.0,
                            size.0,
                            false,
                            None
                    );
            }
        };

        for (_entity, iso, image, size, _stars) in
            (&entities, &isometries, &image_ids, &sizes, &_big_star_markers).join() {
            let image_data = image_datas.get(image.0).unwrap();
            canvas
                .render(
                        &gl,
                        &viewport,
                        &mut frame,
                        &image_data,
                        &iso.0,
                        size.0,
                        false,
                        None
                );
        };
        for (_entity, iso, image, size, _nebula) in
            (&entities, &isometries, &image_ids, &sizes, &nebulas).join() {
            if visible(&*canvas, &iso.0, dims) {
                let image_data = image_datas.get(image.0).unwrap();
                canvas
                    .render(
                            &gl,
                            &viewport,
                            &mut frame,
                            &image_data,
                            &iso.0,
                            size.0,
                            false,
                            None
                    );
            }
        };
        for (_entity, iso, image, size, _planet) in
            (&entities, &isometries, &image_ids, &sizes, &planets).join() {
            if visible(&*canvas, &iso.0, dims) {
                let image_data = image_datas.get(image.0).unwrap();
                canvas
                    .render(
                            &gl,
                            &viewport,
                            &mut frame,
                            &image_data,
                            &iso.0,
                            size.0,
                            false,
                            None
                    );
            }
        };
        flame::end("background rendering");
        // flame::start("particles rendering");
        // for (entity, particles_data) in (&entities, &mut particles_datas).join() {
        //     match **particles_data {
        //         ParticlesData::Explosion(ref mut particles) => {
        //             if particles.update() {
        //                 canvas
        //                     .render_instancing(
        //                         &gl,
        //                         &viewport,
        //                         &mut frame,
        //                         &particles.instancing_data,
        //                         &Isometry3::new(
        //                             Vector3::new(0f32, 0f32, 0f32),
        //                             Vector3::new(0f32, 0f32, 0f32),
        //                         )
        //                     );
        //             } else {
        //                 entities.delete(entity).unwrap();
        //             }
        //     }
        //         _ => ()
        //     };
        // }

        // for (iso, vel, _char_marker) in (&isometries, &velocities, &character_markers).join() {
        //     let translation_vec = iso.0.translation.vector;
        //     let mut isometry = Isometry3::new(translation_vec, Vector3::new(0f32, 0f32, 0f32));
        //     let pure_isometry = isometry.clone();
        //     isometry.translation.vector.z = canvas.get_z_shift();
        //     match **particles_datas
        //         .get_mut(preloaded_particles.movement)
        //         .unwrap()
        //     {
        //         ParticlesData::MovementParticles(ref mut particles) => {
        //             particles.update(1.0 * Vector2::new(-vel.0.x, -vel.0.y));
        //              canvas
        //                 .render_instancing(
        //                     &gl,
        //                     &viewport,
        //                     &mut frame,
        //                     &particles.instancing_data,
        //                     &pure_isometry,
        //                 );
        //         }
        //         _ => panic!(),
        //     };
        // }

        // flame::end("particles rendering");

        flame::start("other");
        for (_entity, iso, image, size, _light) in
            (&entities, &isometries, &image_ids, &sizes, &light_markers).join()
        {
            let translation_vec = iso.0.translation.vector;
            let isometry = Isometry3::new(translation_vec, Vector3::new(0f32, 0f32, 0f32));
            canvas
                .render(
                    &gl,
                    &viewport,
                    &mut frame,
                    &image_datas.get(image.0).unwrap(),
                    &isometry,
                    size.0,
                    true,
                    Some(red::Blend)
                );
        }


        let mut render_lazer = |
            iso: &Isometry,
            lazer: &Lazer,
            force_rendering: bool,
            rotation
        | {
            if lazer.active || force_rendering {
                let h = lazer.current_distance;
                let w = 0.05f32;
                let positions = vec![
                    Vector2::new(-w / 2.0, 0f32),
                    Vector2::new(w / 2.0, 0f32),
                    Vector2::new(0.0, -h) // hmmmmm, don't know why minus
                ];
                let positions: Vec<Point2> = positions
                    .into_iter()
                    .map(|v: Vector2| Point2::from(rotation * v))
                    .collect();
                let indices = [0u16, 1, 2];
                let geometry_data = GeometryData::new(
                    &gl, &positions, &indices
                ).unwrap();
                canvas.render_geometry(
                    &gl,
                    &viewport,
                    &mut frame,
                    &geometry_data,
                    &iso.0,
                    RenderMode::Draw,
                    Point3::new(1.0, 0.0, 0.0)
                );
            }
        };
        let zero_rotation = Rotation2::new(0.0);
        for (rift, isometry) in (&rifts, &isometries).join() {
            for (lazer, dir) in rift.lazers.iter() {
                let pos = isometry.0.translation.vector;
                let up = Vector2::new(0.0, -1.0);
                let dir = Vector2::new(dir.0, dir.1);
                let rotation = Rotation2::rotation_between(&up, &Vector2::new(dir.x, dir.y));
                let isometry = Isometry3::new(
                    Vector3::new(pos.x, pos.y, pos.z), Vector3::new(0f32, 0f32, rotation.angle())
                );
                render_lazer(&Isometry(isometry), &lazer, false, zero_rotation);
            }
        }
        for (iso, multy_lazer) in (&isometries, &multy_lazers).join() {
            for (angle, lazer) in multy_lazer.iter() {
                // let rotation = Rotation2::new(i as f32 * std::f32::consts::PI / 2.0);
                let rotation = Rotation2::new(angle);
                render_lazer(iso, lazer, false, rotation);
            }
        };
        for (_entity, iso, image, size, _projectile) in
            (&entities, &isometries, &image_ids, &sizes, &projectiles).join()
        {
            canvas
                .render(
                    &gl,
                    &viewport,
                    &mut frame,
                    &image_datas.get(image.0).unwrap(),
                    &iso.0,
                    size.0,
                    true,
                    Some(red::Blend)
                );
        }
        for (_entity, iso, _physics_component, image, size, _ship) in
                    (&entities, &isometries, &physics, &image_ids, &sizes, &ship_markers).join() {
                // let iso2 = world
                //     .rigid_body(physics_component.body_handle)
                //     .unwrap()
                //     .position();
                // let iso = iso2_iso3(iso2);
                if visible(&*canvas, &iso.0, dims) {
                    let image_data = &image_datas.get(image.0).unwrap();
                    canvas
                        .render(
                            &gl,
                            &viewport,
                            &mut frame,
                            &image_data,
                            &iso.0,
                            size.0,
                            true,
                            None
                        )
                }
        }
        flame::end("other");
        flame::start("asteroids rendering");
        for (_entity, iso, _image, _size, geom_data, _asteroid) in (
            &entities,
            &isometries,
            &image_ids,
            &sizes,
            &geom_datas,
            &asteroid_markers,
        )
            .join()
        {
            if visible(&*canvas, &iso.0, dims) {
                canvas
                    .render_geometry(
                        &gl, &viewport, 
                        &mut frame, 
                        &geom_data, 
                        &iso.0,
                        RenderMode::Draw,
                        Point3::new(0.8, 0.8, 0.8)
                    )
            }
        }
        flame::end("asteroids rendering");
        flame::start("collectables");
        for (iso, size, image, _collectable) in (&isometries, &sizes, &image_ids, &collectables).join() {
            let image_data = image_datas.get(image.0).unwrap();
            if visible(&*canvas, &iso.0, dims) {
                canvas
                    .render(
                        &gl,
                        &viewport,
                            &mut frame,
                            &image_data,
                            &iso.0,
                            size.0,
                            true,
                            None
                    )
            }
        }
        let _render_line = |
            a: Point2,
            b: Point2
        | {
            let line_width = 0.05;
            let line_length = (b.coords - a.coords).norm();
            let positions = vec![
                Point2::new(-line_width / 2.0, 0f32),
                Point2::new(line_width / 2.0, 0f32),
                Point2::new(-line_width / 2.0, -line_length),
                Point2::new(line_width / 2.0, -line_length)
            ];
            let up = Vector2::new(0.0, -line_length);
            let rotation = Rotation2::rotation_between(&up, &(&b.coords - a.coords));
            let iso = Isometry3::new(
                Vector3::new(a.x, a.y, 0f32), 
                Vector3::new(0f32, 0f32, rotation.angle())
            );
            let indices = [0u16, 1, 2, 0, 2, 3];
            let geometry_data = GeometryData::new(
                &gl, &positions, &indices
            ).unwrap();
            canvas.render_geometry(
                &gl,
                &viewport,
                &mut frame,
                &geometry_data,
                &iso,
                RenderMode::Draw,
                Point3::new(1f32, 1f32, 1f32)
            );
        };
        flame::end("collectables");
        // debug grid drawing
        // for i in 0..nebula_grid.grid.size {
        //     for j in 0..nebula_grid.grid.size {
        //         let ((min_w, max_w), (min_h, max_h)) = nebula_grid.grid.get_rectangle(i, j);
        //         render_line(Point2::new(min_w, min_h), Point2::new(min_w, max_h));
        //         render_line(Point2::new(min_w, max_h), Point2::new(max_w, max_h));
        //         render_line(Point2::new(max_w, max_h), Point2::new(max_w, min_h));                
        //         render_line(Point2::new(max_w, min_h), Point2::new(min_w, min_h));
        //     }
        // }
        flame::start("animation");
        for (iso, size, animation) in (&isometries, &sizes, &mut animations).join() {
            if visible(&*canvas, &iso.0, dims) {
                let animation_frame = animation.next_frame();
                if let Some(animation_frame) = animation_frame {
                    let image_data = image_datas.get(animation_frame.image.0).unwrap();
                    canvas
                        .render(
                            &gl,
                            &viewport,
                            &mut frame,
                            &image_data,
                            &iso.0,
                            size.0,
                            false,
                            Some(red::Blend)
                        )
                };
            }
        };
        flame::end("animation");
        flame::start("primitives rendering");
        primitives_channel.iter_write(ui.primitives.drain(..));
        render_primitives(
            &mouse,
            &mut self.reader,
            &mut frame,
            &image_datas,
            &gl,
            &mut canvas,
            &viewport,
            &mut primitives_channel,
            &mut text_data,
        );
        flame::end("primitives rendering");
        // for (name, span) in time_spans.iter() {
        //     telegraph.insert(name.to_string(), span.evaluate().as_millis() as f32 / 1000.0 * 60.0); // TODO "xFPS" actually
        // }
        flame::end("rendering");
        let spans = flame::spans();
        telegraph.insert("fps".to_string(), dev_info.fps as f32/ 60.0);
        if dev_info.draw_telemetry {
            for span in spans.iter() {
                if [
                    "rendering".to_string(), 
                    "dispatch".to_string(), 
                    "insert".to_string(),
                    "asteroids".to_string()
                ].contains(&span.name.to_string()) {
                    telegraph.insert(span.name.to_string(), span.delta as f32 / 1E9 * 60.0);
                }
                if span.name == "dispatch" {
                    for subspan in span.children.iter() {
                        telegraph.insert(subspan.name.to_string(), subspan.delta as f32 / 1E9 * 60.0);
                    }
                }
            }
            for name in telegraph.iter_names() {
                if let Some(plot) = telegraph.iter(name.to_string()) {
                    render_plot(
                        plot.0,
                        plot.1,
                        14.0, 
                        10.0,
                        &gl,
                        &viewport,
                        &canvas,
                        &mut frame,
                    );
                }
            }
        }
    }
}