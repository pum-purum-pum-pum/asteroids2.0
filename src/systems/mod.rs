use std::mem::swap;
use std::thread;
use std::sync::{Arc, Mutex};

use common::*;
use rand::prelude::*;
use std::time::{Duration, Instant};

use ncollide2d::query::Ray;
use ncollide2d::shape::ShapeHandle;
use ncollide2d::world::CollisionGroups;
use ncollide2d::world::CollisionObjectHandle;
use nphysics2d::object::{Body, BodyHandle, BodyStatus};
use nphysics2d::world::World;

use shrev::EventChannel;
use specs::prelude::*;
use specs::Join;

use crate::gui::{Primitive, PrimitiveKind, Text, UI};
use components::*;
use geometry::{generate_convex_polygon, Polygon, TriangulateFromCenter, EPS};
use gfx_h::{iso3_iso2, Explosion, GeometryData, ParticlesData};
use physics::CollisionId;
use sound::{MusicData, PreloadedSounds, SoundData, EFFECT_MAX_VOLUME};

mod ai;
mod collision;
mod common_respawn;
mod control;
mod deadscreen;
mod gameplay;
mod gui_system;
mod insert;
mod kinematic;
mod menu_rendering_system;
mod rendering;
mod score_table;
mod sound_system;
mod upgrade_ui;
mod destroy_sync;

pub use ai::*;
pub use collision::*;
pub use common_respawn::*;
pub use control::*;
pub use deadscreen::*;
pub use gameplay::*;
pub use gui_system::*;
pub use insert::*;
pub use kinematic::*;
pub use menu_rendering_system::*;
pub use physics_system::*;
pub use rendering::*;
pub use score_table::*;
pub use sound_system::*;
pub use upgrade_ui::*;
pub use destroy_sync::*;

const DAMPING_FACTOR: f32 = 0.98f32;
const VELOCITY_MAX: f32 = 1f32;
const SCREEN_AREA: f32 = 10f32;
// it's a kludge -- TODO redo with camera and screen sizes

// we will spwan new objects in ACTIVE_AREA but not in PLAYER_AREA
const PLAYER_AREA: f32 = 20f32;
const ACTIVE_AREA: f32 = 40f32;
// the same for NEBULAS
const ASTEROIDS_MIN_NUMBER: usize = 100;
const ASTEROID_MAX_RADIUS: f32 = 4.2f32;
const ASTEROID_MIN_RADIUS: f32 = 0.5;
const ASTEROID_INERTIA: f32 = 2f32;

const EXPLOSION_WOBBLE: f32 = 0.4;
const DAMAGED_RED: f32 = 0.2;

const MAGNETO_RADIUS: f32 = 4f32;
const COLLECT_RADIUS: f32 = 0.2;
const COIN_LIFETIME_SECS: u64 = 5;
const EXPLOSION_LIFETIME_SECS: u64 = 1;
const BLAST_LIFETIME_SECS: u64 = 1;
const BULLET_CONTACT_LIFETIME_SECS: u64 = 1;
const COLLECTABLE_SIDE_BULLET: u64 = 5;
const SIDE_BULLET_LIFETIME_SEC: u64 = 6;
const DOUBLE_COINS_LIFETIME_SEC: u64 = 5;
const COLLECTABLE_DOUBLE_COINS_SEC: u64 = 5;
const DESTUCTION_SITES: usize = 20;

pub fn initial_asteroid_velocity() -> Velocity2 {
    let mut rng = thread_rng();
    let rotation = rng.gen_range(-1E-1, 1E-1);
    let linear_velocity =
        Vector2::new(rng.gen_range(-1E-1, 1E-1), rng.gen_range(-1E-1, 1E-1));
    Velocity2::new(linear_velocity, rotation)
}

pub fn spawn_position(char_pos: Point2, forbidden: f32, active: f32) -> Point2 {
    assert!(forbidden < active);
    let mut rng = thread_rng();
    loop {
        let x = rng.gen_range(-active, active);
        let y = rng.gen_range(-active, active);
        if x.abs() >= forbidden || y.abs() >= forbidden {
            return Point2::new(char_pos.x + x, char_pos.y + y);
        }
    }
}

pub fn spawn_in_rectangle(
    min_w: f32,
    max_w: f32,
    min_h: f32,
    max_h: f32,
) -> Point2 {
    let mut rng = thread_rng();
    let x = rng.gen_range(min_w, max_w);
    let y = rng.gen_range(min_h, max_h);
    Point2::new(x, y)
}

pub fn is_active(
    character_position: Point2,
    point: Point2,
    active_area: f32,
) -> bool {
    (point.x - character_position.x).abs() < active_area
        && (point.y - character_position.y).abs() < active_area
}

fn get_collision_groups(kind: EntityType) -> CollisionGroups {
    match kind {
        EntityType::Player => {
            let mut player_bullet_collision_groups = CollisionGroups::new();
            player_bullet_collision_groups
                .set_membership(&[CollisionId::PlayerBullet as usize]);
            player_bullet_collision_groups.set_whitelist(&[
                CollisionId::Asteroid as usize,
                CollisionId::EnemyShip as usize,
            ]);
            player_bullet_collision_groups
                .set_blacklist(&[CollisionId::PlayerShip as usize]);
            player_bullet_collision_groups
        }
        EntityType::Enemy => {
            let mut enemy_bullet_collision_groups = CollisionGroups::new();
            enemy_bullet_collision_groups
                .set_membership(&[CollisionId::EnemyBullet as usize]);
            enemy_bullet_collision_groups.set_whitelist(&[
                CollisionId::Asteroid as usize,
                CollisionId::PlayerShip as usize,
            ]);
            enemy_bullet_collision_groups
                .set_blacklist(&[CollisionId::EnemyShip as usize]);
            enemy_bullet_collision_groups
        }
    }
}

pub fn calculate_shards(

) {

}

pub fn spawn_asteroids(
    isometry: Isometry3,
    polygon: Polygon,
    mut insert_channel: Arc<Mutex<EventChannel<InsertEvent>>>,
    bullet_position: Option<Point2>,
) {
    flame::start("asteroids");
    let position = isometry.translation.vector;
    let new_polygons = if let Some(bullet_position) = bullet_position {
        polygon.deconstruct(
            bullet_position - Vector2::new(position.x, position.y),
            DESTUCTION_SITES,
        )
    } else {
        polygon.deconstruct(polygon.center(), DESTUCTION_SITES)
    };
    let mut rng = thread_rng();
    if new_polygons.len() > 1 {
        for poly in new_polygons.iter() {
            let insert_event = InsertEvent::Asteroid {
                iso: Point3::new(
                    position.x,
                    position.y,
                    isometry.rotation.euler_angles().2,
                ),
                velocity: initial_asteroid_velocity(),
                polygon: poly.clone(),
                spin: rng.gen_range(-1E-2, 1E-2),
            };
            insert_channel.lock().unwrap().single_write(insert_event);
        }
    } else {
        // spawn coins and stuff
        let spawn_position = Point2::new(position.x, position.y);
        if rng.gen_range(0.0, 1.0) < 0.1 {
            insert_channel.lock().unwrap().single_write(InsertEvent::Health {
                value: 100,
                position: spawn_position,
            })
        }

        if rng.gen_range(0.0, 1.0) < 0.1 {
            insert_channel.lock().unwrap().single_write(InsertEvent::Coin {
                value: 1,
                position: spawn_position,
            });
        }
        if rng.gen_range(0.0, 1.0) < 0.05 {
            insert_channel.lock().unwrap().single_write(InsertEvent::SideBulletCollectable {
                position: spawn_position,
            });
        }
        if rng.gen_range(0.0, 1.0) < 0.02 {
            insert_channel.lock().unwrap().single_write(InsertEvent::DoubleCoinsCollectable {
                position: spawn_position,
            });
        }
        if rng.gen_range(0.0, 1.0) < 0.02 {
            insert_channel.lock().unwrap().single_write(InsertEvent::DoubleExpCollectable {
                position: spawn_position,
            });
        }
    }
    flame::end("asteroids");
}

/// returns true if killed
fn process_damage(
    life: &mut Lifes,
    mut shield: Option<&mut Shield>,
    mut projectile_damage: usize,
) -> bool {
    match shield {
        Some(ref mut shield) if shield.0 > 0usize => {
            if shield.0 > projectile_damage {
                shield.0 -= projectile_damage;
                projectile_damage = 0;
            } else {
                shield.0 = 0;
                projectile_damage -= shield.0;
            }
        }
        _ => {}
    };
    if life.0 == 0 {
        return false;
    }
    if life.0 > projectile_damage {
        life.0 -= projectile_damage
    } else {
        life.0 = 0;
        return true;
    }
    false
}

fn ship_explode(
    ship_pos: Point2,
    insert_channel: &mut Write<EventChannel<InsertEvent>>,
    sounds_channel: &mut Write<EventChannel<Sound>>,
    preloaded_sounds: &ReadExpect<PreloadedSounds>,
) {
    insert_channel.single_write(InsertEvent::Exp {
        value: 50,
        position: ship_pos,
    });
    let effect = InsertEvent::Explosion {
        position: ship_pos,
        num: 30,
        lifetime: Duration::from_secs(EXPLOSION_LIFETIME_SECS),
        with_animation: Some(1f32),
    };

    insert_channel.single_write(InsertEvent::Wobble(EXPLOSION_WOBBLE));
    insert_channel.single_write(effect);
    sounds_channel
        .single_write(Sound(preloaded_sounds.ship_explosion, ship_pos));
}

fn bullet_contact(
    contact_pos: Point2,
    insert_channel: &mut Write<EventChannel<InsertEvent>>,
    _sounds_channel: &mut Write<EventChannel<Sound>>,
    _preloaded_sounds: &ReadExpect<PreloadedSounds>,
    preloaded_images: &ReadExpect<PreloadedImages>,
) {
    let effect = InsertEvent::Explosion {
        position: contact_pos,
        num: 2,
        lifetime: Duration::from_secs(EXPLOSION_LIFETIME_SECS),
        with_animation: None,
    };
    let animation = InsertEvent::Animation {
        animation: preloaded_images.bullet_contact.clone(),
        lifetime: Duration::from_secs(BULLET_CONTACT_LIFETIME_SECS),
        pos: contact_pos,
        size: 1f32,
    };
    insert_channel.single_write(animation);
    insert_channel.single_write(effect);
}

fn asteroid_explode(
    explode_position: Point2,
    insert_channel: &mut Write<EventChannel<InsertEvent>>,
    sounds_channel: &mut Write<EventChannel<Sound>>,
    preloaded_sounds: &ReadExpect<PreloadedSounds>,
    _preloaded_images: &ReadExpect<PreloadedImages>,
    size: f32,
) {
    sounds_channel.single_write(Sound(
        preloaded_sounds.asteroid_explosion,
        explode_position,
    ));
    let effect = InsertEvent::Explosion {
        position: Point2::new(explode_position.x, explode_position.y),
        num: 30usize,
        lifetime: Duration::from_secs(EXPLOSION_LIFETIME_SECS),
        with_animation: Some(size),
    };
    insert_channel.single_write(effect);
    sounds_channel.single_write(Sound(
        preloaded_sounds.asteroid_explosion,
        Point2::new(explode_position.x, explode_position.y),
    ));
}

fn blast_explode(
    position: Point2,
    insert_channel: &mut Write<EventChannel<InsertEvent>>,
    sounds_channel: &mut Write<EventChannel<Sound>>,
    preloaded_sounds: &ReadExpect<PreloadedSounds>,
    preloaded_images: &ReadExpect<PreloadedImages>,
    blast_radius: f32,
) {
    insert_channel.single_write(InsertEvent::Animation {
        animation: preloaded_images.blast.clone(),
        lifetime: Duration::from_secs(BLAST_LIFETIME_SECS),
        pos: Point2::new(position.x, position.y),
        size: blast_radius,
    });
    sounds_channel.single_write(Sound(
        preloaded_sounds.blast,
        Point2::new(position.x, position.y),
    ));
}

pub fn to_menu(
    app_state: &mut Write<AppState>,
    _progress: &mut Write<Progress>,
    _score_table: &mut Vec<usize>,
) {
    **app_state = AppState::DeadScreen;
}

fn reflect(d: Vector2, n: Vector2) -> Vector2 {
    d - 2.0 * (d.dot(&n)) * n
}

fn get_min_dist(
    world: &mut Write<World<f32>>,
    ray: Ray<f32>,
    collision_gropus: CollisionGroups,
) -> (f32, Option<BodyHandle>) {
    let mut mintoi = std::f32::MAX;
    let mut closest_body = None;
    for (b, inter) in world
        .collider_world()
        .interferences_with_ray(&ray, &collision_gropus)
    {
        if !b.query_type().is_proximity_query()
            && inter.toi < mintoi
            && inter.toi > EPS
        {
            mintoi = inter.toi;
            closest_body = Some(b.body());
        }
    }
    (mintoi, closest_body)
}
