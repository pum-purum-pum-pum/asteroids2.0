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

const Z_CANVAS: f32 = 0f32;
const Z_FAR: f32 = 10f32;
const MAX_ADD_SPEED_Z: f32 = 10f32;
const SPEED_EMA: f32 = 0.04f32; // new value will be taken with with that coef
pub const _BACKGROUND_SIZE: f32 = 20f32;
const ENGINE_FAR: f32 = 3f32;

#[derive(Copy, Clone)]
#[repr(C, packed)]
#[derive(VertexAttribPointers)]
pub struct GeometryVertex {
    pub position: red::data::f32_f32,
}

pub struct GeometryData {
    positions: GeometryVertexBuffer,
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

#[derive(Copy, Clone)]
#[repr(C, packed)]
#[derive(VertexAttribPointers)]
pub struct Vertex {
    pub position: red::data::f32_f32,
    pub tex_coords: red::data::f32_f32,
}

pub struct ImageData {
    positions: VertexBuffer,
    indices: red::buffer::IndexBuffer,
    texture: red::shader::Texture,
    dim_scales: Vector2,
}

impl ImageData {
    pub fn new(gl: &red::GL, image_name: &str) -> Result<Self, String> {
        let positions = vec![(-1f32, -1f32), (-1f32, 1f32), (1f32, 1f32), (1f32, -1f32)];
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

pub fn create_shader_program(name: String, gl: &red::GL) -> Result<red::Program, String> {
    let vertex = format!("gles/v_{}.glsl", name);
    let fragment = format!("gles/f_{}.glsl", name);
    let (mut vertex_shader, mut fragment_shader) = (
        read_file(&vertex).unwrap(),
        read_file(&fragment).unwrap()
    );
    // vertex_shader = glesit(&read_file(&vertex).unwrap(), ShaderType::Vertex, Version::V100);
    // fragment_shader = glesit(&read_file(&fragment).unwrap(), ShaderType::Fragment, Version::V100);
    // #[cfg(any(target_os = "ios", target_os = "android", target_os = "emscripten"))]
    // {
    //     use std::fs::{self, DirEntry};
    //     use std::path::Path;
    //     trace!("{:?}", read_file(&vertex));
    //     trace!("{:?}", read_file(&fragment));
    //     vertex_shader = glesit(&read_file(&vertex).unwrap(), ShaderType::Vertex, Version::V100);
    //     fragment_shader = glesit(&read_file(&fragment).unwrap(), ShaderType::Fragment, Version::V100);
    //     trace!("{}", vertex_shader);
    //     trace!("{}", fragment_shader);
    // }
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
    pub fn new(gl: &red::GL) -> Result<Self, String> {
        let program = create_shader_program("".to_string(), gl)?;
        let program_primitive = create_shader_program("primitive".to_string(), gl)?;
        let program_primitive_texture = create_shader_program("primitive_texture".to_string(), gl)?;
        let program_light = create_shader_program("light".to_string(), gl)?;
        let program_instancing = create_shader_program("instancing".to_string(), gl)?;
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
        vao: &red::buffer::VertexArray,
        geometry_data: &GeometryData,
        model: &Isometry3,
        // stencil: bool,
    ) {
        let model: [[f32; 4]; 4] = model.to_homogeneous().into();
        let dims = (viewport.x as u32, viewport.y as u32);
        let perspective: [[f32; 4]; 4] = perspective(dims.0, dims.1).to_homogeneous().into();
        let view: [[f32; 4]; 4] = get_view(self.observer).to_homogeneous().into();
        let program = &self.program_light;
        program.set_uniform("model", model);
        program.set_uniform("view", view);
        program.set_uniform("perspective", perspective);
        program.set_layout(&gl, &vao, &[&geometry_data.positions]);
        let draw_type = red::DrawType::Standart;
        frame.draw(&vao, Some(&geometry_data.index_buffer), program, &draw_type);
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




fn is_precision(word: &str) -> bool {
    let precision = vec![
        "vec2",
        "vec4",
        "vec3",
        "mat2",
        "mat3",
        "mat4",
        "mat5",
        "float",
    ];
    for p in precision.iter() {
        if word == *p {
            return true;
        }
    }
    false
}

#[derive(PartialEq)]
pub enum ShaderType {
    Vertex,
    Fragment
}

pub enum Version {
    V300,
    V100,
}

pub fn glesit(src: &str, shader_type: ShaderType, to_version: Version) -> String {
    let lines: Vec<_> = src.split("\n").collect();
    let mut find_and_replace = match to_version {
        Version::V300 => {
            match shader_type {
                ShaderType::Vertex => {
                    vec![("#version 130", "#version 300 es")]
                }
                ShaderType::Fragment => {
                    vec![
                        ("#version 130", "#version 300 es\nout mediump vec4  astro_FragColor;"),
                        ("gl_FragColor", "astro_FragColor")
                    ]
                }
            }
        }
        Version::V100 => {
            let mut res = vec![("#version 130", "#version 100")];
            match shader_type {
                ShaderType::Fragment => {
                    res.push(("texture(", "texture2D("));
                }
                ShaderType::Vertex => ()
            };
            res
        }
    };
    let mut subst = HashMap::new();
    match to_version {
        Version::V100 => {
            match shader_type {
                ShaderType::Vertex => {
                    subst.insert("in", "attribute");
                }
                ShaderType::Fragment => {
                    subst.insert("in", "varying");
                }
            }
            subst.insert("out", "varying");
        }
        Version::V300 => {}
    };
    let mut new_lines = vec!();
    for line in lines.iter() {
        let words: Vec<_> = line.split(" ").collect();
        let mut new_words = vec![];
        let mut last_word = String::new();
        for w in words.iter() {
            if is_precision(w) && (last_word != "in".to_string() || shader_type == ShaderType::Fragment) {
                new_words.push("mediump".to_string());
            }
            match subst.get(w) {
                Some(&new_word) => {
                    new_words.push(String::from_str(new_word).unwrap())
                }
                None => {
                    new_words.push(String::from_str(w).unwrap())
                }
            }
            last_word = String::from_str(w).unwrap();
        }
        let mut new_line = new_words.join(" ");
        for (f, r) in find_and_replace.iter() {
            new_line = new_line.replace(f, r);
        }
        new_lines.push(new_line)
    }
    new_lines.join("\n")
}
