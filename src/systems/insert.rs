use super::*;
use log::info;

// pub fn insert_character(
//     entities: &Entities,
//     gun_kind: GunKind,
//     ship_stats: ShipStats,
//     image: Image,
//     progress: Write<Progress>,
//     lazy_update: Read<LazyUpdate>,
// ) {
//     *progress = Progress::default();
//     let char_size = 0.5f32;
//     let character_shape = Geometry::Circle { radius: char_size };
//     let enemy_size = 0.4f32;
//     let _enemy_shape = Geometry::Circle { radius: enemy_size };
//     let life = Lifes(ship_stats.max_health);
//     let shield = Shield(ship_stats.max_shield);
//     let character = entities.create();
//     match gun_kind {
//         GunKind::MultyLazer(multy_lazer) => {
//             lazy_update.insert(character, multy_lazer.clone())
//         }
//         GunKind::ShotGun(shotgun) => {
//             lazy_update.insert(character, shotgun);
//         }
//         _ => unimplemented!(),
//     };
//     lazy_update.insert(character, life);
//     lazy_update.insert(character, shield);
//     lazy_update.insert(character, Isometry::new(0f32, 0f32, 0f32));
//     lazy_update.insert(character, Velocity::new(0f32, 0f32));
//     lazy_update.insert(character, CharacterMarker::default());
//     lazy_update.insert(character, Damage(ship_stats.damage));
//     lazy_update.insert(character, ShipMarker::default());
//     lazy_update.insert(character, image);
//     lazy_update.insert(character, Spin::default());
//     lazy_update.insert(character, character_shape);
//     lazy_update.insert(character, Size(char_size));
//     lazy_update.insert(character, ship_stats);
//     let character_physics_shape = ncollide2d::shape::Ball::new(char_size);

//     let mut character_collision_groups = CollisionGroups::new();
//     character_collision_groups.set_membership(&[CollisionId::PlayerShip as usize]);
//     character_collision_groups.set_whitelist(&[
//         CollisionId::Asteroid as usize,
//         CollisionId::EnemyBullet as usize,
//         CollisionId::EnemyShip as usize,
//     ]);
//     character_collision_groups.set_blacklist(&[CollisionId::PlayerBullet as usize]);

//     PhysicsComponent::safe_insert(
//         &mut physics,
//         character,
//         ShapeHandle::new(character_physics_shape),
//         Isometry2::new(Vector2::new(0f32, 0f32), 0f32),
//         Velocity2::new(Vector2::new(0f32, 0f32), 0f32),
//         BodyStatus::Dynamic,
//         &mut world,
//         &mut bodies_map,
//         character_collision_groups,
//         0.5f32,
//     );
//     {
//         entities
//             .build_entity()
//             .with(Isometry::new(0f32, 0f32, 0f32), &mut isometries)
//             .with(AttachPosition(character), &mut attach_positions)
//             .with(Velocity::new(0f32, 0f32), &mut velocities)
//             .with(Image(preloaded_images.light_white), &mut images)
//             .with(Spin::default(), &mut spins)
//             .with(Size(15f32), &mut sizes)
//             .with(LightMarker, &mut lights)
//             .build();
//     }
// }

pub fn add_text(
    entities: &Entities,
    text: TextComponent,
    lazy_update: &Read<LazyUpdate>,
    position: Point2,
    lifetime: Option<Lifetime>,
) {
    let entity = entities.create();
    lazy_update.insert(entity, text);
    lazy_update.insert(entity, Isometry::new(position.x, position.y, 0f32));
    if let Some(lifetime) = lifetime {
        lazy_update.insert(entity, lifetime);
    }
}

pub fn add_screen_text(
    entities: &Entities,
    text: TextComponent,
    lazy_update: &Read<LazyUpdate>,
    position: Point2,
    lifetime: Option<Lifetime>,
) {
    let entity = entities.create();
    lazy_update.insert(entity, text);
    if let Some(lifetime) = lifetime {
        lazy_update.insert(entity, lifetime);
    }
    lazy_update.insert(entity, Position2D(Point2::new(position.x, position.y)));
}

pub struct InsertSystem {
    reader: ReaderId<InsertEvent>,
}

impl InsertSystem {
    pub fn new(reader: ReaderId<InsertEvent>) -> Self {
        InsertSystem { reader: reader }
    }
}

impl<'a> System<'a> for InsertSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, PhysicsComponent>,
        ReadExpect<'a, ThreadPin<red::GL>>,
        WriteExpect<'a, PreloadedImages>,
        Write<'a, World<f32>>,
        Write<'a, BodiesMap>,
        Write<'a, Progress>,
        Read<'a, EventChannel<InsertEvent>>,
        WriteExpect<'a, Canvas>,
        Read<'a, LazyUpdate>,
        Write<'a, UpgradesStats>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            mut physics,
            gl,
            preloaded_images,
            mut world,
            mut bodies_map,
            mut progress,
            insert_channel,
            mut canvas,
            lazy_update,
            mut upgrades_stats,
        ) = data;
        let mut rng = thread_rng();
        info!("asteroids: started insert system");
        for insert in insert_channel.read(&mut self.reader) {
            match insert {
                InsertEvent::Character {
                    gun_kind,
                    ship_stats,
                    image,
                } => {
                    *progress = Progress::default();
                    let char_size = 0.5f32;
                    let character_shape =
                        Geometry::Circle { radius: char_size };
                    let enemy_size = 0.4f32;
                    let _enemy_shape = Geometry::Circle { radius: enemy_size };
                    let life = Lifes(ship_stats.max_health);
                    let shield = Shield(ship_stats.max_shield);
                    let character = entities.create();
                    match gun_kind {
                        GunKind::MultyLazer(multy_lazer) => {
                            lazy_update.insert(character, multy_lazer.clone())
                        }
                        GunKind::ShotGun(shotgun) => {
                            lazy_update.insert(character, *shotgun);
                        }
                        _ => unimplemented!(),
                    };
                    lazy_update.insert(character, life);
                    lazy_update.insert(character, shield);
                    lazy_update
                        .insert(character, Isometry::new(0f32, 0f32, 0f32));
                    lazy_update.insert(character, Velocity::new(0f32, 0f32));
                    lazy_update.insert(character, CharacterMarker::default());
                    lazy_update.insert(character, Damage(ship_stats.damage));
                    lazy_update.insert(character, ShipMarker::default());
                    lazy_update.insert(character, *image);
                    lazy_update.insert(character, Spin::default());
                    lazy_update.insert(character, character_shape);
                    lazy_update.insert(character, Size(char_size));
                    lazy_update.insert(character, *ship_stats);
                    let character_physics_shape =
                        ncollide2d::shape::Ball::new(char_size);

                    let mut character_collision_groups = CollisionGroups::new();
                    character_collision_groups
                        .set_membership(&[CollisionId::PlayerShip as usize]);
                    character_collision_groups.set_whitelist(&[
                        CollisionId::Asteroid as usize,
                        CollisionId::EnemyBullet as usize,
                        CollisionId::EnemyShip as usize,
                    ]);
                    character_collision_groups
                        .set_blacklist(&[CollisionId::PlayerBullet as usize]);

                    PhysicsComponent::safe_insert(
                        &mut physics,
                        character,
                        ShapeHandle::new(character_physics_shape),
                        Isometry2::new(Vector2::new(0f32, 0f32), 0f32),
                        Velocity2::new(Vector2::new(0f32, 0f32), 0f32),
                        BodyStatus::Dynamic,
                        &mut world,
                        &mut bodies_map,
                        character_collision_groups,
                        0.5f32,
                    );
                    {
                        let light = entities.create();
                        lazy_update
                            .insert(light, Isometry::new(0f32, 0f32, 0f32));
                        lazy_update.insert(light, AttachPosition(character));
                        lazy_update.insert(light, Velocity::new(0f32, 0f32));
                        lazy_update.insert(light, preloaded_images.light_white);
                        lazy_update.insert(light, Spin::default());
                        lazy_update.insert(light, Size(15f32));
                        lazy_update.insert(light, LightMarker);
                        // entities
                        //     .build_entity()
                        //     .with(Isometry::new(0f32, 0f32, 0f32), &mut isometries)
                        //     .with(AttachPosition(character), &mut attach_positions)
                        //     .with(Velocity::new(0f32, 0f32), &mut velocities)
                        //     .with(preloaded_images.light_white, &mut images)
                        //     .with(Spin::default(), &mut spins)
                        //     .with(Size(15f32), &mut sizes)
                        //     .with(LightMarker, &mut lights)
                        //     .build();
                    }
                }
                InsertEvent::Asteroid {
                    iso,
                    velocity,
                    polygon,
                    spin,
                } => {
                    let mut polygon = polygon.clone();
                    let center = polygon.center();
                    polygon.centralize(Rotation2::new(iso.z));
                    let light_shape = Geometry::Polygon(polygon.clone());
                    let iso =
                        Point3::new(iso.x + center.x, iso.y + center.y, 0.0);
                    let physics_polygon = if let Some(physics_polygon) =
                        ncollide2d::shape::ConvexPolygon::try_from_points(
                            &polygon.points(),
                        ) {
                        physics_polygon
                    } else {
                        // TODO: looks like BUG!
                        dbg!(&polygon.points);
                        break;
                        // panic!();
                    };
                    let triangulation =
                        polygon.clone().into_rounded(5).triangulate();
                    let geom_data = GeometryData::new(
                        &gl,
                        &triangulation.points,
                        &triangulation.indicies,
                    )
                    .unwrap();
                    let asteroid = entities.create();
                    lazy_update.insert(asteroid, light_shape.clone());
                    lazy_update
                        .insert(asteroid, Isometry::new(iso.x, iso.y, iso.z));
                    lazy_update.insert(
                        asteroid,
                        Velocity::new(velocity.linear.x, velocity.linear.y),
                    );
                    lazy_update.insert(
                        asteroid,
                        Lifes(
                            (ASTEROID_MAX_LIFES as f32 * polygon.min_r
                                / ASTEROID_MAX_RADIUS)
                                as usize,
                        ),
                    );
                    lazy_update.insert(asteroid, polygon);
                    lazy_update.insert(asteroid, AsteroidMarker::default());
                    lazy_update.insert(asteroid, Spin(*spin));
                    lazy_update.insert(asteroid, Size(1f32));
                    lazy_update.insert(asteroid, ThreadPin::new(geom_data));

                    // let asteroid = entities
                    //     .build_entity()
                    //     .with(light_shape.clone(), &mut geometries)
                    //     .with(Isometry::new(iso.x, iso.y, iso.z), &mut isometries)
                    //     .with(Velocity::new(velocity.linear.x, velocity.linear.y), &mut velocities)
                    //     .with(Lifes((ASTEROID_MAX_LIFES as f32 * polygon.min_r / ASTEROID_MAX_RADIUS) as usize), &mut lifes)
                    //     .with(polygon, &mut polygons)
                    //     .with(AsteroidMarker::default(), &mut asteroid_markers)
                    //     .with(Image(preloaded_images.asteroid), &mut images)
                    //     .with(Spin(*spin), &mut spins)
                    //     .with(Size(1f32), &mut sizes)
                    //     .build();

                    let mut asteroid_collision_groups = CollisionGroups::new();
                    asteroid_collision_groups
                        .set_membership(&[CollisionId::Asteroid as usize]);
                    asteroid_collision_groups.set_whitelist(&[
                        CollisionId::Asteroid as usize,
                        CollisionId::EnemyShip as usize,
                        CollisionId::PlayerShip as usize,
                        CollisionId::PlayerBullet as usize,
                        CollisionId::EnemyBullet as usize,
                    ]);
                    PhysicsComponent::safe_insert(
                        &mut physics,
                        asteroid,
                        ShapeHandle::new(physics_polygon),
                        Isometry2::new(Vector2::new(iso.x, iso.y), iso.z),
                        *velocity,
                        BodyStatus::Dynamic,
                        &mut world,
                        &mut bodies_map,
                        asteroid_collision_groups,
                        ASTEROID_INERTIA,
                    );
                }
                InsertEvent::Ship {
                    iso,
                    light_shape: _,
                    spin: _,
                    kind,
                    gun_kind,
                    ship_stats,
                    size,
                    image,
                    snake,
                    rift,
                } => {
                    let mut kind = kind.clone();
                    let num =
                        if let Some(chains) = snake { *chains } else { 1 };
                    let mut last_entity = None;
                    for i in 0..num {
                        let size = *size;
                        let enemy_shape = Geometry::Circle { radius: size };
                        let enemy_physics_shape =
                            ncollide2d::shape::Ball::new(size);
                        let mut enemy_collision_groups = CollisionGroups::new();
                        enemy_collision_groups
                            .set_membership(&[CollisionId::EnemyShip as usize]);
                        enemy_collision_groups.set_whitelist(&[
                            CollisionId::Asteroid as usize,
                            CollisionId::EnemyShip as usize,
                            CollisionId::PlayerShip as usize,
                            CollisionId::PlayerBullet as usize,
                        ]);
                        enemy_collision_groups.set_blacklist(&[
                            CollisionId::EnemyBullet as usize,
                        ]);
                        let enemy = entities.create();

                        match gun_kind {
                            GunKind::ShotGun(shotgun) => {
                                let side_num = 3usize;
                                let _shift = std::f32::consts::PI
                                    / (side_num as f32 + 1.0);
                                lazy_update.insert(enemy, *shotgun);
                            }
                            GunKind::MultyLazer(multy_lazer) => {
                                lazy_update.insert(enemy, multy_lazer.clone());
                            }
                            GunKind::Cannon(cannon) => {
                                lazy_update.insert(enemy, cannon.clone());
                            }
                            GunKind::RocketGun(rocket_gun) => {
                                lazy_update.insert(enemy, *rocket_gun);
                            }
                        }
                        for kind in kind.kinds.iter_mut() {
                            match kind {
                                AIType::Charging(time) => {
                                    lazy_update
                                        .insert(enemy, Charge::new(*time));
                                    // enemy = enemy
                                    //     .with(Charge::new(*time), &mut chargings)
                                }
                                AIType::FollowRotate { spin: None } => {
                                    *kind = AIType::FollowRotate {
                                        spin: Some(rng.gen_range(-8.0, 8.0)),
                                    }
                                }
                                _ => (),
                            }
                        }
                        let iso = Point3::new(iso.x + i as f32, iso.y, iso.z);
                        lazy_update
                            .insert(enemy, Isometry::new(iso.x, iso.y, iso.z));
                        lazy_update.insert(enemy, Velocity::new(0f32, 0f32));
                        lazy_update.insert(enemy, EnemyMarker::default());
                        lazy_update.insert(enemy, ShipMarker::default());
                        lazy_update.insert(enemy, *image);
                        lazy_update.insert(enemy, Damage(ship_stats.damage));
                        lazy_update.insert(enemy, Lifes(ship_stats.max_health));
                        lazy_update.insert(enemy, *ship_stats);
                        // if let AIType::FollowRotate{spin: None} = kind.clone() {
                        //     lazy_update.insert(enemy,AIType::FollowRotate{spin: Some(rng.gen_range(1.0, 5.0))})
                        // } else {
                        lazy_update.insert(enemy, kind.clone());
                        // }
                        lazy_update.insert(enemy, Spin::default());
                        lazy_update.insert(enemy, enemy_shape);
                        lazy_update.insert(enemy, Size(size));
                        lazy_update.insert(enemy, DamageFlash(0f32));
                        PhysicsComponent::safe_insert(
                            &mut physics,
                            enemy,
                            ShapeHandle::new(enemy_physics_shape),
                            Isometry2::new(Vector2::new(iso.x, iso.y), iso.z),
                            Velocity2::new(Vector2::new(0f32, 0f32), 0f32),
                            BodyStatus::Dynamic,
                            &mut world,
                            &mut bodies_map,
                            enemy_collision_groups,
                            0.5f32,
                        );
                        // snake thing
                        if let Some(last_entity) = last_entity {
                            if snake.is_some() {
                                lazy_update.insert(
                                    enemy,
                                    Chain {
                                        follow: last_entity,
                                    },
                                )
                            }
                        }
                        if let Some(rift) = rift {
                            lazy_update.insert(enemy, rift.clone());
                            // lazy_update.insert(enemy, Aim(last_entity))
                        }
                        last_entity = Some(enemy);
                        // with light
                        //     {
                        // let _light = entities
                        //     .build_entity()
                        //     .with(Isometry::new(0f32, 0f32, 0f32), &mut isometries)
                        //     .with(Velocity::new(0f32, 0f32), &mut velocities)
                        //     .with(Spin::default(), &mut spins)
                        //     .with(AttachPosition(enemy), &mut attach_positions)
                        //     .with(Image(preloaded_images.light_sea), &mut images)
                        //     .with(Size(1f32), &mut sizes)
                        //     .with(LightMarker, &mut lights)
                        //     .build();
                        // }
                    }
                }
                InsertEvent::Bullet {
                    kind,
                    iso,
                    size,
                    velocity,
                    damage,
                    owner,
                    lifetime,
                    bullet_image,
                    blast,
                    reflection,
                } => {
                    let bullet = entities.create();
                    lazy_update.insert(bullet, Damage(*damage));
                    lazy_update
                        .insert(bullet, Velocity::new(velocity.x, velocity.y));
                    lazy_update
                        .insert(bullet, Isometry::new(iso.x, iso.y, iso.z));
                    lazy_update.insert(bullet, *bullet_image);
                    lazy_update.insert(bullet, Spin::default());
                    lazy_update.insert(bullet, Projectile { owner: *owner });
                    lazy_update.insert(bullet, Lifetime::new(*lifetime));
                    lazy_update.insert(bullet, Size(*size));
                    if let Some(reflection) = reflection {
                        lazy_update.insert(bullet, *reflection);
                    }

                    // let mut bullet = entities
                    //     .build_entity()
                    //     .with(Damage(*damage), &mut damages)
                    //     .with(Velocity::new(velocity.x, velocity.y), &mut velocities)
                    //     .with(Isometry::new(iso.x, iso.y, iso.z), &mut isometries)
                    //     .with(Image(bullet_image.0), &mut images)
                    //     .with(Spin::default(), &mut spins)
                    //     .with(Projectile { owner: *owner }, &mut projectiles)
                    //     .with(Lifetime::new(*lifetime), &mut lifetimes)
                    //     .with(Size(r), &mut sizes);
                    if let Some(blast) = blast {
                        lazy_update.insert(bullet, *blast);
                        // bullet = bullet
                        //     .with(*blast, &mut blasts)
                    }
                    // let bullet = bullet
                    //     .build();
                    let bullet_collision_groups = get_collision_groups(*kind);
                    let ball = ncollide2d::shape::Ball::new(*size);
                    let bullet_physics_component =
                        PhysicsComponent::safe_insert(
                            &mut physics,
                            bullet,
                            ShapeHandle::new(ball),
                            Isometry2::new(Vector2::new(iso.x, iso.y), iso.z),
                            Velocity2::new(Vector2::new(0f32, 0f32), 0f32),
                            BodyStatus::Dynamic,
                            &mut world,
                            &mut bodies_map,
                            bullet_collision_groups,
                            0.1f32,
                        );
                    let body = world
                        .rigid_body_mut(bullet_physics_component.body_handle)
                        .unwrap();
                    let mut velocity_tmp = *body.velocity();
                    *velocity_tmp.as_vector_mut() =
                        Vector3::new(velocity.x, velocity.y, 0f32);
                    body.set_velocity(velocity_tmp);
                }
                InsertEvent::Rocket {
                    kind,
                    iso,
                    damage,
                    owner,
                    rocket_image,
                } => {
                    let r = 0.3;
                    let entity = entities.create();
                    lazy_update.insert(entity, Damage(*damage));
                    lazy_update
                        .insert(entity, Isometry::new(iso.x, iso.y, iso.z));
                    lazy_update.insert(entity, Velocity::new(0f32, 0f32));
                    lazy_update.insert(entity, *rocket_image);
                    lazy_update.insert(entity, Spin::default());
                    lazy_update.insert(entity, Rocket(Instant::now()));
                    lazy_update.insert(entity, Projectile { owner: *owner });
                    lazy_update.insert(entity, Size(r));
                    let bullet_collision_groups = get_collision_groups(*kind);
                    let ball = ncollide2d::shape::Ball::new(r);
                    let bullet_physics_component =
                        PhysicsComponent::safe_insert(
                            &mut physics,
                            entity,
                            ShapeHandle::new(ball),
                            Isometry2::new(Vector2::new(iso.x, iso.y), iso.z),
                            Velocity2::new(Vector2::new(0f32, 0f32), 0f32),
                            BodyStatus::Dynamic,
                            &mut world,
                            &mut bodies_map,
                            bullet_collision_groups,
                            0.25f32,
                        );
                    let _body = world
                        .rigid_body_mut(bullet_physics_component.body_handle)
                        .unwrap();
                }
                InsertEvent::Coin { value, position } => {
                    let iso = Isometry::new(position.x, position.y, 0f32);
                    let entity = entities.create();
                    lazy_update.insert(entity, CollectableMarker);
                    lazy_update.insert(entity, Coin(*value));
                    lazy_update.insert(entity, iso);
                    lazy_update.insert(entity, Size(0.25));
                    lazy_update.insert(entity, preloaded_images.coin);
                    lazy_update.insert(
                        entity,
                        Lifetime::new(Duration::from_secs(COIN_LIFETIME_SECS)),
                    );
                }
                InsertEvent::SideBulletCollectable { position } => {
                    let iso = Isometry::new(position.x, position.y, 0f32);
                    let entity = entities.create();
                    lazy_update.insert(entity, CollectableMarker);
                    lazy_update.insert(entity, SideBulletCollectable);
                    lazy_update.insert(
                        entity,
                        Lifetime::new(Duration::from_secs(
                            COLLECTABLE_SIDE_BULLET,
                        )),
                    );
                    lazy_update.insert(entity, iso);
                    lazy_update.insert(entity, Size(0.5));
                    lazy_update
                        .insert(entity, preloaded_images.side_bullet_ability);
                }
                InsertEvent::SideBulletAbility => {
                    let entity = entities.create();
                    lazy_update.insert(entity, SideBulletAbility);
                    lazy_update.insert(
                        entity,
                        Lifetime::new(Duration::from_secs(
                            SIDE_BULLET_LIFETIME_SEC,
                        )),
                    );
                }
                InsertEvent::DoubleCoinsCollectable { position } => {
                    let entity = entities.create();
                    let iso = Isometry::new(position.x, position.y, 0f32);
                    lazy_update.insert(entity, CollectableMarker);
                    lazy_update.insert(entity, DoubleCoinsCollectable);
                    lazy_update.insert(
                        entity,
                        Lifetime::new(Duration::from_secs(
                            COLLECTABLE_DOUBLE_COINS_SEC,
                        )),
                    );
                    lazy_update.insert(entity, iso);
                    lazy_update.insert(entity, Size(0.5));
                    lazy_update.insert(entity, preloaded_images.double_coin);
                }
                InsertEvent::DoubleCoinsAbility => {
                    let entity = entities.create();
                    upgrades_stats.coins_mult *= 2;
                    lazy_update.insert(entity, DoubleCoinsAbility);
                    lazy_update.insert(
                        entity,
                        Lifetime::new(Duration::from_secs(
                            DOUBLE_COINS_LIFETIME_SEC,
                        )),
                    );
                }
                InsertEvent::DoubleExpCollectable { position } => {
                    let iso = Isometry::new(position.x, position.y, 0f32);
                    let coin_entity = entities.create();
                    lazy_update.insert(coin_entity, CollectableMarker);
                    lazy_update.insert(coin_entity, DoubleExpCollectable);
                    lazy_update.insert(
                        coin_entity,
                        Lifetime::new(Duration::from_secs(
                            COLLECTABLE_DOUBLE_COINS_SEC,
                        )),
                    );
                    lazy_update.insert(coin_entity, iso);
                    lazy_update.insert(coin_entity, Size(0.5));
                    lazy_update
                        .insert(coin_entity, preloaded_images.double_exp);
                    lazy_update.insert(
                        coin_entity,
                        Lifetime::new(Duration::from_secs(COIN_LIFETIME_SECS)),
                    );
                }
                InsertEvent::DoubleExpAbility => {
                    upgrades_stats.exp_mult *= 2;
                    let entity = entities.create();
                    lazy_update.insert(entity, DoubleExpAbility);
                    lazy_update.insert(
                        entity,
                        Lifetime::new(Duration::from_secs(
                            DOUBLE_COINS_LIFETIME_SEC,
                        )),
                    );
                }
                InsertEvent::Health { value, position } => {
                    let iso = Isometry::new(position.x, position.y, 0f32);
                    let coin_entity = entities.create();
                    lazy_update.insert(coin_entity, CollectableMarker);
                    lazy_update.insert(coin_entity, Health(*value));
                    lazy_update.insert(coin_entity, iso);
                    lazy_update.insert(coin_entity, Size(0.25));
                    lazy_update.insert(coin_entity, preloaded_images.health);
                    lazy_update.insert(
                        coin_entity,
                        Lifetime::new(Duration::from_secs(COIN_LIFETIME_SECS)),
                    );
                }
                InsertEvent::Exp { value, position } => {
                    let iso = Isometry::new(position.x, position.y, 0f32);
                    let exp_entity = entities.create();
                    lazy_update.insert(exp_entity, CollectableMarker);
                    lazy_update.insert(exp_entity, Exp(*value));
                    lazy_update.insert(exp_entity, iso);
                    lazy_update.insert(exp_entity, Size(0.25));
                    lazy_update.insert(exp_entity, preloaded_images.exp);
                }
                InsertEvent::Explosion {
                    position,
                    num,
                    lifetime,
                    with_animation,
                } => {
                    let iso = Isometry::new(position.x, position.y, 0f32);
                    if let Some(size) = with_animation {
                        let animation_entity = entities.create();
                        lazy_update.insert(animation_entity, iso);
                        lazy_update.insert(
                            animation_entity,
                            preloaded_images.explosion.clone(),
                        );
                        lazy_update.insert(
                            animation_entity,
                            Lifetime::new(Duration::from_secs(
                                EXPLOSION_LIFETIME_SECS,
                            )),
                        );
                        lazy_update.insert(animation_entity, Size(size * 2.0));
                    }
                    // particles of explosion
                    let explosion_particles = ThreadPin::new(
                        ParticlesData::Explosion(Explosion::new(
                            &gl,
                            *position,
                            *num,
                            Some(*lifetime),
                        )),
                    );
                    let explosion_particles_entity = entities.create();
                    lazy_update.insert(
                        explosion_particles_entity,
                        explosion_particles,
                    );
                }
                InsertEvent::Animation {
                    animation,
                    lifetime,
                    pos,
                    size,
                } => {
                    let iso = Isometry::new(pos.x, pos.y, 0f32);
                    let animation_entity = entities.create();
                    lazy_update.insert(animation_entity, iso);
                    lazy_update.insert(animation_entity, animation.clone());
                    lazy_update
                        .insert(animation_entity, Lifetime::new(*lifetime));
                    lazy_update.insert(animation_entity, Size(*size));
                }
                InsertEvent::Nebula { iso } => {
                    let mut rng = thread_rng();
                    let z = rng.gen_range(-120f32, -80f32);
                    let nebulas_num = preloaded_images.nebulas.len();
                    let nebula_id = rng.gen_range(0, nebulas_num);
                    let nebula = entities.create();
                    lazy_update.insert(
                        nebula,
                        Isometry::new3d(iso.x, iso.y, z, iso.z),
                    );
                    lazy_update
                        .insert(nebula, preloaded_images.nebulas[nebula_id]);
                    lazy_update.insert(nebula, NebulaMarker::default());
                    lazy_update.insert(nebula, Size(60f32));
                }
                InsertEvent::Stars { iso } => {
                    let mut rng = thread_rng();
                    let z = rng.gen_range(-180f32, -140f32);
                    let stars_num = preloaded_images.stars.len();
                    let stars_id = rng.gen_range(0, stars_num);
                    let stars = entities.create();
                    lazy_update
                        .insert(stars, Isometry::new3d(iso.x, iso.y, z, iso.z));
                    lazy_update.insert(stars, preloaded_images.stars[stars_id]);
                    lazy_update.insert(stars, StarsMarker);
                    lazy_update.insert(stars, Size(30f32));
                }
                InsertEvent::Fog { iso } => {
                    let mut rng = thread_rng();
                    let z = rng.gen_range(-40f32, -20f32);
                    let entity = entities.create();
                    lazy_update.insert(
                        entity,
                        Isometry::new3d(iso.x, iso.y, z, iso.z),
                    );
                    lazy_update.insert(entity, preloaded_images.fog);
                    lazy_update.insert(entity, FogMarker);
                    lazy_update.insert(entity, Size(35f32));
                }
                InsertEvent::Planet { iso } => {
                    let mut rng = thread_rng();
                    let z = -45.0;
                    let planets_num = preloaded_images.planets.len();
                    let planet_id = rng.gen_range(0, planets_num);
                    let nebula = entities.create();
                    lazy_update.insert(
                        nebula,
                        Isometry::new3d(iso.x, iso.y, z, iso.z),
                    );
                    lazy_update
                        .insert(nebula, preloaded_images.planets[planet_id]);
                    lazy_update.insert(nebula, PlanetMarker::default());
                    lazy_update.insert(nebula, Size(25f32));
                }
                InsertEvent::Wobble(wobble) => canvas.add_wobble(*wobble),
            }
        }
        info!("asteroids: ended insert system");
    }
}
