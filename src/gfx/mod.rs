use crate::types::{*};
use rand::prelude::*;
use std::fs::File;
use std::io::{BufReader, Error as IOError, Read};
use std::str::FromStr;
use std::collections::HashMap;
use nalgebra::geometry::Orthographic3;

use red;
use red::VertexAttribPointers;
use red::glow::Context;
use red::shader::UniformValue;
use image;
use image::ImageError;
use sdl2::rwops::RWops;
use std::path::Path;

mod effects;
pub use effects::*;

const Z_CANVAS: f32 = 0f32;
const Z_FAR: f32 = 10f32;
const MAX_ADD_SPEED_Z: f32 = 10f32;
const SPEED_EMA: f32 = 0.04f32; // new value will be taken with with that coef
pub const _BACKGROUND_SIZE: f32 = 20f32;
const ENGINE_FAR: f32 = 3f32;

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
#[derive(VertexAttribPointers)]
pub struct GeometryVertex {
    pub position: red::data::f32_f32,
}

pub struct GeometryData {
    positions: GeometryVertexBuffer<GeometryVertex>,
    index_buffer: red::buffer::IndexBuffer,
}

impl GeometryData {
    pub fn new(gl: &red::GL, positions: &[Point2], indices: &[u16]) -> Result<Self, String> {
        let shape: Vec<GeometryVertex> = positions
            .iter()
            .map(|pos| GeometryVertex {
                position: red::data::f32_f32::new(pos.x, pos.y),
            })
            .collect();
        let vertex_buffer = GeometryVertexBuffer::new(gl, &shape)?;
        let index_buffer = red::buffer::IndexBuffer::new(gl , &indices)?;
        Ok(GeometryData{
            positions: vertex_buffer,
            index_buffer: index_buffer
        })
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
#[derive(VertexAttribPointers)]
pub struct Vertex {
    pub position: red::data::f32_f32,
    pub tex_coords: red::data::f32_f32,
}


#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
#[derive(VertexAttribPointers)]
pub struct WorldVertex {
    #[divisor = "1"]
    pub world_position: red::data::f32_f32_f32,
}

pub struct InstancingData {
    pub vertex_buffer: GeometryVertexBuffer<GeometryVertex>,
    pub indices: red::buffer::IndexBuffer,
    pub per_instance: WorldVertexBuffer<WorldVertex>,
}


pub struct ImageData {
    positions: VertexBuffer<Vertex>,
    indices: red::buffer::IndexBuffer,
    texture: red::shader::Texture,
    dim_scales: Vector2,
}

impl ImageData {
    pub fn new(gl: &red::GL, image_name: &str) -> Result<Self, String> {
        let positions = vec![(-1f32, 1f32), (-1f32, -1f32), (1f32, -1f32), (1f32, 1f32)];
        let textures = vec![(0f32, 0f32), (0f32, 1f32), (1f32, 1f32), (1f32, 0f32)];
        let shape: Vec<Vertex> = positions
            .into_iter()
            .zip(textures)
            .map(|(pos, tex)| Vertex {
                position: pos.into(),
                tex_coords: tex.into(),
            })
            .collect();
        let vertex_buffer = VertexBuffer::new(gl, &shape)?;
        let index_buffer = red::buffer::IndexBuffer::new(gl, &[0u16, 1, 2, 2, 3, 0])?;
        let texture = load_texture(gl, image_name);
        let dimensions = texture.dimensions();
        let dimensions = Vector2::new(1.0, dimensions.1 as f32 / dimensions.0 as f32);
        Ok(ImageData {
            positions: vertex_buffer,
            indices: index_buffer,
            texture: texture,
            dim_scales: dimensions,
        })
    }
}


pub fn read_file(filename: &str) -> Result<String, IOError> {
    let mut result_str = String::new();
    let mut rw = RWops::from_file(Path::new(filename), "r").unwrap();
    rw.read_to_string(&mut result_str)?;
    Ok(result_str)
}

pub fn load_texture(gl: &red::GL, name: &str) -> red::shader::Texture {
    let path_str = format!("assets/{}.png", name);
    let texture_file = RWops::from_file(Path::new(&path_str), "r").unwrap();
    let reader = BufReader::new(texture_file);
    let image = image::load(reader, image::PNG).unwrap().to_rgba();
    let image_dimensions = image.dimensions();
    let image =
        red::shader::Texture::from_rgba8(gl, image_dimensions.0, image_dimensions.1, &image.into_raw());
    image
}

pub fn create_shader_program(gl: &red::GL, name: &str, glsl_version: &str) -> Result<red::Program, String> {
    let vertex = format!("gles/v_{}.glsl", name);
    let fragment = format!("gles/f_{}.glsl", name);
    let (mut vertex_shader, mut fragment_shader) = (
        format!("{}\n{}", glsl_version, read_file(&vertex).unwrap()),
        format!("{}\n{}", glsl_version, read_file(&fragment).unwrap())
    );
    #[cfg(any(target_os = "ios", target_os = "android", target_os = "emscripten"))]
    trace!("{:?} \n {:?} \n {:?}", vertex_shader, "---", fragment_shader);
    let vertex_shader = red::Shader::from_vert_source(&gl, &vertex_shader).unwrap();
    let fragment_shader = red::Shader::from_frag_source(&gl, &fragment_shader).unwrap();
    let program = red::Program::from_shaders(&gl, &[vertex_shader, fragment_shader])?;
    Ok(program)
}

/// 2D graphics
pub struct Canvas {
    program: red::Program,       // @vlad TODO: we want to use many programs
    program_light: red::Program, // but for now simpler=better
    program_instancing: red::Program,
    program_primitive: red::Program,
    program_primitive_texture: red::Program,
    observer: Point3,
    // default_params: glium::DrawParameters<'a>,
    // stencil_check_params: glium::DrawParameters<'a>,
    // stencil_write_params: glium::DrawParameters<'a>,
}

impl Canvas {
    pub fn new(gl: &red::GL, glsl_version: &str) -> Result<Self, String> {
        let program = create_shader_program(gl, "", glsl_version)?;
        let program_primitive = create_shader_program(gl, "primitive", glsl_version)?;
        let program_primitive_texture = create_shader_program(gl, "primitive_texture", glsl_version)?;
        let program_light = create_shader_program(gl, "light", glsl_version)?;
        let program_instancing = create_shader_program(gl, "instancing", glsl_version)?;
        Ok(Canvas {
            program: program,
            program_primitive: program_primitive,
            program_primitive_texture: program_primitive_texture,
            program_light: program_light,
            program_instancing: program_instancing,
            observer: Point3::new(0f32, 0f32, Z_FAR),
        })
    }

    pub fn observer(&self) -> Point3 {
        self.observer
    }

    pub fn update_observer(&mut self, pos: Point2, speed_ratio: f32) {
        self.observer.x = pos.x;
        self.observer.y = pos.y;
        self.observer.z = (1.0 - SPEED_EMA) * self.observer.z
            + SPEED_EMA * (Z_FAR + MAX_ADD_SPEED_Z * speed_ratio);
    }

    pub fn get_z_shift(&self) -> f32 {
        self.observer.z - Z_FAR
    }

    pub fn render_geometry(
        &self,
        gl: &red::GL,
        viewport: &red::Viewport,
        frame: &mut red::Frame,
        geometry_data: &GeometryData,
        model: &Isometry3,
        // stencil: bool,
    ) {
        let model: [[f32; 4]; 4] = model.to_homogeneous().into();
        let dims = (viewport.x as u32, viewport.y as u32);
        let perspective: [[f32; 4]; 4] = perspective(dims.0, dims.1).to_homogeneous().into();
        let view: [[f32; 4]; 4] = get_view(self.observer).to_homogeneous().into();
        let vao = &geometry_data.positions.vao;
        let program = &self.program_light;
        program.set_uniform("model", model);
        program.set_uniform("view", view);
        program.set_uniform("perspective", perspective);
        program.set_layout(&gl, vao, &[&geometry_data.positions]);
        let draw_type = red::DrawType::Standart;
        frame.draw(vao, Some(&geometry_data.index_buffer), program, &draw_type);
    }

    pub fn render_primitive(
        &self,
        gl: &red::GL,
        viewport: &red::Viewport,
        frame: &mut red::Frame,
        geometry_data: &GeometryData,
        model: &Isometry3,
        fill_color: (f32, f32, f32),
        with_projection: bool,
    ) {
        let model: [[f32; 4]; 4] = model.to_homogeneous().into();
        let dims = viewport.dimensions();
        let dims = (dims.0 as u32, dims.1 as u32);
        let (projection, view) = if with_projection {
            let perspective: [[f32; 4]; 4] = perspective(dims.0, dims.1).to_homogeneous().into();
            let view: [[f32; 4]; 4] = get_view(self.observer).to_homogeneous().into();
            (perspective, view)
        } else {
            let orthographic: [[f32; 4]; 4] = orthographic(dims.0, dims.1).to_homogeneous().into();
            let view: [[f32; 4]; 4] = Matrix4::identity().into();
            (orthographic, view)
        };
        let vao = &geometry_data.positions.vao;
        let program = &self.program_primitive;
        program.set_uniform("model", model);
        program.set_uniform("view", view);
        program.set_uniform("projection", projection);
        program.set_uniform("fill_color", fill_color);
        program.set_layout(&gl, vao, &[&geometry_data.positions]);
        let draw_type = red::DrawType::Standart;
        frame.draw(vao, Some(&geometry_data.index_buffer), program, &draw_type)
    }

    pub fn render(
        &self,
        gl: &red::GL,
        viewport: &red::Viewport,
        frame: &mut red::Frame,
        image_data: &ImageData,
        model: &Isometry3,
        scale: f32,
        with_lights: bool,
    ) {
        let model: [[f32; 4]; 4] = model.to_homogeneous().into();
        let dims = viewport.dimensions();
        let dims = (dims.0 as u32, dims.1 as u32);
        let texture = &image_data.texture;
        // let draw_params = if with_lights {
        //     &self.stencil_check_params
        // } else {
        //     &self.default_params
        // };
        let scales = image_data.dim_scales;
        let perspective: [[f32; 4]; 4] = perspective(dims.0, dims.1).to_homogeneous().into();
        let view: [[f32; 4]; 4] = get_view(self.observer).to_homogeneous().into();
        let vao = &image_data.positions.vao;
        let program = &self.program;
        program.set_uniform("model", model);
        program.set_uniform("view", view);
        program.set_uniform("perspective", perspective);
        program.set_uniform("dim_scales", (scales.x, scales.y));
        program.set_uniform("tex", texture.clone());
        program.set_uniform("scale", scale);
        program.set_layout(&gl, vao, &[&image_data.positions]);
        let draw_type = red::DrawType::Standart;
        frame.draw(
            vao, 
            Some(&image_data.indices), 
            &program, 
            &draw_type
        );
    }

    pub fn render_instancing(
        &self,
        gl: &red::GL,
        viewport: &red::Viewport,
        frame: &mut red::Frame,
        instancing_data: &InstancingData,
        model: &Isometry3,
        // transparency: f32,
    ) {
        let model: [[f32; 4]; 4] = model.to_homogeneous().into();
        let dims = viewport.dimensions();
        let dims = (dims.0 as u32, dims.1 as u32);
        let perspective: [[f32; 4]; 4] = perspective(dims.0, dims.1).to_homogeneous().into();
        let view: [[f32; 4]; 4] = get_view(self.observer).to_homogeneous().into();
        let vao = &instancing_data.vertex_buffer.vao;
        let program = &self.program_instancing;
        program.set_uniform("model", model);
        program.set_uniform("view", view);
        program.set_uniform("perspective", perspective);
        program.set_uniform("transparency", 1f32);
        program.set_layout(&gl, vao, &[&instancing_data.vertex_buffer, &instancing_data.per_instance]);
        let draw_type = red::DrawType::Instancing(instancing_data.per_instance.len);
        frame.draw(
            vao, 
            Some(&instancing_data.indices),
            &program, 
            &draw_type
        );
    }

    pub fn render_primitive_texture(
        &self,
        gl: &red::GL,
        viewport: &red::Viewport,
        frame: &mut red::Frame,
        image_data: &ImageData,
        model: &Isometry3,
        with_projection: bool,
        size: f32,
    ) {
        let model: [[f32; 4]; 4] = model.to_homogeneous().into();
        let dims = viewport.dimensions();
        let dims = (dims.0 as u32, dims.1 as u32);
        let vao = &image_data.positions.vao;
        let program = &self.program_primitive_texture;
        let (projection, view) = if with_projection {
            let perspective: [[f32; 4]; 4] = perspective(dims.0, dims.1).to_homogeneous().into();
            let view: [[f32; 4]; 4] = get_view(self.observer).to_homogeneous().into();
            (perspective, view)
        } else {
            let orthographic: [[f32; 4]; 4] = orthographic(dims.0, dims.1).to_homogeneous().into();
            let view: [[f32; 4]; 4] = Matrix4::identity().into();
            (orthographic, view)
        };
        program.set_uniform("model", model);
        program.set_uniform("view", view);
        program.set_uniform("projection", projection);
        program.set_uniform("tex", image_data.texture.clone());
        program.set_uniform("size", size);
        program.set_layout(&gl, vao, &[&image_data.positions]);

        let draw_type = red::DrawType::Standart;
        frame.draw(vao, Some(&image_data.indices), &program, &draw_type);
    }
}


pub fn get_view(observer: Point3) -> Isometry3 {
    let mut target = observer.clone();
    target.z = Z_CANVAS;
    Isometry3::look_at_rh(&observer, &target, &Vector3::y())
}

pub fn orthographic_from_zero(width: u32, height: u32) -> Orthographic3<f32> {
    Orthographic3::new(0f32, width as f32, 0f32, height as f32, -0.9, 0.0)
} 

// creates ortograohic projection left=bot=0 z_near=0.1 far=1.0
pub fn orthographic(width: u32, height: u32) -> Orthographic3<f32>{
    Orthographic3::new(0f32, width as f32, 0f32, height as f32, 0.1, 1f32)
}

pub fn perspective(width: u32, height: u32) -> Perspective3 {
    let aspect_ratio = width as f32 / height as f32;
    Perspective3::new(aspect_ratio, 3.14 / 3.0, 0.1, 1000.0)
}

pub fn ortho_unproject(width: u32, height: u32, point: Point2) -> Point2 {
    let ortho: Matrix4 = orthographic(width, height).into();
    let unortho = ortho.try_inverse().unwrap();
    let res = unortho * Point4::new(point.x, point.y, 1f32, 1f32);
    Point2::new(res.x, res.y)
}

pub fn unproject(
    observer: Point3,
    window_coord: &Point2,
    width: u32,
    height: u32,
) -> (Point3, Vector3) {
    let begin_ray = Point4::new(window_coord.x, window_coord.y, 0f32, 1f32);
    let ingame_window_coord = Point4::new(window_coord.x, window_coord.y, Z_FAR, 1f32);
    let perspective: Matrix4 = perspective(width, height).into();
    let view: Matrix4 = get_view(observer).to_homogeneous().into();
    let inverse_transform = (perspective * view).try_inverse().unwrap();
    let unprojected_begin = inverse_transform * begin_ray;
    let unprojected_end = inverse_transform * ingame_window_coord;
    let unprojected_begin = Point3::from_homogeneous(unprojected_begin.coords).unwrap();
    let unprojected_end = Point3::from_homogeneous(unprojected_end.coords).unwrap();
    // * Why (perspective * view)^-1
    // * Exlanation:
    // * * this coords then passed to the Isometry and then as model to shader
    // * * the order is:
    // * * perspective * view * model
    (
        unprojected_begin,
        (unprojected_end - unprojected_begin).normalize(),
    )
}

pub fn unproject_with_z(
    observer: Point3,
    window_coord: &Point2,
    z_coord: f32,
    width: u32,
    height: u32,
) -> Point3 {
    let (pos, dir) = unproject(observer, window_coord, width, height);
    let z_safe_scaler = (-pos.z + z_coord) / dir.z;
    return pos + dir * z_safe_scaler;
}