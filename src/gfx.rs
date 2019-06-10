use al::prelude::*;
use astro_lib as al;
use std::fs::File;
use std::io::{BufReader, Error as IOError};

use glium;
use glium::draw_parameters::Blend;
use glium::index::PrimitiveType;
use glium::texture::{SrgbTexture2d, TextureCreationError};
use glium::Surface;
use glium::{implement_vertex, uniform, DrawError};
use image;
use image::ImageError;

use crate::gfx_backend::SDL2Facade;

const Z_CANVAS: f32 = 0f32;
const Z_FAR: f32 = 10f32;

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

#[derive(Copy, Clone)]
pub struct GeometryVertex {
    pub position: [f32; 2],
}
implement_vertex!(GeometryVertex, position);

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
    let path_str = &format!("{}/assets/{}.png", env!("CARGO_MANIFEST_DIR"), name);
    let texture_file = File::open(path_str)?;
    let reader = BufReader::new(texture_file);
    let image = image::load(reader, image::PNG)?.to_rgba();
    let image_dimensions = image.dimensions();
    let image =
        glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
    let texture = glium::texture::SrgbTexture2d::new(display, image)?;
    Ok(texture)
}

pub struct GeometryData {
    positions: glium::VertexBuffer<GeometryVertex>,
    indices: glium::IndexBuffer<u16>,
}

impl GeometryData {
    pub fn new(display: &SDL2Facade, positions: &[Point2], indices: &[u16]) -> Self {
        let shape: Vec<GeometryVertex> = positions
            .iter()
            .map(|pos| GeometryVertex {
                position: [pos.x, pos.y],
            })
            .collect();
        let vertex_buffer = glium::VertexBuffer::new(display, &shape).unwrap();
        let indices =
            glium::IndexBuffer::new(display, PrimitiveType::TrianglesList, indices).unwrap();
        GeometryData {
            positions: vertex_buffer,
            indices: indices,
        }
    }
}

/// Contains all data that is used by glium to render image
pub struct ImageData {
    positions: glium::VertexBuffer<Vertex>,
    indices: glium::IndexBuffer<u16>,
    texture: glium::texture::SrgbTexture2d,
    dim_scales: Vector2,
    scale: f32,
}

impl ImageData {
    /// panic if failed to create buffers. TODO Result
    /// image_name - is name of the image to load in assets directory
    pub fn new(
        display: &SDL2Facade,
        image_name: &str,
        scale: f32,
    ) -> Result<Self, LoadTextureError> {
        let positions = vec![[-1f32, -1f32], [-1f32, 1f32], [1f32, 1f32], [1f32, -1f32]];
        let textures = vec![[0f32, 0f32], [0f32, 1f32], [1f32, 1f32], [1f32, 0f32]];
        let shape: Vec<Vertex> = positions
            .into_iter()
            .zip(textures)
            .map(|(pos, tex)| Vertex {
                position: pos,
                tex_coords: tex,
            })
            .collect();
        let vertex_buffer = glium::VertexBuffer::new(display, &shape).unwrap();
        let indices = glium::IndexBuffer::new(
            display,
            PrimitiveType::TrianglesList,
            &[0u16, 1, 2, 2, 3, 0],
        )
        .unwrap();
        let texture = load_texture(display, image_name)?;
        let dimensions = texture.dimensions();
        let dimensions = Vector2::new(1.0, dimensions.1 as f32 / dimensions.0 as f32);
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
    program: glium::Program,       // @vlad TODO: we want to use many programs
    program_light: glium::Program, // but for now simpler=better
    observer: Point3,
}

impl Canvas {
    pub fn new(display: &SDL2Facade) -> Self {
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

        let vertex_light_shader_src = r#"
            #version 130
            in vec2 position;

            uniform mat4 perspective;
            uniform mat4 view;
            uniform mat4 model;

            void main() {
                gl_Position = perspective * view * model * vec4(position, 0.0, 1.0);
            }
        "#;

        let fragment_light_shader_src = r#"
            #version 130
            out vec4 color;

            void main() {
                color = vec4(1.0, 1.0, 1.0, 1.0);
            }
        "#;

        let program =
            glium::Program::from_source(display, vertex_shader_src, fragment_shader_src, None)
                .unwrap();
        let program_light = glium::Program::from_source(
            display,
            vertex_light_shader_src,
            fragment_light_shader_src,
            None,
        )
        .unwrap();
        Canvas {
            program: program,
            program_light: program_light,
            observer: Point3::new(0f32, 0f32, Z_FAR),
        }
    }

    pub fn observer(&self) -> Point3 {
        self.observer
    }

    pub fn update_observer(&mut self, pos: Point2) {
        self.observer.x = pos.x;
        self.observer.y = pos.y;
    }

    pub fn render(
        &self,
        display: &SDL2Facade,
        target: &mut glium::Frame,
        image_data: &ImageData,
        model: &Isometry3,
    ) -> Result<(), DrawError> {
        let model: [[f32; 4]; 4] = model.to_homogeneous().into();
        let dims = display.get_framebuffer_dimensions();
        let processed_texture = image_data
            .texture
            .sampled()
            .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
            .minify_filter(glium::uniforms::MinifySamplerFilter::Linear);

        let draw_params = glium::DrawParameters {
            stencil: glium::draw_parameters::Stencil {
                test_clockwise: glium::StencilTest::IfEqual { mask: 0xFF }, // mask which has 1 in all it's bits. u32::max_value()?
                test_counter_clockwise: glium::StencilTest::IfEqual { mask: 0xFF },
                reference_value_clockwise: 1,
                reference_value_counter_clockwise: 1,
                ..Default::default()
            },
            blend: Blend::alpha_blending(),
            ..Default::default()
        };
        let scales = image_data.dim_scales;
        let perspective: [[f32; 4]; 4] = perspective(dims.0, dims.1).to_homogeneous().into();
        let view: [[f32; 4]; 4] = get_view(self.observer).to_homogeneous().into();
        target.draw(
            &image_data.positions,
            &image_data.indices,
            &self.program,
            &uniform! {
                model: model,
                view: view,
                perspective: perspective,
                tex: processed_texture,
                dim_scales: (scales.x, scales.y),
                scale: image_data.scale,
            },
            &draw_params,
        )
    }

    pub fn render_geometry(
        &self,
        display: &SDL2Facade,
        target: &mut glium::Frame,
        image_data: &GeometryData,
        model: &Isometry3,
    ) -> Result<(), DrawError> {
        let model: [[f32; 4]; 4] = model.to_homogeneous().into();
        let dims = display.get_framebuffer_dimensions();
        // @vlad TODO move to field
        let draw_params = glium::DrawParameters {
            stencil: glium::draw_parameters::Stencil {
                test_counter_clockwise: glium::StencilTest::AlwaysPass,
                test_clockwise: glium::StencilTest::AlwaysPass,
                depth_pass_operation_counter_clockwise: glium::StencilOperation::Replace,
                depth_pass_operation_clockwise: glium::StencilOperation::Replace,
                // pass_depth_fail_operation_clockwise: glium::StencilOperation::Replace,
                reference_value_clockwise: 1,
                // reference_value_counter_clockwise: 1,
                ..Default::default()
            },
            color_mask: (false, false, false, false),
            ..Default::default()
        };
        let perspective: [[f32; 4]; 4] = perspective(dims.0, dims.1).to_homogeneous().into();
        let view: [[f32; 4]; 4] = get_view(self.observer).to_homogeneous().into();
        target.draw(
            &image_data.positions,
            &image_data.indices,
            &self.program_light,
            &uniform! {
                model: model,
                view: view,
                perspective: perspective,
            },
            &draw_params,
        )
    }
}

fn get_view(observer: Point3) -> Isometry3 {
    let mut target = observer.clone();
    target.z = Z_CANVAS;
    Isometry3::look_at_rh(&observer, &target, &Vector3::y())
}

pub fn perspective(width: u32, height: u32) -> Perspective3 {
    let aspect_ratio = width as f32 / height as f32;
    Perspective3::new(aspect_ratio, 3.14 / 3.0, 0.1, 1000.0)
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
