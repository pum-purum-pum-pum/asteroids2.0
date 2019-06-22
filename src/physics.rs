use astro_lib as al;
use al::prelude::*;
use nphysics2d::world::World;
use nphysics2d::object::{BodyHandle, ColliderHandle, BodyStatus, RigidBodyDesc, ColliderDesc};
use nphysics2d::volumetric::volumetric::Volumetric;
use ncollide2d::shape::{ShapeHandle};
use ncollide2d::world::CollisionGroups;
use derive_deref::{Deref, DerefMut};
use specs::Component;

pub const PHYSICS_SIMULATION_TIME: f32 = 1.0;

#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(usize)]
pub enum CollisionId {
    Asteroid,
    PlayerShip,
    EnemyShip,
    PlayerBullet,
    EnemyBullet,
}

#[derive(Deref, DerefMut, Default)]
pub struct BodiesMap(fnv::FnvHashMap<BodyHandle, specs::Entity>);

impl BodiesMap {
    pub fn new() -> Self {
        BodiesMap(fnv::FnvHashMap::default())
    }
}

pub fn safe_maintain(world: &mut specs::World) {
    world.maintain();
    let mut physic_world = world.write_resource::<World<f32>>();
    let mut bodies_map = world.write_resource::<BodiesMap>();

    let retained = world
        .write_storage::<PhysicsComponent>()
        .retained()
        .iter()
        .map(|r| r.body_handle)
        .collect::<Vec<_>>();
    physic_world.remove_bodies(&retained);
    for handle in &retained {
        bodies_map.remove(handle);
    }
}

#[derive(Debug, Clone)]
pub struct PhysicsComponent {
    pub body_handle: BodyHandle,
    pub collider_handle: ColliderHandle,
}

impl Component for PhysicsComponent {
    type Storage = RetainedStorage<Self, specs::VecStorage<Self>>;
}

impl PhysicsComponent {
    pub fn new(body_handle: BodyHandle, collider_handle: ColliderHandle) -> Self {
        PhysicsComponent {
            body_handle: body_handle,
            collider_handle: collider_handle,
        }
    }
    pub fn safe_insert<'a>(
            storage: &mut specs::WriteStorage<'a, PhysicsComponent>,
            entity: specs::Entity,
            shape: ShapeHandle<f32>,
            default_position: Isometry2,
            body_status: BodyStatus,
            physics_world: &mut World<f32>,
            bodies_map: &mut BodiesMap,
            collision_groups: CollisionGroups,
            inertia: f32
    ) -> Self {
        let inertia = shape.inertia(inertia);
        let center_of_mass = shape.center_of_mass();
        let (body_handle, body_part_handle) = {
            let rigid_body = RigidBodyDesc::new()
                .position(default_position)
                .local_inertia(inertia)
                .local_center_of_mass(center_of_mass)
                .status(body_status)
                .build(physics_world);
            (rigid_body.handle(), rigid_body.part_handle())
        };

        let collider_desc =  ColliderDesc::new(shape);
        let collider_handle = collider_desc
            .build_with_parent(body_part_handle, physics_world)
            .unwrap()
            .handle();
        let collision_world = physics_world.collider_world_mut();
        collision_world.set_collision_groups(collider_handle, collision_groups);
        let component = PhysicsComponent::new(body_handle, collider_handle);
        storage.insert(
            entity,
            component.clone(),
        ).unwrap(); // TODO RESULT
        bodies_map.insert(body_handle, entity);
        component
    }
}