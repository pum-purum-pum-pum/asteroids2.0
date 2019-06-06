use std::fs::{File};
use std::io::{BufReader, Error as IOError};
use astro_lib as al;
use al::prelude::*;

use glium;
use glium::Surface;
use glium::{implement_vertex, uniform, DrawError};
use glium::index::PrimitiveType;
use glium::texture::{SrgbTexture2d, TextureCreationError};
use glium::draw_parameters::Blend;
use nalgebra::{Isometry3};
use image;
use image::{ImageError};

use crate::gfx_backend::SDL2Facade;

#[derive(Copy, Clone)]
pub struct Vertex2 {
    pub position: [f32; 2],
}

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
}
implement_vertex!(Vertex, position, tex_coords);

#[derive(Debug)]
pub enum LoadTextureError {
    CreationError(TextureCreationError),
    ImageError(ImageError),
    IOError(IOError),
}

impl From<ImageError> for LoadTextureError {
    fn from(image_error: ImageError) -> LoadTextureError {
        LoadTextureError::ImageError(image_error)
    }
}

impl From<IOError> for LoadTextureError {
    fn from(io_error: IOError) -> LoadTextureError {
        LoadTextureError::IOError(io_error)
    }
}

impl From<TextureCreationError> for LoadTextureError {
    fn from(texture_error: TextureCreationError) -> LoadTextureError {
        LoadTextureError::CreationError(texture_error)
    }
}

type LoadTextureResult = Result<SrgbTexture2d, LoadTextureError>;

pub fn load_texture(display: &SDL2Facade, name: &str) -> LoadTextureResult {
    let path_str = &format!(
        "{}/assets/{}.png",
        env!("CARGO_MANIFEST_DIR"),
        name
    );
    let texture_file = File::open(path_str)?;
    let reader = BufReader::new(texture_file);
    let image = image::load(reader, image::PNG)?.to_rgba();
    let image_dimensions = image.dimensions();
    let image =
        glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
    let texture = glium::texture::SrgbTexture2d::new(display, image)?;
    Ok(texture)
}

/// Contains all data that is used by glium to render image
pub struct ImageData{
    positions: glium::VertexBuffer<Vertex>,
    indices: glium::IndexBuffer<u16>,
    texture: glium::texture::SrgbTexture2d,
    dim_scales: Vector2,
    scale: f32,
}

impl ImageData {
    /// panic if failed to create buffers. TODO Result
    /// image_name - is name of the image to load in assets directory
    pub fn new(display: &SDL2Facade, image_name: &str, scale: f32) -> Result<Self, LoadTextureError> {
        let positions = vec![
            [-1f32, -1f32],
            [-1f32, 1f32],
            [1f32, 1f32],
            [1f32, -1f32]
        ];
        let textures = vec![
            [0f32, 0f32],
            [0f32, 1f32],
            [1f32, 1f32],
            [1f32, 0f32]
        ];
        let shape: Vec<Vertex> = positions
            .into_iter().zip(textures)
            .map(|(pos, tex)| { Vertex{position: pos, tex_coords: tex} })
            .collect();
        let vertex_buffer = glium::VertexBuffer::new(display, &shape).unwrap();
        let indices = glium::IndexBuffer::new(
            display,
            PrimitiveType::TrianglesList,
            &[0u16, 1, 2, 2, 3, 0],
        ).unwrap();
        let texture = load_texture(display, image_name)?;
        let dimensions = texture.dimensions();
        let mut dimensions = Vector2::new(1.0, dimensions.1 as f32 / dimensions.0 as f32);
        Ok(ImageData {
            positions: vertex_buffer,
            indices: indices,
            texture: texture,
            dim_scales: dimensions,
            scale: scale,
        })
    }
}

/// 2D graphics on screen
pub struct Canvas {
    program: glium::Program, // @vlad TODO: we want to use many programs
    observer: nalgebra::Isometry3<f32>,
    observer_current: nalgebra::Isometry3<f32>,
}

impl Canvas {
    pub fn new(display: &SDL2Facade) -> Self{
        let vertex_shader_src = r#"
            #version 130
            in vec2 tex_coords;
            in vec2 position;
            out vec2 v_tex_coords;

            uniform mat4 perspective;
            uniform mat4 view;
            uniform mat4 model;
            uniform float scale;
            uniform vec2 dim_scales;
            
            vec2 position_scaled;

            void main() {
                v_tex_coords = tex_coords;
                position_scaled = scale * dim_scales * position;
                gl_Position = perspective * view * model * vec4(position_scaled, 0.0, 1.0);
            }
        "#;

        let fragment_shader_src = r#"
            #version 130
            in vec2 v_tex_coords;
            out vec4 color;

            uniform sampler2D tex;
            void main() {
                vec4 texture_colors = vec4(texture(tex, v_tex_coords));
                color = texture_colors;
            }
        "#;
        let program = glium::Program::from_source(display, vertex_shader_src, fragment_shader_src, None).unwrap();
        Canvas{
            program: program,
            observer: 
                Isometry3::new(
                    Vector3::new(0f32, 0f32, 0f32), 
                    Vector3::new(0f32, 0f32, 0f32)
                ),
            observer_current: 
                Isometry3::new(
                    Vector3::new(0f32, 0f32, 1f32), 
                    Vector3::new(0f32, 0f32, 0f32)
                ),
        }
    }

    fn get_view(&self) -> [[f32; 4]; 4] {
        self.observer_current.to_homogeneous().into()
    }

    pub fn perspective(width: u32, height: u32) -> [[f32; 4]; 4] {
        let aspect_ratio = height as f32 / width as f32;
        let fov: f32 = 3.141592 / 3.0;
        let zfar = 1024.0;
        let znear = 0.1;
        let f = 1.0 / (fov / 2.0).tan();
        [
            [f * aspect_ratio, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, (zfar + znear) / (zfar - znear), 1.0],
            [0.0, 0.0, -(2.0 * zfar * znear) / (zfar - znear), 0.0],
        ]
    }

    pub fn render(
        &self,
        display: &SDL2Facade,
        target: &mut glium::Frame,
        image_data: &ImageData,
        model: &Isometry3<f32>,
    ) -> Result<(), DrawError> {
        let model: [[f32; 4]; 4] = model.to_homogeneous().into();
        let dims = display.get_framebuffer_dimensions();
        let processed_texture = image_data.texture
            .sampled()
            .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
            .minify_filter(glium::uniforms::MinifySamplerFilter::Linear);

        let draw_params = glium::DrawParameters {
            blend: Blend::alpha_blending(),
            ..Default::default()
        };
        let scales = image_data.dim_scales;
        target.draw(
            &image_data.positions, 
            &image_data.indices, 
            &self.program,
            &uniform! {
                model: model,
                view: self.get_view(),
                perspective: Self::perspective(dims.0, dims.1),
                tex: processed_texture,
                dim_scales: (scales.x, scales.y),
                scale: image_data.scale,
            },
            &draw_params,
        )
    }
}