use std::ops::AddAssign;
use std::collections::{HashMap};

pub use crate::geometry::{Polygon, NebulaGrid, PlanetGrid};
pub use crate::physics::{BodiesMap, PhysicsComponent};
pub use crate::gfx::{ImageData};
pub use crate::sound::{SoundData};
pub use crate::gui::{Button, Rectangle};
pub use crate::gfx::animation::{Animation, AnimationFrame};
use crate::types::{*};


use specs::prelude::*;
use serde::{Serialize, Deserialize};
use specs_derive::Component;
use crate::run::FINGER_NUMBER;
use rand::prelude::*;
use sdl2::mixer::Channel;

pub const ASTEROID_MAX_LIFES: usize = 1000usize;


pub const BULLET_SPEED_INIT: f32 = 0.5;
pub const THRUST_FORCE_INIT: f32 = 0.01;
pub const SHIP_ROTATION_SPEED_INIT: f32 = 1.0;

use crate::gfx::{unproject_with_z, ortho_unproject, Canvas as SDLCanvas};

// pub type SDLDisplay = ThreadPin<SDL2Facade>;
pub type Canvas = ThreadPin<SDLCanvas>;
pub type SpawnedUpgrades = Vec<[usize; 2]>;



#[derive(Debug, Default)]
pub struct CurrentWave{
    pub id: usize,
    pub iteration: usize
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Waves (pub Vec<Wave>);

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WavesSave (pub Vec<WaveSave>);

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Wave {
    pub distribution: Vec<(usize, f32)>,
    pub ships_number: usize,
    pub const_distribution: Vec<(usize, usize)>,
    pub iterations: usize
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WaveSave {
    pub distribution: Vec<(String, f32)>,
    pub ships_number: usize,
    pub const_distribution: Vec<(String, usize)>,
    pub iterations: usize
}

#[derive(Debug, Default)]
pub struct Music {
    pub current_battle: Option<usize>,
    pub menu_play: bool,
}

pub struct ChoosedUpgrade(pub Option<usize>);

#[derive(Debug, Default)]
pub struct LoopSound {
    pub player_lazer_channel: Option<Channel>,
    pub player_engine_channel: Option<Channel>
}

#[derive(Debug)]
pub struct Description {
    pub player_ships_stats: Vec<ShipStats>,
    pub player_guns: Vec<GunKind>,
    pub enemies: Vec<EnemyKind>            
}

#[derive(Debug, Clone)]
pub struct EnemyKind {
    pub ai_kind: AI,
    pub gun_kind: GunKind,
    pub ship_stats: ShipStats,
    pub size: f32,
    pub image: Image,
}

#[derive(Clone, Copy)]
pub enum EntityType {
    Player,
    Enemy,
}

#[derive(Clone)]
pub enum InsertEvent {
    Character {
        gun_kind: GunKind,
        ship_stats: ShipStats
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
        kind: AI,
        ship_stats: ShipStats,
        image: Image,
        size: f32,
    },
    Bullet {
        kind: EntityType,
        iso: Point3,
        velocity: Point2,
        damage: usize,
        owner: specs::Entity,
        bullet_image: Image,
        blast: Option<Blast>
    },
    Coin {
        value: usize,
        position: Point2
    },
    Exp {
        value: usize,
        position: Point2,
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
        with_animation: bool
    },
    Engine {
        position: Point2,
        num: usize,
        attached: AttachPosition
    },
    Nebula {
        iso: Point3
    },
    Planet {
        iso: Point3
    },
    Wobble(f32),
    Animation {
        animation: Animation,
        lifetime: usize,
        pos: Point2
    }
}

#[derive(Default, Clone)]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct AI {
    pub kinds: Vec<AIType>
}

#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AIType {
    Shoot,
    Follow,
    Aim,
    Rotate(f32),
    Kamikadze,
    Charging(usize)
}

#[derive(Debug, Clone, Copy, Component, Serialize, Deserialize)]
pub struct ShipStats {
    pub thrust_force: f32,
    pub torque: f32,
    pub health_regen: usize,
    pub shield_regen: usize,
    pub max_health: usize,
    pub max_shield: usize,
    pub damage: usize,
}

// impl Default for ShipStats {
//     fn default() -> Self {
//         Self {
//             thrust_force: 0.003f32,
//             torque: 0.2f32,
//             health_regen: 1usize,
//             shield_regen: 1usize,
//             max_health: MAX_LIFES,
//             max_shield: MAX_SHIELDS
//         }
//     }
// }

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
pub struct BlockSegment {
    pub point1: Point2, 
    pub point2: Point2 
}

#[derive(Component, Debug, Clone)]
pub enum Geometry {
    Circle { radius: f32 },
    Segment(BlockSegment),
    Polygon(Polygon),
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Charge {
    pub recharge_state: usize,
    pub recharge_time: usize,
}

impl Gun for Charge {
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
        _entity_type: EntityType,
        _isometry: Isometry3,
        _bullet_speed: f32,
        _bullet_damage: usize,
        _ship_velocity: Vector2,
        _owner: specs::Entity
    ) -> Vec<InsertEvent> {
        unimplemented!();
    }
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

#[derive(Clone)]
pub struct MacroGame {
    pub coins: usize,
}

#[derive(Default, Clone, Copy)]
pub struct Progress {
    pub experience: usize,
    pub level: usize,
    pub score: usize,
}

impl Progress {
    pub fn current_max_experience(&self) -> usize {
        100usize * (1f32 +1.2f32.powf(self.level as f32)) as usize
    }

    pub fn level_up(&mut self) {
        self.experience %= self.current_max_experience();
        self.level += 1usize;
    }

    // pub fn kill(&mut self, exp: usize, score: usize) {
    //     self.experience += exp;
    //     self.score += score;
    // }
    pub fn add_exp(&mut self, exp: usize) {
        self.experience += exp;
    }

    pub fn add_score(&mut self, score: usize) {
        self.score += score;
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
    pub enemy4: specs::Entity,
    pub background: specs::Entity,
    pub nebulas: Vec<specs::Entity>,
    pub planets: Vec<specs::Entity>,
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
    pub coin: specs::Entity,
    pub exp: specs::Entity,
    pub explosion: Animation,
    pub blast: Animation
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
    pub fn new(id: usize, x: f32, y: f32, observer: Point3, pressure: f32, width_u: u32, height_u: u32, z_far: f32) -> Self {
        let (width, height) = (width_u as f32, height_u as f32);
        // dpi already multiplyed
        let (x, y) = (x as f32, height_u as f32 - y as f32);
        let (x, y) = (2f32 * x / width - 1f32, 2f32 * y / height - 1f32);
        // with z=0f32 -- which is coordinate of our canvas in 3d space
        let ortho_point = ortho_unproject(width_u, height_u, Point2::new(x, -y));
        let point = unproject_with_z(observer, &Point2::new(x, -y), 0f32, width_u, height_u, z_far);
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
    pub x01: f32,
    pub y01:f32,
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
    pub fn set_position(&mut self, x: i32, y: i32, observer: Point3, width_u: u32, height_u: u32, z_far: f32) {
        let (width, height) = (width_u as f32, height_u as f32);
        // dpi already multiplyed
        let (x, y) = (x as f32, y as f32);
        let (x, y) = (2f32 * x / width - 1f32, 2f32 * y / height - 1f32);
        self.x01 = x;
        self.y01 = y;
        // with z=0f32 -- which is coordinate of our canvas in 3d space
        let ortho_point = ortho_unproject(width_u, height_u, Point2::new(x, y));
        self.o_x = ortho_point.x;
        self.o_y = ortho_point.y;
        let point = unproject_with_z(observer, &Point2::new(x, y), 0f32, width_u, height_u, z_far);
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

#[derive(Default, Component, Clone, Copy)]
#[storage(NullStorage)]
pub struct PlanetMarker;

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
pub struct Coin(pub usize);

#[derive(Component)]
pub struct Exp(pub usize);

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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GunKindMarker {
    Blaster,
    Lazer,
    ShotGun,
    MultyLazer,
    Cannon
}

impl Into<GunKindMarker> for &GunKind {
    fn into(self) -> GunKindMarker {
        match self {
            GunKind::Blaster(_) => GunKindMarker::Blaster,
            GunKind::ShotGun(_) => GunKindMarker::ShotGun,
            GunKind::Lazer(_) => GunKindMarker::Lazer,
            GunKind::MultyLazer(_) => GunKindMarker::MultyLazer,
            GunKind::Cannon(_) => GunKindMarker::Cannon,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Assigned {
    General,
    ToGun(GunKindMarker),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpgradeCardRaw {
    pub upgrade_type: UpgradeType,
    pub image: String,
    pub name: String,
    pub description: String,
    pub assigned: Vec<Assigned>
}

pub fn get_avaliable_cards(
    cards: &[UpgradeCardRaw], 
    gun: &GunKind,
    name_to_image: &HashMap<String, specs::Entity>
) -> Vec<UpgradeCard> {
    let gun_marker: GunKindMarker = gun.into();
    let avaliable_cards: Vec<UpgradeCard> = cards.iter().filter(
        |raw_card| {
            raw_card.assigned.contains(&Assigned::General) ||
            raw_card.assigned.contains(&Assigned::ToGun(gun_marker))
        }
    ).map(
        |upgrade| {
            UpgradeCard {
                upgrade_type: upgrade.upgrade_type,
                image: Image(name_to_image[&upgrade.image]),
                name: upgrade.name.clone(),
                description: upgrade.description.clone()
            }
        }
    ).collect();
    avaliable_cards
}

/// attach entity positions to some other entity position
#[derive(Component, Debug, Clone)]
pub struct AttachPosition(pub specs::Entity);

#[derive(Debug, Clone)]
pub enum GunKind {
    Blaster(Blaster),
    Lazer(Lazer),
    ShotGun(ShotGun),
    MultyLazer(MultyLazer),
    Cannon(Cannon)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GunKindSave {
    Blaster(BlasterSave),
    Lazer(Lazer),
    ShotGun(ShotGunSave),
    MultyLazer(MultyLazer),
    Cannon(CannonSave)
}

impl GunKindSave {
    pub fn convert(&self, name_to_image: &HashMap<String, specs::Entity>) -> GunKind {
        match self {
            GunKindSave::Blaster(blaster_save) => {
                GunKind::Blaster(blaster_save.convert(name_to_image))
            }
            GunKindSave::Lazer(lazer) => {
                GunKind::Lazer(lazer.clone())
            }
            GunKindSave::ShotGun(shotgun_save) => {
                GunKind::ShotGun(shotgun_save.convert(name_to_image))
            }
            GunKindSave::MultyLazer(multy_lazer) => {
                GunKind::MultyLazer(multy_lazer.clone())
            }
            GunKindSave::Cannon(cannon_save) => {
                GunKind::Cannon(cannon_save.convert(name_to_image))
            }
        }
        // name_to_image[]
    }
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct MultyLazer {
    pub lazers: Vec<Lazer>
}

#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Blast {
    pub blast_damage: usize,
    pub blast_radius: f32,
}

#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
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

#[derive(Component, Debug, Clone, Copy)]
pub struct ShotGun {
    recharge_state: usize,
    pub recharge_time: usize,
    pub bullets_damage: usize,
    pub side_projectiles_number: usize,
    pub angle_shift: f32,
    pub bullet_speed: f32,
    pub bullet_image: Image
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct ShotGunSave {
    recharge_state: usize,
    pub recharge_time: usize,
    pub bullets_damage: usize,
    pub side_projectiles_number: usize,
    pub angle_shift: f32,
    pub bullet_speed: f32,    
    pub bullet_image: String
}

impl ShotGunSave {
    pub fn convert(&self, name_to_image: &HashMap<String, specs::Entity>) -> ShotGun {
        ShotGun::new(
            self.recharge_time,
            self.bullets_damage,
            self.side_projectiles_number,
            self.angle_shift,
            self.bullet_speed,
            Image(name_to_image[&self.bullet_image])
        )
    }
}

impl ShotGun {
    pub fn new(
        recharge_time: usize, 
        bullets_damage: usize, 
        side_projectiles_number: usize, 
        angle_shift: f32, 
        bullet_speed: f32,
        bullet_image: Image,
    ) -> Self {
        Self {
            recharge_state: 0usize,
            recharge_time: recharge_time,
            bullets_damage: bullets_damage,
            side_projectiles_number: side_projectiles_number,
            angle_shift: angle_shift,
            bullet_speed: bullet_speed,
            bullet_image: bullet_image
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
                bullet_image: self.bullet_image,
                blast: None,
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
                    bullet_image: self.bullet_image,
                    blast: None
                });
            }
        }
        res
    }
}

/// gun reloading status and time
#[derive(Component, Debug, Clone, Copy)]
pub struct Blaster {
    recharge_state: usize,
    pub recharge_time: usize,
    pub bullets_damage: usize,
    pub bullet_speed: f32,
    pub bullet_image: Image
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct BlasterSave {
    pub recharge_state: usize,
    pub recharge_time: usize,
    pub bullets_damage: usize,
    pub bullet_speed: f32,
    pub bullet_image: String
}

impl BlasterSave {
    pub fn convert(&self, name_to_image: &HashMap<String, specs::Entity>) -> Blaster {
        Blaster::new(
            self.recharge_time, 
            self.bullets_damage, 
            self.bullet_speed, 
            Image(name_to_image[&self.bullet_image])
        )
    }
}

impl Blaster {
    pub fn new(recharge_time: usize, bullets_damage: usize, bullet_speed: f32, bullet_image: Image) -> Self {
        Self {
            recharge_state: 0usize,
            recharge_time: recharge_time,
            bullets_damage: bullets_damage,
            bullet_speed: bullet_speed,
            bullet_image: bullet_image
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
                bullet_image: self.bullet_image,
                blast: None
            };
            res.push(insert_event)
        }
        res
    }
}


#[derive(Component, Debug, Clone, Copy)]
pub struct Cannon {
    recharge_state: usize,
    pub recharge_time: usize,
    pub bullets_damage: usize,
    pub bullet_speed: f32,
    pub bullet_blast: Blast,
    pub bullet_image: Image
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct CannonSave {
    recharge_state: usize,
    pub recharge_time: usize,
    pub bullets_damage: usize,
    pub bullet_speed: f32,
    pub bullet_blast: Blast,
    pub bullet_image: String
}

impl CannonSave {
    pub fn convert(&self, name_to_image: &HashMap<String, specs::Entity>) -> Cannon {
        Cannon::new(
            self.recharge_time,
            self.bullets_damage,
            self.bullet_speed,
            self.bullet_blast,
            Image(name_to_image[&self.bullet_image])
        )
    }
}

impl Cannon {
    pub fn new(recharge_time: usize, bullets_damage: usize, bullet_speed: f32, bullet_blast: Blast, bullet_image: Image) -> Self {
        Self {
            recharge_state: 0usize,
            recharge_time: recharge_time,
            bullets_damage: bullets_damage,
            bullet_speed: bullet_speed,
            bullet_blast: bullet_blast,
            bullet_image: bullet_image
        }
    }
}

impl Gun for Cannon {
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
                bullet_image: self.bullet_image,
                blast: Some(self.bullet_blast)
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
