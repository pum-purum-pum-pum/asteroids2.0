use super::*;

pub enum ParticlesData {
    MovementParticles(MovementParticles),
    Explosion(Explosion),
    Engine(Engine),
}

pub struct Explosion {
    pub instancing_data: InstancingData,
    pub velocities: Vec<Vector2>,
    pub lifetime: Option<usize>,
    time: usize,
}

impl Explosion {
    pub fn new(
        gl: &red::GL,
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
            .map(|pos| GeometryVertex { position: red::data::f32_f32::new(pos[0], pos[1]) })
            .collect();
        let vertex_buffer = GeometryVertexBuffer::new(gl, &shape).unwrap();
        let index_buffer = red::buffer::IndexBuffer::new(gl, &[0u16, 1, 2, 2, 3, 0]).unwrap();
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
                vertex_buffer: vertex_buffer,
                indices: index_buffer,
                per_instance: per_instance,
            },
            velocities: velocities,
            lifetime: lifetime,
            time: 0,
        }
    }

    pub fn update(&mut self) -> bool {
        self.time += 1;
        let instanced = self.instancing_data.per_instance.map_array().unwrap();
        for (particle, vel) in instanced
            .slice
            .iter_mut()
            .zip(self.velocities.iter())
        {
            particle.world_position.0 += vel.x;
            particle.world_position.1 += vel.y;
        }
        if self.lifetime.is_some() {
            return self.time <= self.lifetime.unwrap();
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
            .map(|pos| GeometryVertex { position: red::data::f32_f32::new(pos[0], pos[1]) })
            .collect();
        let vertex_buffer = GeometryVertexBuffer::new(gl, &shape).unwrap();
        let index_buffer = red::buffer::IndexBuffer::new(gl, &[0u16, 1, 2, 2, 3, 0]).unwrap();
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
        let world_vertex_buffer = WorldVertexBuffer::new(gl, &quad_positions).unwrap();
        MovementParticles {
            instancing_data: InstancingData {
                vertex_buffer: vertex_buffer,
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
            let cut_hight = |x, min, max| if x > max { min + x - max } else { x };
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



pub struct Engine {
    pub instancing_data: InstancingData,
    pub velocities: Vec<Vector2>,
    pub start_moving: Vec<usize>,
    pub lifetime: Option<usize>,
    time: usize,
}

impl Engine {
    pub fn new(
        gl: &red::GL,
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
            .map(|pos| GeometryVertex { position: red::data::f32_f32::new(pos[0], pos[1]) })
            .collect();
        let vertex_buffer = GeometryVertexBuffer::new(gl, &shape).unwrap();
        let index_buffer = red::buffer::IndexBuffer::new(gl, &[0u16, 1, 2, 2, 3, 0]).unwrap();
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
                world_position: red::data::f32_f32_f32::new(x, y, z),
            });
            let angle = rng.gen_range(0f32, 2.0 * std::f32::consts::PI);
            let vel_x = rng.gen_range(0.05, 0.2) * f32::cos(angle);
            let vel_y = rng.gen_range(0.05, 0.2) * f32::sin(angle);
            velocities.push(Vector2::new(vel_x, vel_y));
            start_moving.push(rng.gen_range(1usize, 10usize));
        }
        let world_vertex_buffer = WorldVertexBuffer::new(gl, &quad_positions).unwrap();
        Engine {
            instancing_data: InstancingData {
                vertex_buffer: vertex_buffer,
                indices: index_buffer,
                per_instance: world_vertex_buffer,
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
        _ship_velocity: Vector2,
        ship_direction: Vector2,
    ) -> bool {
        let instanced = self.instancing_data.per_instance.map_array().unwrap();
        self.time += 1;
        for ((particle, vel), &start_time) in instanced.slice
            .iter_mut()
            .zip(self.velocities.iter())
            .zip(self.start_moving.iter())
        {
            if self.time < start_time {continue};
            let particle_position = Vector2::new(particle.world_position.0, particle.world_position.1);
            let distance = (particle_position - ship_position).norm();
            if distance > ENGINE_FAR {
                particle.world_position.0 = ship_position.x;
                particle.world_position.1 = ship_position.y;
            };
            particle.world_position.0 += -0.1 * ship_direction.x + 0.6 * vel.x;
            particle.world_position.1 += -0.1 * ship_direction.y + 0.6 * vel.y;
        }
        if self.lifetime.is_some() {
            return self.time <= self.lifetime.unwrap();
        };
        true
    }
}