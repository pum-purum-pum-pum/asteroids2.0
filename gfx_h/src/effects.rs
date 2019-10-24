// we just allocate new objects instead of GPU streaming or preallocating.
// It's suboptimal but works ok for now.
use super::*;
use std::time::{Duration, Instant};

pub enum ParticlesData {
    MovementParticles(MovementParticles),
    Explosion(Explosion),
    MenuParticles(MenuParticles),
}

pub struct Explosion {
    pub instancing_data: InstancingData,
    pub velocities: Vec<Vector2>,
    pub lifetime: Option<Duration>,
    start_time: Instant,
}

impl Explosion {
    pub fn new(
        gl: &red::GL,
        position: Point2,
        num: usize,
        lifetime: Option<Duration>,
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
            .map(|pos| GeometryVertex {
                position: red::data::f32_f32::new(pos[0], pos[1]),
            })
            .collect();
        let vertex_buffer = GeometryVertexBuffer::new(gl, &shape).unwrap();
        let index_buffer =
            red::buffer::IndexBuffer::new(gl, &[0u16, 1, 2, 2, 3, 0]).unwrap();
        let mut rng = thread_rng();
        let mut quad_positions = vec![];
        let mut velocities = vec![];
        for _ in 0..num {
            let x = position.x;
            let y = position.y;
            let depth = 1f32;
            let z = rng.gen_range(-depth, depth);
            quad_positions.push(WorldVertex {
                world_position: red::data::f32_f32_f32(x, y, z),
            });
            let angle = rng.gen_range(0f32, 2.0 * std::f32::consts::PI);
            let vel_x = rng.gen_range(0.05, 0.2) * f32::cos(angle);
            let vel_y = rng.gen_range(0.05, 0.2) * f32::sin(angle);
            velocities.push(Vector2::new(vel_x, vel_y));
        }
        let per_instance = WorldVertexBuffer::new(gl, &quad_positions).unwrap();
        Explosion {
            instancing_data: InstancingData {
                vertex_buffer,
                indices: index_buffer,
                per_instance,
            },
            velocities,
            lifetime,
            start_time: Instant::now(),
        }
    }

    pub fn update(&mut self) -> bool {
        let instanced = self.instancing_data.per_instance.map_array().unwrap();
        for (particle, vel) in
            instanced.slice.iter_mut().zip(self.velocities.iter())
        {
            particle.world_position.0 += vel.x;
            particle.world_position.1 += vel.y;
        }
        if let Some(lifetime) = self.lifetime {
            return Instant::now() - self.start_time <= lifetime;
        };
        true
    }
}

pub struct MovementParticles {
    pub instancing_data: InstancingData,
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
}

impl MovementParticles {
    pub fn new_quad(
        gl: &red::GL,
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
            .map(|pos| GeometryVertex {
                position: red::data::f32_f32::new(pos[0], pos[1]),
            })
            .collect();
        let vertex_buffer = GeometryVertexBuffer::new(gl, &shape).unwrap();
        let index_buffer =
            red::buffer::IndexBuffer::new(gl, &[0u16, 1, 2, 2, 3, 0]).unwrap();
        let mut rng = thread_rng();
        let mut quad_positions = vec![];
        for _ in 0..num {
            let x = rng.gen_range(x_min, x_max);
            let y = rng.gen_range(y_min, y_max);
            let depth = 20f32;
            let z = rng.gen_range(-depth, 4f32);
            quad_positions.push(WorldVertex {
                world_position: red::data::f32_f32_f32::new(x, y, z),
            });
        }
        let world_vertex_buffer =
            WorldVertexBuffer::new(gl, &quad_positions).unwrap();
        MovementParticles {
            instancing_data: InstancingData {
                vertex_buffer,
                indices: index_buffer,
                per_instance: world_vertex_buffer,
            },
            x_min,
            y_min,
            x_max,
            y_max,
        }
    }

    pub fn update(&mut self, vel: Vector2) {
        let instanced = self.instancing_data.per_instance.map_array().unwrap();
        for particle in instanced.slice.iter_mut() {
            particle.world_position.0 += vel.x;
            particle.world_position.1 += vel.y;
            let cut_low = |x, min, max| if x < min { max - min + x } else { x };
            let cut_hight =
                |x, min, max| if x > max { min + x - max } else { x };
            particle.world_position.0 = cut_low(
                cut_hight(particle.world_position.0, self.x_min, self.x_max),
                self.x_min,
                self.x_max,
            );
            particle.world_position.1 = cut_low(
                cut_hight(particle.world_position.1, self.y_min, self.y_max),
                self.y_min,
                self.y_max,
            );
        }
    }
}

pub struct TraceImage {}

pub struct MenuParticles {
    pub instancing_data: InstancingData,
    pub z_min: f32,
    pub z_max: f32,
}

impl MenuParticles {
    pub fn new_quad(
        gl: &red::GL,
        x_range: (f32, f32),
        y_range: (f32, f32),
        z_range: (f32, f32),
        num: usize,
    ) -> Self {
        let (x_min, x_max) = x_range;
        let (y_min, y_max) = y_range;
        let (z_min, z_max) = z_range;
        let scale = 0.03f32;
        let positions = vec![
            [-scale, -scale],
            [-scale, scale],
            [scale, scale],
            [scale, -scale],
        ];
        let shape: Vec<GeometryVertex> = positions
            .into_iter()
            .map(|pos| GeometryVertex {
                position: red::data::f32_f32::new(pos[0], pos[1]),
            })
            .collect();
        let vertex_buffer = GeometryVertexBuffer::new(gl, &shape).unwrap();
        let index_buffer =
            red::buffer::IndexBuffer::new(gl, &[0u16, 1, 2, 2, 3, 0]).unwrap();
        let mut rng = thread_rng();
        let mut quad_positions = vec![];
        for _ in 0..num {
            let x = rng.gen_range(x_min, x_max);
            let y = rng.gen_range(y_min, y_max);
            let z = rng.gen_range(z_min, z_max);
            quad_positions.push(WorldVertex {
                world_position: red::data::f32_f32_f32::new(x, y, z),
            });
        }
        let world_vertex_buffer =
            WorldVertexBuffer::new(gl, &quad_positions).unwrap();
        Self {
            instancing_data: InstancingData {
                vertex_buffer,
                indices: index_buffer,
                per_instance: world_vertex_buffer,
            },
            z_min,
            z_max,
        }
    }

    pub fn update(&mut self, z_vel: f32) {
        let instanced = self.instancing_data.per_instance.map_array().unwrap();
        for particle in instanced.slice.iter_mut() {
            particle.world_position.2 += z_vel;
            // particle.world_position.0 += vel.x;
            // particle.world_position.1 += vel.y;

            let cut_low = |x, min, max| if x < min { max - min + x } else { x };
            let cut_hight =
                |x, min, max| if x > max { min + x - max } else { x };
            particle.world_position.2 = cut_low(
                cut_hight(particle.world_position.2, self.z_min, self.z_max),
                self.z_min,
                self.z_max,
            );
        }
    }
}
