use crate::types::{*};
use rand::prelude::*;
use std::fs::File;
use std::io::{BufReader, Error as IOError};
use nalgebra::geometry::Orthographic3;

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
const MAX_ADD_SPEED_Z: f32 = 10f32;
const SPEED_EMA: f32 = 0.04f32; // new value will be taken with with that coef
pub const _BACKGROUND_SIZE: f32 = 20f32;
const ENGINE_FAR: f32 = 3f32;

pub struct TextData {
    pub text_system: glium_text::TextSystem,
    pub font: glium_text::FontTexture
}

impl TextData {
    pub fn new(display: &SDL2Facade, font: glium_text::FontTexture) -> TextData {
        TextData {
            text_system: glium_text::TextSystem::new(display),
            font: font
        }
    }
}

pub enum ParticlesData {
    MovementParticles(MovementParticles),
    Explosion(Explosion),
    Engine(Engine),
}

impl ParticlesData {
    pub fn _gfx(&self) -> &InstancingData {
        match self {
            ParticlesData::MovementParticles(particles) => &particles.gfx,
            ParticlesData::Explosion(particles) => &particles.gfx,
            ParticlesData::Engine(particles) => &particles.gfx
        }
    }
}

pub struct InstancingData {
    pub vertex_buffer: glium::VertexBuffer<GeometryVertex>,
    pub indices: glium::IndexBuffer<u16>,
    pub per_instance: glium::VertexBuffer<WorldVertex>,
}

/// white star like particles
pub struct MovementParticles {
    pub gfx: InstancingData,
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
}

pub struct Engine {
    pub gfx: InstancingData,
    pub velocities: Vec<Vector2>,
    pub start_moving: Vec<usize>,
    pub lifetime: Option<usize>,
    time: usize,
}

impl Engine {
    pub fn new(
        display: &SDL2Facade,
        position: Point2,
        num: usize,
        lifetime: Option<usize>,
    ) -> Self {
        let scale = 0.07f32;
        let positions = vec![
            [-scale, -scale],
            [-scale, scale],
            [scale, scale],
            [scale, -scale],
        ];
        let shape: Vec<GeometryVertex> = positions
            .into_iter()
            .map(|pos| GeometryVertex { position: pos })
            .collect();
        let vertex_buffer = glium::VertexBuffer::new(display, &shape).unwrap();
        let indices = glium::IndexBuffer::new(
            display,
            PrimitiveType::TrianglesList,
            &[0u16, 1, 2, 2, 3, 0],
        )
        .unwrap();
        let mut rng = thread_rng();
        let mut quad_positions = vec![];
        let mut velocities = vec![];
        let mut start_moving = vec![];
        for _ in 0..num {
            let x = position.x;
            let y = position.y;
            let depth = 1f32;
            let z = rng.gen_range(-depth, depth);
            quad_positions.push(WorldVertex {
                world_position: [x, y, z],
            });
            let angle = rng.gen_range(0f32, 2.0 * std::f32::consts::PI);
            let vel_x = rng.gen_range(0.05, 0.2) * f32::cos(angle);
            let vel_y = rng.gen_range(0.05, 0.2) * f32::sin(angle);
            velocities.push(Vector2::new(vel_x, vel_y));
            start_moving.push(rng.gen_range(1usize, 10usize));
        }
        let per_instance = glium::VertexBuffer::new(display, &quad_positions).unwrap();
        Engine {
            gfx: InstancingData {
                vertex_buffer: vertex_buffer,
                indices: indices,
                per_instance: per_instance,
            },
            velocities: velocities,
            start_moving: start_moving,
            lifetime: lifetime,
            time: 0,
        }
    }

    pub fn update(
        &mut self, 
        ship_position: Vector2,
        ship_velocity: Vector2,
        ship_direction: Vector2,
    ) -> bool {
        self.time += 1;
        for ((particle, vel), &start_time) in self
            .gfx
            .per_instance
            .map()
            .iter_mut()
            .zip(self.velocities.iter())
            .zip(self.start_moving.iter())
        {
            if self.time < start_time {continue};
            let particle_position = Vector2::new(particle.world_position[0], particle.world_position[1]);
            let distance = (particle_position - ship_position).norm();
            if distance > ENGINE_FAR {
                particle.world_position[0] = ship_position.x;
                particle.world_position[1] = ship_position.y;
            };
            particle.world_position[0] += -0.1 * ship_direction.x + 0.6 * vel.x;
            particle.world_position[1] += -0.1 * ship_direction.y + 0.6 * vel.y;
        }
        if self.lifetime.is_some() {
            return self.time <= self.lifetime.unwrap();
        };
        true
    }
}

pub struct Explosion {
    pub gfx: InstancingData,
    pub velocities: Vec<Vector2>,
    pub lifetime: Option<usize>,
    time: usize,
}

impl Explosion {
    pub fn new(
        display: &SDL2Facade,
        position: Point2,
        num: usize,
        lifetime: Option<usize>,
    ) -> Self {
        let scale = 0.07f32;
        let positions = vec![
            [-scale, -scale],
            [-scale, scale],
            [scale, scale],
            [scale, -scale],
        ];
        let shape: Vec<GeometryVertex> = positions
            .into_iter()
            .map(|pos| GeometryVertex { position: pos })
            .collect();
        let vertex_buffer = glium::VertexBuffer::new(display, &shape).unwrap();
        let indices = glium::IndexBuffer::new(
            display,
            PrimitiveType::TrianglesList,
            &[0u16, 1, 2, 2, 3, 0],
        )
        .unwrap();
        let mut rng = thread_rng();
        let mut quad_positions = vec![];
        let mut velocities = vec![];
        for _ in 0..num {
            let x = position.x;
            let y = position.y;
            let depth = 1f32;
            let z = rng.gen_range(-depth, depth);
            quad_positions.push(WorldVertex {
                world_position: [x, y, z],
            });
            let angle = rng.gen_range(0f32, 2.0 * std::f32::consts::PI);
            let vel_x = rng.gen_range(0.05, 0.2) * f32::cos(angle);
            let vel_y = rng.gen_range(0.05, 0.2) * f32::sin(angle);
            velocities.push(Vector2::new(vel_x, vel_y));
        }
        let per_instance = glium::VertexBuffer::new(display, &quad_positions).unwrap();
        Explosion {
            gfx: InstancingData {
                vertex_buffer: vertex_buffer,
                indices: indices,
                per_instance: per_instance,
            },
            velocities: velocities,
            lifetime: lifetime,
            time: 0,
        }
    }

    pub fn update(&mut self) -> bool {
        self.time += 1;
        for (particle, vel) in self
            .gfx
            .per_instance
            .map()
            .iter_mut()
            .zip(self.velocities.iter())
        {
            particle.world_position[0] += vel.x;
            particle.world_position[1] += vel.y;
        }
        if self.lifetime.is_some() {
            return self.time <= self.lifetime.unwrap();
        };
        true
    }
}

impl MovementParticles {
    pub fn new_quad(
        display: &SDL2Facade,
        x_min: f32,
        y_min: f32,
        x_max: f32,
        y_max: f32,
        num: usize,
    ) -> Self {
        let scale = 0.03f32;
        let positions = vec![
            [-scale, -scale],
            [-scale, scale],
            [scale, scale],
            [scale, -scale],
        ];
        let shape: Vec<GeometryVertex> = positions
            .into_iter()
            .map(|pos| GeometryVertex { position: pos })
            .collect();
        let vertex_buffer = glium::VertexBuffer::new(display, &shape).unwrap();
        let indices = glium::IndexBuffer::new(
            display,
            PrimitiveType::TrianglesList,
            &[0u16, 1, 2, 2, 3, 0],
        )
        .unwrap();
        let mut rng = thread_rng();
        let mut quad_positions = vec![];
        for _ in 0..num {
            let x = rng.gen_range(x_min, x_max);
            let y = rng.gen_range(y_min, y_max);
            let depth = 10f32;
            let z = rng.gen_range(-depth, depth);
            quad_positions.push(WorldVertex {
                world_position: [x, y, z],
            });
        }
        let per_instance = glium::VertexBuffer::new(display, &quad_positions).unwrap();
        MovementParticles {
            gfx: InstancingData {
                vertex_buffer: vertex_buffer,
                indices: indices,
                per_instance: per_instance,
            },
            x_min,
            y_min,
            x_max,
            y_max,
        }
    }

    pub fn update(&mut self, vel: Vector2) {
        for particle in self.gfx.per_instance.map().iter_mut() {
            particle.world_position[0] += vel.x;
            particle.world_position[1] += vel.y;
            let cut_low = |x, min, max| if x < min { max - min + x } else { x };
            let cut_hight = |x, min, max| if x > max { min + x - max } else { x };
            particle.world_position[0] = cut_low(
                cut_hight(particle.world_position[0], self.x_min, self.x_max),
                self.x_min,
                self.x_max,
            );
            particle.world_position[1] = cut_low(
                cut_hight(particle.world_position[1], self.y_min, self.y_max),
                self.y_min,
                self.y_max,
            );
        }
    }
}

#[derive(Copy, Clone)]
pub struct GeometryVertex {
    pub position: [f32; 2],
}

#[derive(Copy, Clone)]
pub struct WorldVertex {
    pub world_position: [f32; 3],
}

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
}
implement_vertex!(Vertex, position, tex_coords);
implement_vertex!(GeometryVertex, position);
implement_vertex!(WorldVertex, world_position);

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
}

impl ImageData {
    /// panic if failed to create buffers. TODO Result
    /// image_name - is name of the image to load in assets directory
    pub fn new(display: &SDL2Facade, image_name: &str) -> Result<Self, LoadTextureError> {
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
        })
    }
}

/// 2D graphics on screen
pub struct Canvas<'a> {
    program: glium::Program,       // @vlad TODO: we want to use many programs
    program_light: glium::Program, // but for now simpler=better
    program_instancing: glium::Program,
    program_primitive: glium::Program,
    program_primitive_texture: glium::Program,
    observer: Point3,
    default_params: glium::DrawParameters<'a>,
    stencil_check_params: glium::DrawParameters<'a>,
    stencil_write_params: glium::DrawParameters<'a>,
}

impl<'a> Canvas<'a> {
    pub fn new(display: &SDL2Facade) -> Self {
        let vertex_shader_primitive_texture_src = r#"
            #version 130
            in vec2 position;
            in vec2 tex_coords;
            uniform mat4 projection;
            uniform mat4 view;
            uniform mat4 model;
            uniform float size;
            out vec2 v_tex_coords;

            void main() {
                v_tex_coords = tex_coords;
                gl_Position = projection * view * model * vec4(size * (position + 1.0) / 2.0, -1.0, 1.0);
            }
        "#;
 
        let fragment_shader_primitive_texture_src = r#"
            #version 130
            out vec4 color;
            uniform sampler2D tex;
            in vec2 v_tex_coords;

            void main() {
                vec4 texture_colors = vec4(texture(tex, v_tex_coords));
                color = texture_colors;
            }
        "#;
        
        let vertex_shader_primitive_src = r#"
            #version 130
            in vec2 position;
            uniform mat4 projection;
            uniform mat4 view;
            uniform mat4 model;
            
            void main() {
                gl_Position = projection * view * model * vec4(position, -1.0, 1.0);
            }
        "#;

        let fragment_shader_primitive_src = r#"
            #version 130
            out vec4 color;
            uniform vec3 fill_color;

            void main() {
                color = vec4(fill_color, 1.0);
            }
        "#;

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

        let vertex_shader_instancing = r#"
            #version 130
            in vec2 position;
            in vec3 world_position;

            uniform mat4 perspective;
            uniform mat4 view;
            uniform mat4 model;
            
            vec3 position_moved;

            void main() {
                position_moved = world_position + vec3(position, 0.0);
                gl_Position = perspective * view * model * vec4(position_moved, 1.0);
            }
        "#;

        let fragment_shader_instancing = r#"
            #version 130
            out vec4 color;
            uniform float transparency;
            float alpha = 0.5;

            void main() {
                color =  vec4(1.0, 1.0, 1.0, alpha + (1.0 - alpha) * transparency);
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
        let default_params = glium::DrawParameters {
                blend: Blend::alpha_blending(),
                ..Default::default()
        };
        let stencil_check_params = glium::DrawParameters {
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
        let stencil_write_params = glium::DrawParameters {
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

        let program =
            glium::Program::from_source(display, vertex_shader_src, fragment_shader_src, None)
                .unwrap();
        let program_primitive = 
            glium::Program::from_source(display, vertex_shader_primitive_src, fragment_shader_primitive_src, None)
                .unwrap();
        let program_primitive_texture = 
            glium::Program::from_source(display, vertex_shader_primitive_texture_src, fragment_shader_primitive_texture_src, None)
                .unwrap();
        let program_light = glium::Program::from_source(
            display,
            vertex_light_shader_src,
            fragment_light_shader_src,
            None,
        )
        .unwrap();
        let program_instancing = glium::Program::from_source(
            display,
            vertex_shader_instancing,
            fragment_shader_instancing,
            None,
        )
        .unwrap();
        Canvas {
            program: program,
            program_primitive: program_primitive,
            program_primitive_texture: program_primitive_texture,
            program_light: program_light,
            program_instancing: program_instancing,
            observer: Point3::new(0f32, 0f32, Z_FAR),
            default_params: default_params,
            stencil_check_params: stencil_check_params,
            stencil_write_params: stencil_write_params,
        }
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

    pub fn render(
        &self,
        display: &SDL2Facade,
        target: &mut glium::Frame,
        image_data: &ImageData,
        model: &Isometry3,
        scale: f32,
        with_lights: bool,
    ) -> Result<(), DrawError> {
        let model: [[f32; 4]; 4] = model.to_homogeneous().into();
        let dims = display.get_framebuffer_dimensions();
        let processed_texture = image_data
            .texture
            .sampled()
            .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
            .minify_filter(glium::uniforms::MinifySamplerFilter::Linear);
        let draw_params = if with_lights {
            &self.stencil_check_params
        } else {
            &self.default_params
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
                scale: scale,
            },
            draw_params,
        )
    }

    pub fn render_primitive_texture(
        &self,
        display: &SDL2Facade,
        target: &mut glium::Frame,
        image_data: &ImageData,
        model: &Isometry3,
        with_projection: bool,
        size: f32,
    ) -> Result<(), DrawError> {
        let model: [[f32; 4]; 4] = model.to_homogeneous().into();
        let dims = display.get_framebuffer_dimensions();
        let (projection, view) = if with_projection {
            let perspective: [[f32; 4]; 4] = perspective(dims.0, dims.1).to_homogeneous().into();
            let view: [[f32; 4]; 4] = get_view(self.observer).to_homogeneous().into();
            (perspective, view)
        } else {
            let orthographic: [[f32; 4]; 4] = orthographic(dims.0, dims.1).to_homogeneous().into();
            let view: [[f32; 4]; 4] = Matrix4::identity().into();
            (orthographic, view)
        };
        let texture = image_data
            .texture
            .sampled()
            .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
            .minify_filter(glium::uniforms::MinifySamplerFilter::Linear);
        target.draw(
            &image_data.positions,
            &image_data.indices,
            &self.program_primitive_texture,
            &uniform! {
                tex: texture,
                projection: projection,
                view: view,
                model: model,
                size: size
            },
            &self.default_params,
        )
    }


    pub fn render_primitive(
        &self,
        display: &SDL2Facade,
        target: &mut glium::Frame,
        geometry_data: &GeometryData,
        model: &Isometry3,
        fill_color: Point3,
        with_projection: bool,
    ) -> Result<(), DrawError> {
        let model: [[f32; 4]; 4] = model.to_homogeneous().into();
        let dims = display.get_framebuffer_dimensions();
        let (projection, view) = if with_projection {
            let perspective: [[f32; 4]; 4] = perspective(dims.0, dims.1).to_homogeneous().into();
            let view: [[f32; 4]; 4] = get_view(self.observer).to_homogeneous().into();
            (perspective, view)
        } else {
            let orthographic: [[f32; 4]; 4] = orthographic(dims.0, dims.1).to_homogeneous().into();
            let view: [[f32; 4]; 4] = Matrix4::identity().into();
            (orthographic, view)
        };
        target.draw(
            &geometry_data.positions,
            &geometry_data.indices,
            &self.program_primitive,
            &uniform! {
                projection: projection,
                view: view,
                model: model,
                fill_color: [fill_color.x, fill_color.y, fill_color.z]
            },
            &self.default_params,
        )
    }

    pub fn render_geometry(
        &self,
        display: &SDL2Facade,
        target: &mut glium::Frame,
        geometry_data: &GeometryData,
        model: &Isometry3,
        stencil: bool,
    ) -> Result<(), DrawError> {
        let model: [[f32; 4]; 4] = model.to_homogeneous().into();
        let dims = display.get_framebuffer_dimensions();
        let draw_params = if stencil {
            &self.stencil_write_params
        } else {
            &self.default_params
        };
        let perspective: [[f32; 4]; 4] = perspective(dims.0, dims.1).to_homogeneous().into();
        let view: [[f32; 4]; 4] = get_view(self.observer).to_homogeneous().into();
        target.draw(
            &geometry_data.positions,
            &geometry_data.indices,
            &self.program_light,
            &uniform! {
                model: model,
                view: view,
                perspective: perspective,
            },
            &draw_params,
        )
    }

    pub fn render_particles(
        &self,
        display: &SDL2Facade,
        target: &mut glium::Frame,
        instancing_data: &InstancingData,
        model: &Isometry3,
        transparency: f32, // TODO make params structure
    ) -> Result<(), DrawError> {
        let model: [[f32; 4]; 4] = model.to_homogeneous().into();
        let dims = display.get_framebuffer_dimensions();
        let perspective: [[f32; 4]; 4] = perspective(dims.0, dims.1).to_homogeneous().into();
        let view: [[f32; 4]; 4] = get_view(self.observer).to_homogeneous().into();
        target.draw(
            (
                &instancing_data.vertex_buffer,
                instancing_data.per_instance.per_instance().unwrap(),
            ),
            &instancing_data.indices,
            &self.program_instancing,
            &uniform! {
                model: model,
                view: view,
                perspective: perspective,
                transparency: transparency
            },
            &self.default_params,
        )
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
