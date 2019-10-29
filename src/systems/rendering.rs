pub use crate::gui::{Button, Picture, Rectangle, Selector};
use gfx_h::{
    unproject_with_z, RenderMode, SpriteBatch, TextData, WorldTextData,
};
use num_enum::TryFromPrimitive;
use telemetry::{render_plot, TeleGraph};

// use flame;

use super::*;
#[cfg(any(target_os = "android"))]
use crate::gui::VecController;
use geometry::{shadow_geometry, Triangulation};
use glyph_brush::{rusttype::Scale, Section};
use physics_system::MENU_VELOCITY;

fn visible(canvas: &Canvas, iso: &Isometry3, dims: (i32, i32)) -> bool {
    let unprojected = unproject_with_z(
        canvas.observer(),
        &Point2::new(1.0, 1.0),
        iso.translation.vector.z,
        dims.0 as u32,
        dims.1 as u32,
        canvas.z_far,
    );
    let corner_rvec = Vector2::new(unprojected.x, unprojected.y);
    let object_rvec =
        Vector2::new(iso.translation.vector.x, iso.translation.vector.y);
    object_rvec.norm() < corner_rvec.norm()
}

#[derive(Clone, Copy, Debug, TryFromPrimitive)]
#[repr(usize)]
pub enum Widgets {
    BackMenu,
    BlasterGun,
    LazerGun,
    ShotGun,
    LockedBlasterGun,
    LockedLazerGun,
    BasicShip,
    HeavyShip,
    SuperShip,
    LockedBasicShip,
    LockedHeavyShip,
    LockedSuperShip,
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
    gl: &ReadExpect<'a, ThreadPin<red::GL>>,
    canvas: &mut WriteExpect<'a, Canvas>,
    viewport: &ReadExpect<'a, red::Viewport>,
    primitives_channel: &mut Write<'a, EventChannel<Primitive>>,
    text_data: &mut WriteExpect<'a, ThreadPin<TextData<'static>>>,
    world_text_data: &mut WriteExpect<'a, ThreadPin<WorldTextData<'static>>>,
) {
    let dims = viewport.dimensions();
    let (w, h) = (dims.0 as f32, dims.1 as f32);
    let world_text_scale = ((w * w + h * h).sqrt() / 10000.0 * mouse.hdpi as f32).round();
    let scale = ((w * w + h * h).sqrt() / 11000.0 * mouse.hdpi as f32).round();
    for primitive in primitives_channel.read(reader) {
        match primitive {
            Primitive {
                kind: PrimitiveKind::Picture(picture),
                with_projection,
            } => {
                let (model, _points, _indicies) = picture.get_gfx();
                canvas.render_primitive_texture(
                    &gl,
                    &viewport,
                    frame,
                    &picture.image,
                    &model,
                    *with_projection,
                    (picture.width, picture.height),
                );
            }
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
                    RenderMode::Draw,
                );
            }
            Primitive {
                kind: PrimitiveKind::Text(text),
                with_projection,
            } => {
                use glyph_brush::{HorizontalAlign, Layout, VerticalAlign};
                if *with_projection {
                    // INSANE hack to deal with projections and glyph brush!
                    // calculating text screen coords via projections on CPU
                    // YEEEY!
                    use gfx_h::{get_view, ortho_unproject_real, perspective};
                    let persp = perspective(dims.0 as u32, dims.1 as u32)
                        .to_homogeneous();
                    let view = get_view(canvas.observer()).to_homogeneous();
                    let t_pos = persp
                        * view
                        * Point4::new(
                            text.position.x,
                            text.position.y,
                            0.0,
                            1.0,
                        );
                    let point = ortho_unproject_real(
                        dims.0 as u32,
                        dims.1 as u32,
                        t_pos,
                    );
                    let point = (point.x, point.y);
                    world_text_data.glyph_brush.queue(Section {
                        text: &text.text,
                        scale: Scale::uniform(world_text_scale * text.font_size),
                        screen_position: point,
                        // bounds: (w /20.0, h / 20.0),
                        color: [text.color.0, text.color.1, text.color.2, text.color.3],
                        layout: Layout::default()
                            .h_align(HorizontalAlign::Center)
                            .v_align(VerticalAlign::Center),
                        ..Section::default()
                    });
                } else {
                    // orthographic projection
                    text_data.glyph_brush.queue(Section {
                        text: &text.text,
                        scale: Scale::uniform(scale * text.font_size),
                        screen_position: (text.position.x, text.position.y),
                        // bounds: (w /3.15, h),
                        color: [text.color.0, text.color.1, text.color.2, text.color.3],
                        layout: Layout::default()
                            .h_align(HorizontalAlign::Center)
                            .v_align(VerticalAlign::Center),
                        ..Section::default()
                    });
                }
            }
        }
    }
    canvas.render_text(text_data, &viewport, frame, false);
    canvas.render_world_text(world_text_data, &viewport, frame, false);
}

pub struct RenderingSystem {
    reader: ReaderId<Primitive>,
}

impl RenderingSystem {
    pub fn new(reader: ReaderId<Primitive>) -> Self {
        RenderingSystem { reader: reader }
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
            ReadStorage<'a, FogMarker>,
            ReadStorage<'a, Projectile>,
            ReadStorage<'a, ThreadPin<ImageData>>,
            ReadStorage<'a, AtlasImage>,
            WriteStorage<'a, Animation>,
            ReadStorage<'a, Size>,
            ReadStorage<'a, Geometry>,
            ReadStorage<'a, CollectableMarker>,
            WriteStorage<'a, ThreadPin<ParticlesData>>,
            ReadStorage<'a, MultyLazer>,
            ReadStorage<'a, Chain>,
            ReadStorage<'a, Rift>,
            ReadStorage<'a, ThreadPin<GeometryData>>,
            ReadStorage<'a, DamageFlash>,
            WriteStorage<'a, TextComponent>,
        ),
        WriteStorage<'a, Position2D>,
        ReadStorage<'a, Lifetime>,
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
        WriteExpect<'a, ThreadPin<WorldTextData<'static>>>,
        WriteExpect<'a, GlobalParams>,
        ReadExpect<'a, DevInfo>,
        Write<'a, EventChannel<Sound>>,
        ReadExpect<'a, PreloadedImages>,
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
                _fog_markers,
                projectiles,
                image_datas,
                atlas_images,
                mut animations,
                sizes,
                geometries,
                collectables,
                mut particles_datas,
                multy_lazers,
                _chains,
                rifts,
                geom_datas,
                damage_flash,
                mut text_components,
            ),
            positions2d,
            lifetimes,
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
            mut world_text_data,
            mut global_params,
            dev_info,
            mut sounds_channel,
            preloaded_images,
        ) = data;
        let dims = viewport.dimensions();
        flame::start("rendering");
        flame::start("clear");
        let mut frame = red::Frame::new(&gl);
        global_params.update();
        frame.set_clear_color(global_params.red.min(1.0), 0.004, 0.0, 1.0);
        // frame.set_clear_color(0.15, 0.004, 0.0, 1.0);
        frame.set_clear_stencil(0);
        frame.clear_color_and_stencil();
        flame::start("color");
        frame.clear_color();
        flame::end("color");
        flame::start("stencil");
        frame.clear_stencil();
        flame::end("stencil");
        telegraph.update();
        flame::end("clear");
        if let Some((iso, vel, _)) =
            (&isometries, &velocities, &character_markers).join().next()
        {
            canvas.update_observer(
                Point2::new(
                    iso.0.translation.vector.x,
                    iso.0.translation.vector.y,
                ),
                vel.0.norm() / VELOCITY_MAX,
                Vector2::new(mouse.x01, mouse.y01).normalize(),
            );
            let char_pos = iso.0.translation.vector;
            flame::start("shadow rendering");
            let mut final_triangulation = Triangulation::new();
            for (_entity, iso, geom, _) in
                (&entities, &isometries, &geometries, &asteroid_markers).join()
            {
                if visible(&*canvas, &iso.0, dims) {
                    let pos = Point2::new(
                        iso.0.translation.vector.x,
                        iso.0.translation.vector.y,
                    );
                    // light_poly.clip_one((*geom).clone(), pos);
                    let rotation = iso.0.rotation.euler_angles().2;
                    let rotation = Rotation2::new(rotation);
                    // sooht angles of polygon
                    let geom = if let Geometry::Polygon(polygon) = geom {
                        Geometry::Polygon(polygon.clone().into_rounded(2))
                    } else {
                        geom.clone()
                    };
                    let shadow_triangulation = shadow_geometry(
                        Point2::new(char_pos.x, char_pos.y),
                        geom,
                        pos,
                        rotation,
                    );
                    if let Some(mut shadow_triangulation) = shadow_triangulation
                    {
                        shadow_triangulation.translate(pos.coords);
                        final_triangulation.extend(shadow_triangulation);
                    }
                }
            }
            let geometry_data = GeometryData::new(
                &gl,
                &final_triangulation.points,
                &final_triangulation.indicies,
            )
            .unwrap();
            let iso = Isometry3::new(
                Vector3::new(0f32, 0f32, 0f32),
                Vector3::new(0f32, 0f32, 0f32),
            );
            // draw shadows
            canvas.render_geometry(
                &gl,
                &viewport,
                &mut frame,
                &geometry_data,
                &iso,
                RenderMode::StencilWrite,
                Point3::new(0f32, 0f32, 0f32),
            );

            flame::end("shadow rendering");
        };
        flame::start("sprite batch rendering");
        // struct to store batch data(needed to do z sorting)
        pub struct MiniBatch {
            pub images: Vec<AtlasImage>,
            pub isometries: Vec<Isometry3>,
            pub sizes: Vec<f32>,
        }

        impl MiniBatch {
            pub fn new() -> Self {
                // TODO: with capacity
                Self {
                    images: vec![],
                    isometries: vec![],
                    sizes: vec![],
                }
            }

            pub fn append(
                &mut self,
                image: AtlasImage,
                isometry: Isometry3,
                size: f32,
            ) {
                self.images.push(image);
                self.isometries.push(isometry);
                self.sizes.push(size);
            }

            pub fn extend(&mut self, other: MiniBatch) {
                self.images.extend(other.images.iter());
                self.isometries.extend(other.isometries.iter());
                self.sizes.extend(other.sizes.iter());
            }
        }
        let mut all_batch = MiniBatch::new();
        let mut stars_batch = MiniBatch::new();
        let mut nebulas_batch = MiniBatch::new();
        let mut planets_batch = MiniBatch::new();
        let mut fog_batch = MiniBatch::new();
        for (entity, iso, atlas_image, size) in
            (&entities, &isometries, &atlas_images, &sizes).join()
        {
            if stars.get(entity).is_some() {
                stars_batch.append(*atlas_image, iso.0, size.0);
            }
            if nebulas.get(entity).is_some() {
                nebulas_batch.append(*atlas_image, iso.0, size.0);
            }

            if planets.get(entity).is_some() {
                planets_batch.append(*atlas_image, iso.0, size.0);
            }
            if _fog_markers.get(entity).is_some() {
                fog_batch.append(*atlas_image, iso.0, size.0)
            }
        }
        all_batch.extend(stars_batch);
        all_batch.extend(nebulas_batch);
        all_batch.extend(planets_batch);
        all_batch.extend(fog_batch);
        for (entity, iso, atlas_image, size, (), (), (), ()) in (
            &entities,
            &isometries,
            &atlas_images,
            &sizes,
            !&stars,
            !&planets,
            !&nebulas,
            !&_fog_markers,
        )
            .join()
        {
            let mut image = *atlas_image;
            let intensity = if let Some(flash) = damage_flash.get(entity) {
                flash.0
            } else {
                0f32
            };
            image.color = (1f32, 1f32, 1f32, intensity);
            all_batch.append(image, iso.0, size.0);
        }
        // flame::start("animation");
        for (iso, size, animation) in
            (&isometries, &sizes, &mut animations).join()
        {
            if visible(&*canvas, &iso.0, dims) {
                let animation_frame = animation.next_frame();
                if let Some(animation_frame) = animation_frame {
                    all_batch.append(animation_frame.image, iso.0, size.0);
                };
            }
        }
        // cursor image
        {
            let cursor_iso = Isometry::new(mouse.x, mouse.y, 0f32);
            let cursor_image = preloaded_images.cursor;
            let cursor_size = 0.5f32;
            let cursor_scale = if mouse.left { 1f32 } else { 2f32 };
            all_batch.append(
                cursor_image,
                cursor_iso.0,
                cursor_size * cursor_scale,
            );
        }
        // speed glow
        {
            if let Some((iso, vel, _char_marker)) =
                (&isometries, &velocities, &character_markers).join().next()
            {
                let mut glow_image = preloaded_images.basic_ship;
                glow_image.transparency = vel.0.norm().min(1f32);
                all_batch.append(glow_image, iso.0, 1f32);
            }
        }
        let sprite_batch = SpriteBatch::new(
            &gl,
            &all_batch.images,
            &all_batch.isometries,
            &all_batch.sizes,
        );
        canvas.render_sprite_batch(
            &gl,
            &viewport,
            &mut frame,
            &sprite_batch,
            true,
            None,
        );
        flame::end("sprite batch rendering");
        flame::start("particles rendering");
        for (entity, particles_data) in (&entities, &mut particles_datas).join()
        {
            match **particles_data {
                ParticlesData::Explosion(ref mut particles) => {
                    if particles.update() {
                        canvas.render_instancing(
                            &gl,
                            &viewport,
                            &mut frame,
                            &particles.instancing_data,
                            &Isometry3::new(
                                Vector3::new(0f32, 0f32, 0f32),
                                Vector3::new(0f32, 0f32, 0f32),
                            ),
                        );
                    } else {
                        entities.delete(entity).unwrap();
                    }
                }
                _ => (),
            };
        }

        let (iso, vel) = if let Some((iso, vel, _char_marker)) =
            (&isometries, &velocities, &character_markers).join().next()
        {
            (*iso, *vel)
        } else {
            (
                Isometry::new(0f32, 0f32, 0f32),
                Velocity::new(MENU_VELOCITY.0, MENU_VELOCITY.1),
            )
        };
        {
            let translation_vec = iso.0.translation.vector;
            let mut isometry =
                Isometry3::new(translation_vec, Vector3::new(0f32, 0f32, 0f32));
            let pure_isometry = isometry.clone();
            isometry.translation.vector.z = canvas.get_z_shift();
            match **particles_datas
                .get_mut(preloaded_particles.movement)
                .unwrap()
            {
                ParticlesData::MovementParticles(ref mut particles) => {
                    particles.update(1.0 * Vector2::new(-vel.0.x, -vel.0.y));
                    canvas.render_instancing(
                        &gl,
                        &viewport,
                        &mut frame,
                        &particles.instancing_data,
                        &pure_isometry,
                    );
                }
                _ => panic!(),
            };
        }

        flame::end("particles rendering");

        flame::start("other");
        let mut render_lazer =
            |iso: &Isometry, lazer: &Lazer, force_rendering: bool, rotation| {
                if lazer.active || force_rendering {
                    let h = lazer.current_distance;
                    let w = 0.05f32;
                    let positions = vec![
                        Vector2::new(-w / 2.0, 0f32),
                        Vector2::new(w / 2.0, 0f32),
                        Vector2::new(0.0, -h), // hmmmmm, don't know why minus
                    ];
                    let positions: Vec<Point2> = positions
                        .into_iter()
                        .map(|v: Vector2| Point2::from(rotation * v))
                        .collect();
                    let indices = [0u16, 1, 2];
                    // let geometry_data = GeometryData::new(&gl, &positions, &indices).unwrap();
                    let iso2 = iso3_iso2(&iso.0);
                    let mut triangulation = Triangulation {
                        points: positions,
                        indicies: indices.to_vec(),
                    };
                    triangulation.apply(iso2);
                    return triangulation;
                    // canvas.render_geometry(
                    //     &gl,
                    //     &viewport,
                    //     &mut frame,
                    //     &geometry_data,
                    //     &iso.0,
                    //     RenderMode::StencilCheck,
                    //     Point3::new(1.0, 0.0, 0.0),
                    // );
                }
                Triangulation::new()
            };
        let zero_rotation = Rotation2::new(0.0);
        let mut lazer_geometries = Triangulation::new();
        for (rift, isometry) in (&rifts, &isometries).join() {
            for (lazer, dir) in rift.lazers.iter() {
                let pos = isometry.0.translation.vector;
                let up = Vector2::new(0.0, -1.0);
                let dir = Vector2::new(dir.0, dir.1);
                let rotation = Rotation2::rotation_between(
                    &up,
                    &Vector2::new(dir.x, dir.y),
                );
                let isometry = Isometry3::new(
                    Vector3::new(pos.x, pos.y, pos.z),
                    Vector3::new(0f32, 0f32, rotation.angle()),
                );
                let lazer_geom = render_lazer(
                    &Isometry(isometry),
                    &lazer,
                    false,
                    zero_rotation,
                );
                lazer_geometries.extend(lazer_geom);
            }
        }
        let iso0 = Isometry3::new(
            Vector3::new(0f32, 0f32, 0f32),
            Vector3::new(0f32, 0f32, 0f32),
        );
        // let mut lazers = Triangulation::new();
        for (iso, multy_lazer) in (&isometries, &multy_lazers).join() {
            for (angle, lazer) in multy_lazer.iter() {
                // let rotation = Rotation2::new(i as f32 * std::f32::consts::PI / 2.0);
                let rotation = Rotation2::new(angle);
                lazer_geometries
                    .extend(render_lazer(iso, lazer, false, rotation));
            }
        }
        // render lazers with only one draw call
        let geometry_data = GeometryData::new(
            &gl,
            &lazer_geometries.points,
            &lazer_geometries.indicies,
        )
        .unwrap();
        canvas.render_geometry(
            &gl,
            &viewport,
            &mut frame,
            &geometry_data,
            &iso0,
            RenderMode::StencilCheck,
            Point3::new(1.0, 0.0, 0.0),
        );
        flame::end("other");
        flame::start("asteroids rendering");
        // let mut asteroids_triangulation = Triangulation::new();
        for (_entity, iso, _size, geom_data, _asteroid) in (
            &entities,
            &isometries,
            &sizes,
            &geom_datas,
            &asteroid_markers,
        )
            .join()
        {
            if visible(&*canvas, &iso.0, dims) {
                canvas.render_geometry(
                    &gl,
                    &viewport,
                    &mut frame,
                    &geom_data,
                    &iso.0,
                    RenderMode::Draw,
                    Point3::new(0.8, 0.8, 0.8),
                )
            }
        }
        flame::end("asteroids rendering");
        let _render_line = |a: Point2, b: Point2| {
            let line_width = 0.05;
            let line_length = (b.coords - a.coords).norm();
            let positions = vec![
                Point2::new(-line_width / 2.0, 0f32),
                Point2::new(line_width / 2.0, 0f32),
                Point2::new(-line_width / 2.0, -line_length),
                Point2::new(line_width / 2.0, -line_length),
            ];
            let up = Vector2::new(0.0, -line_length);
            let rotation =
                Rotation2::rotation_between(&up, &(&b.coords - a.coords));
            let iso = Isometry3::new(
                Vector3::new(a.x, a.y, 0f32),
                Vector3::new(0f32, 0f32, rotation.angle()),
            );
            let indices = [0u16, 1, 2, 0, 2, 3];
            let geometry_data =
                GeometryData::new(&gl, &positions, &indices).unwrap();
            canvas.render_geometry(
                &gl,
                &viewport,
                &mut frame,
                &geometry_data,
                &iso,
                RenderMode::Draw,
                Point3::new(1f32, 1f32, 1f32),
            );
        };
        for (iso, text, lifetime) in (&isometries, &mut text_components, &lifetimes).join() {
            let lifetime_fraction = lifetime.rest_fraction();
            text.color.3 = lifetime_fraction;
            ui.primitives.push(Primitive {
                kind: PrimitiveKind::Text(Text {
                    position: Point2::new(
                        iso.0.translation.vector.x,
                        iso.0.translation.vector.y,
                    ),
                    color: text.color,
                    text: text.text.clone(),
                    font_size: 1.0
                }),
                with_projection: true,
            });
        }
        for (text, lifetime, position) in (&mut text_components, &lifetimes, &positions2d).join() {
            let lifetime_fraction = lifetime.rest_fraction();
            text.color.3 = lifetime_fraction;
            ui.primitives.push(Primitive {
                kind: PrimitiveKind::Text(Text {
                    position: Point2::new(
                        position.0.x,
                        position.0.y,
                    ),
                    color: text.color,
                    text: text.text.clone(),
                    font_size: 13.0
                }),
                with_projection: false,
            });
        }
        for (i, (iso, _)) in (&isometries, &ship_markers).join().enumerate() {}
        flame::start("primitives rendering");
        primitives_channel.iter_write(ui.primitives.drain(..));
        sounds_channel.iter_write(ui.sounds.drain(..));
        render_primitives(
            &mouse,
            &mut self.reader,
            &mut frame,
            &gl,
            &mut canvas,
            &viewport,
            &mut primitives_channel,
            &mut text_data,
            &mut world_text_data,
        );
        flame::end("primitives rendering");
        // for (name, span) in time_spans.iter() {
        //     telegraph.insert(name.to_string(), span.evaluate().as_millis() as f32 / 1000.0 * 60.0); // TODO "xFPS" actually
        // }
        flame::end("rendering");
        let spans = flame::spans();
        telegraph.insert("fps".to_string(), dev_info.fps as f32 / 60.0);
        let rendered_spans = [
            "rendering".to_string(),
            "dispatch".to_string(),
            "insert".to_string(),
            // "asteroids".to_string(),
            // "asteroids rendering".to_string(),
            // "foreground rendering".to_string(),
            // "background rendering".to_string(),
            // "shadow rendering".to_string(),
            // "sprite batch rendering".to_string(),
            // "clear".to_string()
        ];
        if dev_info.draw_telemetry {
            for span in spans.iter() {
                if rendered_spans.contains(&span.name.to_string()) {
                    telegraph.insert(
                        span.name.to_string(),
                        span.delta as f32 / 1E9 * 60.0,
                    );
                }
                if span.name == "dispatch" {
                    for subspan in span.children.iter() {
                        telegraph.insert(
                            subspan.name.to_string(),
                            subspan.delta as f32 / 1E9 * 60.0,
                        );
                    }
                }
                if span.name == "rendering" {
                    for subspan in span.children.iter() {
                        if rendered_spans.contains(&subspan.name.to_string()) {
                            telegraph.insert(
                                subspan.name.to_string(),
                                subspan.delta as f32 / 1E9 * 60.0,
                            );
                        }
                    }
                }
            }
            for name in telegraph.iter_names() {
                if let Some(plot) = telegraph.iter(name.to_string()) {
                    render_plot(
                        plot.0, plot.1, 14.0, 10.0, &gl, &viewport, &canvas,
                        &mut frame,
                    );
                }
            }
        }
    }
}
