use std::ops::AddAssign;

pub use crate::geometry::Polygon;
pub use crate::physics::{BodiesMap, PhysicsComponent};
pub use crate::gfx::{ImageData};
pub use crate::sound::{SoundData};
pub use crate::gui::{Button, Rectangle};
use crate::types::{*};
use sdl2::mixer::Chunk;
use sdl2::sys::SDL_Finger;
use specs::prelude::*;
use specs_derive::Component;
use crate::run::FINGER_NUMBER;
use rand::prelude::*;

pub const MAX_LIFES: usize = 100usize;
pub const ASTEROID_MAX_LIFES: usize = 100usize;
pub const MAX_SHIELDS: usize = 100usize;
pub const ENEMY_MAX_LIFES: usize = 20usize;
pub const ENEMY_MAX_SHIELDS: usize = 20usize;


pub const BULLET_SPEED_INIT: f32 = 0.5;
pub const THRUST_FORCE_INIT: f32 = 0.01;
pub const SHIP_ROTATION_SPEED_INIT: f32 = 1.0;

use crate::gfx::{unproject_with_z, ortho_unproject, Canvas as SDLCanvas};

// pub type SDLDisplay = ThreadPin<SDL2Facade>;
pub type Canvas = ThreadPin<SDLCanvas>;
pub type SpawnedUpgrades = Vec<[usize; 2]>;

#[derive(Clone, Copy)]
pub enum EntityType {
    Player,
    Enemy,
}

pub enum InsertEvent {
    Character {
        gun_kind: GunKind
    },
    Asteroid {
        iso: Point3,
        velocity: Velocity2,
        polygon: Polygon,
        light_shape: Geometry,
        spin: f32,
    },
    Ship {
        iso: Point3,
        light_shape: Geometry,
        spin: f32,
        gun_kind: GunKind,
        kind: AIType,
        image: Image
    },
    Bullet {
        kind: EntityType,
        iso: Point3,
        velocity: Point2,
        damage: usize,
        owner: specs::Entity,
    },
    // Lazer {
    //     kind: EntityType,
    //     iso: Isometry2,
    //     damage: usize,
    //     distance: f32,
    //     owner: specs::Entity
    // },
    Explosion {
        position: Point2,
        num: usize,
        lifetime: usize,
    },
    Engine {
        position: Point2,
        num: usize,
        attached: AttachPosition
    },
    Nebula {
        iso: Point3
    }
}

#[derive(Default, Clone, Copy)]
pub struct MenuChosedGun(pub Option<GunKind>);

#[derive(Debug, Clone)]
pub struct UpgradeCard {
    pub upgrade_type: UpgradeType,
    pub image: Image,
    pub name: String,
    pub description: String
}

pub type AvaliableUpgrades = Vec<UpgradeCard>;

// #[derive(Default)]
// pub struct AvaliableUpgrades {
//     pub list: Vec<UpgradeCard>
// }

#[derive(Debug, Clone, Copy)]
pub enum UpgradeType {
    AttackSpeed,
    BulletSpeed,
    ShipSpeed,
    ShipRotationSpeed,
    ShieldRegen,
    HealthRegen,
    ShieldSize,
    HealthSize,
}

#[derive(Component, Debug, Clone, Copy)]
pub enum AIType {
    ShootAndFollow,
    Kamikadze
}

#[derive(Debug, Clone, Copy)]
pub struct PlayerStats {
    // pub attack_speed: f32,
    pub bullet_speed: f32,
    pub bullet_damage: f32,
    pub thrust_force: f32,
    pub torque: f32,
    pub health_regen: usize,
    pub shield_regen: usize,
    pub max_health: usize,
    pub max_shield: usize
}

impl Default for PlayerStats {
    fn default() -> Self {
        PlayerStats {
            // attack_speed: 1f32,
            bullet_speed: 0.5f32,
            bullet_damage: 1f32,
            thrust_force: 0.01f32,
            torque: 0.2f32,
            health_regen: 1usize,
            shield_regen: 1usize,
            max_health: MAX_LIFES,
            max_shield: MAX_SHIELDS
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PlayState {
    Action,
    Upgrade
}

#[derive(Debug, Clone, Copy)]
pub enum AppState {
    Menu,
    Play(PlayState),
}

impl Default for AppState {
    fn default() -> Self {
        AppState::Menu
    }
}

#[derive(Default, Debug)]
pub struct Stat {
    pub asteroids_number: usize,
}

#[derive(Component, Debug, Clone, Copy)]
pub enum Geometry {
    Circle { radius: f32 },
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Size(pub f32);

/// Index of Images structure
#[derive(Debug, Component, Clone, Copy)]
pub struct Image(pub specs::Entity);

#[derive(Component, Clone, Copy)]
pub struct Sound(pub specs::Entity);

#[derive(Component, Clone, Copy)]
pub struct Particles(pub usize);

#[derive(Component, Clone, Copy)]
pub struct Lifes(pub usize);

#[derive(Component, Clone, Copy)]
pub struct Shield(pub usize);

/// damage on collision
#[derive(Component, Clone, Copy)]
pub struct Damage(pub usize);

#[derive(Default, Clone, Copy)]
pub struct Progress {
    pub experience: usize,
    pub level: usize
}

impl Progress {
    pub fn current_max_experience(&self) -> usize {
        100usize * (1f32 +1.2f32.powf(self.level as f32)) as usize
    }

    pub fn level_up(&mut self) {
        self.experience %= self.current_max_experience();
        self.level += 1usize;
    }
}

// pub type Images = Collector<ImageData, Image>;

/// contains preloaded images ids
/// use it when you need to insert entity in system
pub struct PreloadedImages {
    pub character: specs::Entity,
    pub projectile: specs::Entity,
    pub enemy_projectile: specs::Entity,
    pub asteroid: specs::Entity,
    pub enemy: specs::Entity,
    pub enemy2: specs::Entity,
    pub enemy3: specs::Entity,
    pub background: specs::Entity,
    pub nebulas: Vec<specs::Entity>,
    pub ship_speed_upgrade: specs::Entity,
    pub bullet_speed_upgrade: specs::Entity,
    pub attack_speed_upgrade: specs::Entity,
    pub light_white: specs::Entity,
    pub light_sea: specs::Entity,
    pub direction: specs::Entity,
    pub circle: specs::Entity,
    pub lazer: specs::Entity,
    pub blaster: specs::Entity,
    pub shotgun: specs::Entity,
}

pub struct PreloadedParticles {
    pub movement: specs::Entity,
}

#[derive(Clone, Copy, Debug)]
pub struct Finger {
    pub id: usize,
    pub x: f32,
    pub y: f32,
    pub x_o: f32,
    pub y_o: f32,
    pub pressure: f32
}

impl Finger {
    pub fn new(id: usize, x: f32, y: f32, observer: Point3, pressure: f32, width_u: u32, height_u: u32) -> Self {
        let (width, height) = (width_u as f32, height_u as f32);
        // dpi already multiplyed
        let (x, y) = (x as f32, y as f32);
        let (x, y) = (2f32 * x / width - 1f32, 2f32 * y / height - 1f32);
        // with z=0f32 -- which is coordinate of our canvas in 3d space
        let ortho_point = ortho_unproject(width_u, height_u, Point2::new(x, -y));
        let point = unproject_with_z(observer, &Point2::new(x, -y), 0f32, width_u, height_u);
        Finger{
            id: id,
            x: point.x,
            y: point.y,
            x_o: ortho_point.x,
            y_o: ortho_point.y,
            pressure: pressure
        }
    }
}

pub type Touches = [Option<Finger>; FINGER_NUMBER];

#[derive(Default, Debug)]
pub struct Mouse {
    pub o_x: f32,
    pub o_y: f32,
    pub x: f32,
    pub y: f32,
    pub left_released: bool,
    pub right_released: bool,
    pub left: bool,
    pub right: bool,
    pub wdpi: f32,
    pub hdpi: f32,
}

impl Mouse {
    /// get system location of mouse and then unproject it into canvas coordinates
    pub fn set_position(&mut self, x: i32, y: i32, observer: Point3, width_u: u32, height_u: u32) {
        let (width, height) = (width_u as f32, height_u as f32);
        // dpi already multiplyed
        let (x, y) = (x as f32, y as f32);
        let (x, y) = (2f32 * x / width - 1f32, 2f32 * y / height - 1f32);
        // with z=0f32 -- which is coordinate of our canvas in 3d space
        let ortho_point = ortho_unproject(width_u, height_u, Point2::new(x, -y));
        self.o_x = ortho_point.x;
        self.o_y = ortho_point.y;
        let point = unproject_with_z(observer, &Point2::new(x, -y), 0f32, width_u, height_u);
        self.x = point.x;
        self.y = point.y;
    }

    pub fn set_left(&mut self, is_left: bool) {
        self.left_released = self.left && !is_left;
        self.left = is_left;
    }

    pub fn set_right(&mut self, is_right: bool) {
        self.right_released = self.right && !is_right;
        self.right = is_right;
    }
}

#[derive(Default, Component)]
#[storage(NullStorage)]
pub struct CharacterMarker;

#[derive(Default, Component, Clone, Copy)]
#[storage(NullStorage)]
pub struct NebulaMarker;

#[derive(Default, Component)]
#[storage(NullStorage)]
pub struct ShipMarker;

#[derive(Default, Component)]
#[storage(NullStorage)]
pub struct EnemyMarker;

#[derive(Default, Component)]
#[storage(NullStorage)]
pub struct AsteroidMarker;

#[derive(Default, Component)]
#[storage(NullStorage)]
pub struct LightMarker;

#[derive(Component)]
pub struct Projectile {
    pub owner: specs::Entity,
}

#[derive(Component)]
pub struct Lifetime {
    life_state: usize,
    life_time: usize,
}

impl Lifetime {
    pub fn new(live_time: usize) -> Self {
        Lifetime {
            life_state: 0usize,
            life_time: live_time,
        }
    }

    pub fn update(&mut self) {
        self.life_state = usize::min(self.life_time, self.life_state + 1usize);
    }

    pub fn delete(&self) -> bool {
        self.life_state >= self.life_time
    }
}

/// attach entity positions to some other entity position
#[derive(Component, Debug)]
pub struct AttachPosition(pub specs::Entity);

#[derive(Clone, Copy)]
pub enum GunKind {
    Blaster,
    Lazer,
    ShotGun
}

#[derive(Component, Debug)]
pub struct Lazer {
    pub damage: usize,
    pub active: bool,
    pub distance: f32,
    pub current_distance: f32,
}

impl Lazer {
    pub fn new(damage: usize, distance: f32) -> Self {
        Lazer {
            damage: damage,
            active: false,
            distance: distance,
            current_distance: distance
        }
    }
}

pub trait Gun {
    fn recharge_state(&self) -> usize;

    fn set_recharge_state(&mut self, recharge_state: usize);

    fn recharge_time(&self) -> usize;

    fn update(&mut self) {
        self.set_recharge_state(usize::min(self.recharge_time(), self.recharge_state() + 1usize));
        // self.recharge_state = usize::min(self.recharge_time, self.recharge_state + 1usize);
    }

    fn is_ready(&self) -> bool {
        self.recharge_state() >= self.recharge_time()
        // self.recharge_state >= self.recharge_time
    }

    fn shoot(&mut self) -> bool {
        let result = self.is_ready();
        if result {
            self.set_recharge_state(0usize)
        };
        result
    }

    fn spawn_bullets(
        &self,
        entity_type: EntityType,
        isometry: Isometry3,
        bullet_speed: f32,
        bullet_damage: usize,
        ship_velocity: Vector2,
        owner: specs::Entity
    ) -> Vec<InsertEvent>;
}

#[derive(Component, Debug)]
pub struct ShotGun {
    recharge_state: usize,
    pub recharge_time: usize,
    pub bullets_damage: usize,
    pub side_projectiles_number: usize,
    pub angle_shift: f32,
}

impl ShotGun {
    pub fn new(recharge_time: usize, bullets_damage: usize, side_projectiles_number: usize, angle_shift: f32) -> Self {
        Self {
            recharge_state: 0usize,
            recharge_time: recharge_time,
            bullets_damage: bullets_damage,
            side_projectiles_number: side_projectiles_number,
            angle_shift: angle_shift
        }
    }
}

impl Gun for ShotGun {
    fn recharge_state(&self) -> usize {
        self.recharge_state
    }

    fn set_recharge_state(&mut self, recharge_state: usize) {
        self.recharge_state = recharge_state;
    }

    fn recharge_time(&self) -> usize {
        self.recharge_time
    }

    fn spawn_bullets(
        &self,
        entity_type: EntityType,
        isometry: Isometry3,
        bullet_speed: f32,
        bullet_damage: usize,
        ship_velocity: Vector2,
        owner: specs::Entity
    ) -> Vec<InsertEvent> {

        let mut res = vec![];
        let position = isometry.translation.vector;
        {
            let direction = isometry * Vector3::new(0f32, -1f32, 0f32);
            let velocity_rel = bullet_speed * direction;
            let projectile_velocity = Velocity::new(
                ship_velocity.x + velocity_rel.x,
                ship_velocity.y + velocity_rel.y,
            ) ;
            res.push(InsertEvent::Bullet {
                kind: entity_type,
                iso: Point3::new(position.x, position.y, isometry.rotation.euler_angles().2),
                velocity: Point2::new(projectile_velocity.0.x, projectile_velocity.0.y),
                damage: bullet_damage,
                owner: owner,
            });
        }
        for i in 1..=self.side_projectiles_number {
            for j in 0i32..=1 {
                let sign = j * 2 - 1;
                let shift = self.angle_shift * i as f32 * sign as f32;
                let rotation = Rotation3::new(Vector3::new(0f32, 0f32, shift));
                let direction = isometry * (rotation * Vector3::new(0f32, -1f32, 0f32));
                let velocity_rel = bullet_speed * direction;
                let projectile_velocity = Velocity::new(
                    ship_velocity.x + velocity_rel.x,
                    ship_velocity.y + velocity_rel.y,
                ) ;
                res.push(InsertEvent::Bullet {
                    kind: entity_type,
                    iso: Point3::new(
                        position.x, 
                        position.y, 
                        isometry.rotation.euler_angles().2 + shift
                    ),
                    velocity: Point2::new(projectile_velocity.0.x, projectile_velocity.0.y),
                    damage: self.bullets_damage,
                    owner: owner,
                });
            }
        }
        res
    }
}

/// gun reloading status and time
#[derive(Component, Debug)]
pub struct Blaster {
    recharge_state: usize,
    pub recharge_time: usize,
    pub bullets_damage: usize,
}

impl Blaster {
    pub fn new(recharge_time: usize, bullets_damage: usize) -> Self {
        Self {
            recharge_state: 0usize,
            recharge_time: recharge_time,
            bullets_damage: bullets_damage
        }
    }
}

impl Gun for Blaster {
    fn recharge_state(&self) -> usize {
        self.recharge_state
    }

    fn set_recharge_state(&mut self, recharge_state: usize) {
        self.recharge_state = recharge_state;
    }

    fn recharge_time(&self) -> usize {
        self.recharge_time
    }

    fn spawn_bullets(
        &self,
        entity_type: EntityType,
        isometry: Isometry3,
        bullet_speed: f32,
        bullet_damage: usize,
        ship_velocity: Vector2,
        owner: specs::Entity
    ) -> Vec<InsertEvent> {
        let mut rng = thread_rng();
        let mut res = vec![];
        {
            let position = isometry.translation.vector;
            let mut rng = rand::thread_rng();
            let shift = rng.gen_range(-0.2f32, 0.2f32);
            let direction = isometry * Vector3::new(shift, -1f32, 0f32).normalize();
            let velocity_rel = bullet_speed * direction;
            let projectile_velocity = Velocity::new(
                ship_velocity.x + velocity_rel.x,
                ship_velocity.y + velocity_rel.y,
            ) ;
            let insert_event = InsertEvent::Bullet {
                kind: entity_type,
                iso: Point3::new(position.x, position.y, isometry.rotation.euler_angles().2),
                velocity: Point2::new(projectile_velocity.0.x, projectile_velocity.0.y),
                damage: bullet_damage,
                owner: owner,
            };
            res.push(insert_event)
        }
        res
    }
}

/// translation + rotation
#[derive(Component, Debug, Clone, Copy)]
pub struct Isometry(pub Isometry3);

/// rotation speed
#[derive(Component, Default, Debug)]
pub struct Spin(pub f32);

impl Isometry {
    pub fn new(x: f32, y: f32, angle: f32) -> Self {
        Isometry(Isometry3::new(
            Vector3::new(x, y, 0f32),
            Vector3::new(0f32, 0f32, angle),
        ))
    }

    pub fn new3d(x: f32, y: f32, z: f32, angle: f32) -> Self {
        Isometry(Isometry3::new(
            Vector3::new(x, y, z),
            Vector3::new(0f32, 0f32, angle),
        ))
    }

    pub fn add_spin(&mut self, spin: f32) {
        let rotation = Rotation3::new(Vector3::new(0f32, 0f32, spin));
        self.0.rotation *= rotation;
    }

    /// return clockwise angle in XY plane
    pub fn rotation(&self) -> f32 {
        // self.0.rotation.angle()
        self.0.rotation.euler_angles().2
    }
}

impl AddAssign<&Velocity> for &mut Isometry {
    fn add_assign(&mut self, other: &Velocity) {
        self.0.translation.vector.x += other.0.x;
        self.0.translation.vector.y += other.0.y;
    }
}

#[derive(Copy, Clone, Component, Debug)]
pub struct Velocity(pub Vector2);

impl AddAssign<Velocity> for &mut Velocity {
    fn add_assign(&mut self, other: Velocity) {
        self.0.x += other.0.x;
        self.0.y += other.0.y;
    }
}

impl Velocity {
    pub fn new(x: f32, y: f32) -> Self {
        Velocity(Vector2::new(x, y))
    }
}
