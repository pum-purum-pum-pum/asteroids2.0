use super::*;
use log::info;

// TODO: probably move out proc gen
#[derive(Default)]
pub struct GamePlaySystem;

impl<'a> System<'a> for GamePlaySystem {
    type SystemData = (
        (
            Entities<'a>,
            WriteStorage<'a, Isometry>,
            WriteStorage<'a, Blast>,
            WriteStorage<'a, MultyLazer>,
            WriteStorage<'a, ShotGun>,
            WriteStorage<'a, Lifetime>,
            WriteStorage<'a, AsteroidMarker>,
            ReadStorage<'a, CharacterMarker>,
            ReadStorage<'a, ShipMarker>,
            WriteStorage<'a, Polygon>,
            WriteStorage<'a, Shield>,
            WriteStorage<'a, Lifes>,
            WriteStorage<'a, DamageFlash>,
            ReadStorage<'a, ShipStats>,
            ReadStorage<'a, Coin>,
            ReadStorage<'a, Exp>,
            ReadStorage<'a, Health>,
            ReadStorage<'a, SideBulletCollectable>,
            ReadStorage<'a, SideBulletAbility>,
            ReadStorage<'a, DoubleCoinsCollectable>,
            ReadStorage<'a, DoubleExpCollectable>,
            ReadStorage<'a, CollectableMarker>,
            ReadStorage<'a, DoubleCoinsAbility>,
            ReadStorage<'a, DoubleExpAbility>,
            ReadStorage<'a, AtlasImage>,
            ReadStorage<'a, Size>,
        ),
        ReadStorage<'a, ReflectBulletCollectable>,
        ReadStorage<'a, ReflectBulletAbility>,
        ReadExpect<'a, red::Viewport>,
        ReadStorage<'a, Projectile>,
        ReadExpect<'a, PreloadedImages>,
        Write<'a, EventChannel<InsertEvent>>,
        Write<'a, Progress>,
        Write<'a, SpawnedUpgrades>,
        Read<'a, AvaliableUpgrades>,
        ReadExpect<'a, Description>,
        Write<'a, CurrentWave>,
        Read<'a, Waves>,
        Write<'a, EventChannel<Sound>>,
        ReadExpect<'a, PreloadedSounds>,
        Write<'a, AppState>,
        WriteExpect<'a, MacroGame>,
        WriteExpect<'a, GlobalParams>,
        ReadExpect<'a, Arc<Mutex<EventChannel<InsertEvent>>>>,
        Read<'a, LazyUpdate>,
        Write<'a, UpgradesStats>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            (
                entities,
                mut isometries,
                blasts,
                mut multiple_lazers,
                mut shotguns,
                mut lifetimes,
                asteroid_markers,
                character_markers,
                ships,
                polygons,
                mut shields,
                mut lifes,
                mut flashes,
                ships_stats,
                coins,
                exps,
                healths,
                side_bullet_collectables,
                side_bullet_ability,
                double_coins_collectable,
                double_exp_collectable,
                collectables,
                double_coins_abilities,
                double_exp_abilities,
                atlas_images,
                sizes,
            ),
            reflect_bullet_collectable,
            reflect_bullet_ability,
            viewport,
            projectiles,
            preloaded_images,
            mut insert_channel,
            mut progress,
            mut spawned_upgrades,
            avaliable_upgrades,
            description,
            mut current_wave,
            waves,
            mut sounds_channel,
            preloaded_sounds,
            mut app_state,
            mut macro_game,
            mut global_params,
            asteroids_channel,
            lazy_update,
            mut upgrade_stats,
        ) = data;
        let dims = viewport.dimensions();
        let (w, h) = (dims.0 as f32, dims.1 as f32);
        info!("asteroids: gameplay started");
        for flash in (&mut flashes).join() {
            flash.0 /= 1.2f32;
        }
        if let Some((shield, life, ship_stats, _character)) =
            (&mut shields, &mut lifes, &ships_stats, &character_markers)
                .join()
                .next()
        {
            shield.0 =
                (shield.0 + ship_stats.shield_regen).min(ship_stats.max_shield);
            life.0 =
                (life.0 + ship_stats.health_regen).min(ship_stats.max_health);
        } else {
            return;
        };
        if progress.experience >= progress.current_max_experience() {
            progress.level_up();
            let mut rng = thread_rng();
            let up_id = rng.gen_range(0, avaliable_upgrades.len());
            let mut second_id = rng.gen_range(0, avaliable_upgrades.len());
            while second_id == up_id {
                second_id = rng.gen_range(0, avaliable_upgrades.len());
            }
            spawned_upgrades.push([up_id, second_id]);
            // *app_state = AppState::Play(PlayState::Upgrade);
        }
        let (char_entity, char_isometry, _char) =
            (&entities, &isometries, &character_markers)
                .join()
                .next()
                .unwrap();
        let char_isometry = char_isometry.clone(); // to avoid borrow
        let pos3d = char_isometry.0.translation.vector;
        let character_position = Point2::new(pos3d.x, pos3d.y);
        {
            // player trace
            let mut transparent_basic = preloaded_images.basic_ship;
            transparent_basic.transparency = 0.1;
            let trace = entities.create();
            lazy_update.insert(trace, *sizes.get(char_entity).unwrap());
            lazy_update.insert(trace, transparent_basic);
            lazy_update.insert(trace, char_isometry);
            lazy_update
                .insert(trace, Lifetime::new(Duration::from_millis(300)));
            // let transparent_projectile = ;
            for (entity, iso, projectile, bullet_image, size) in
                (&entities, &isometries, &projectiles, &atlas_images, &sizes)
                    .join()
            {
                let mut transparent_bullet = bullet_image.clone();
                transparent_bullet.transparency = 0.1;
                let trace = entities.create();
                lazy_update.insert(trace, *size);
                lazy_update.insert(trace, transparent_bullet);
                lazy_update.insert(trace, iso.clone());
                lazy_update
                    .insert(trace, Lifetime::new(Duration::from_millis(300)));
            }
        }
        for (entity, lifetime) in (&entities, &mut lifetimes).join() {
            if lifetime.delete() {
                if double_coins_abilities.get(entity).is_some() {
                    upgrade_stats.coins_mult /= 2;
                    entities.delete(entity).unwrap();
                }
                if double_exp_abilities.get(entity).is_some() {
                    upgrade_stats.exp_mult /= 2;
                    entities.delete(entity).unwrap();
                }
                if side_bullet_ability.get(entity).is_some() {
                    if let Some(gun) = shotguns.get_mut(char_entity) {
                        // it's hack to avoid overflow
                        // posibble if we forgot to delete upgrade from previous game
                        if gun.side_projectiles_number > 0 {
                            gun.side_projectiles_number -= 1;
                        }
                    }
                    if let Some(multy_lazer) =
                        multiple_lazers.get_mut(char_entity)
                    {
                        multy_lazer.minus_side_lazers();
                    }
                    entities.delete(entity).unwrap();
                }
                if let Some(reflect_ability) = reflect_bullet_ability.get(entity) {
                    dbg!(reflect_bullet_ability.count());
                    if reflect_bullet_ability.count() == 1 {
                        if let Some(gun) = shotguns.get_mut(char_entity) {
                            if gun.reflection.is_some() {
                                gun.reflection = None
                            }
                        }
                        // if let Some(ref mut reflection) = gun.reflection {
                        //     reflection.lifetime += Duration::from_millis(200);
                        // }
                    }
                    entities.delete(entity).unwrap();
                }
                if let Some(blast) = blasts.get(entity) {
                    let owner =
                        if let Some(projectile) = projectiles.get(entity) {
                            projectile.owner
                        } else {
                            entity
                        };
                    let position =
                        isometries.get(entity).unwrap().0.translation.vector;
                    blast_explode(
                        Point2::new(position.x, position.y),
                        &mut insert_channel,
                        &mut sounds_channel,
                        &preloaded_sounds,
                        &preloaded_images,
                        blast.blast_radius,
                    );

                    // process_blast_damage
                    let blast_position =
                        isometries.get(entity).unwrap().0.translation.vector;
                    for (entity, life, isometry) in
                        (&entities, &mut lifes, &isometries).join()
                    {
                        let position = isometry.0.translation.vector;
                        let is_character = entity == char_entity;
                        let is_asteroid =
                            asteroid_markers.get(entity).is_some();
                        let affected = is_character && owner != char_entity
                            || entity != char_entity
                                && (owner == char_entity || is_asteroid);
                        if affected
                            && (blast_position - position).norm()
                                < blast.blast_radius
                        {
                            if is_character {
                                global_params.damaged(DAMAGED_RED);
                            }
                            if process_damage(
                                life,
                                shields.get_mut(entity),
                                blast.blast_damage,
                            ) {
                                if is_asteroid {
                                    let asteroid = entity;
                                    let polygon = polygons.get(entity).unwrap();
                                    asteroid_explode(
                                        Point2::new(position.x, position.y),
                                        &mut insert_channel,
                                        &mut sounds_channel,
                                        &preloaded_sounds,
                                        &preloaded_images,
                                        polygon.max_r,
                                    );
                                    let iso =
                                        isometries.get(asteroid).unwrap().0;
                                    let poly =
                                        polygons.get(asteroid).unwrap().clone();
                                    let channel_arc =
                                        (*asteroids_channel).clone();
                                    thread::spawn(move || {
                                        spawn_asteroids(
                                            iso,
                                            poly,
                                            channel_arc,
                                            None,
                                        );
                                    });
                                }
                                if is_character {
                                    to_menu(
                                        &mut app_state,
                                        &mut progress,
                                        &mut macro_game.score_table,
                                    );
                                }
                                // delete character
                                entities.delete(entity).unwrap();
                                // dbg!("dead");
                            }
                        }
                    }
                    entities.delete(entity).unwrap();
                }
                entities.delete(entity).unwrap()
            }
        }
        for (entity, iso, _collectable) in
            (&entities, &mut isometries, &collectables).join()
        {
            let collectable_position = iso.0.translation.vector;
            if (pos3d - collectable_position).norm() < MAGNETO_RADIUS {
                let vel = 0.3 * (pos3d - collectable_position).normalize();
                iso.0.translation.vector += vel;
            }
            if (pos3d - collectable_position).norm() < COLLECT_RADIUS {
                let mut rng = thread_rng();
                if let Some(coin) = coins.get(entity) {
                    let coin_id = rng.gen_range(1, 3);
                    let coins_add = upgrade_stats.coins_mult * coin.0;
                    add_text(
                        &entities,
                        TextComponent {
                            text: format!("+{}", coins_add).to_string(),
                            color: (1.0, 1.0, 0.7, 1.0)
                        },
                        &lazy_update,
                        Point2::new(
                            collectable_position.x,
                            collectable_position.y,
                        ),
                        Some(Lifetime::new(Duration::from_secs(1))),
                    );
                    let coin_sound = if coin_id == 0 {
                        preloaded_sounds.coin
                    } else {
                        preloaded_sounds.coin2
                    };
                    sounds_channel.single_write(Sound(
                        coin_sound,
                        Point2::new(
                            collectable_position.x,
                            collectable_position.y,
                        ),
                    ));
                    progress.add_coins(coins_add);
                    progress.add_score(coins_add);
                    macro_game.coins += coins_add;
                }
                if let Some(exp) = exps.get(entity) {
                    sounds_channel.single_write(Sound(
                        preloaded_sounds.exp,
                        Point2::new(
                            collectable_position.x,
                            collectable_position.y,
                        ),
                    ));
                    let add_exp = upgrade_stats.exp_mult * exp.0;
                    progress.add_score(3 * add_exp);
                    add_text(
                        &entities,
                        TextComponent {
                            text: format!("+{}", add_exp).to_string(),
                            color: (0.6, 3.0, 1.0, 1.0)
                        },
                        &lazy_update,
                        Point2::new(
                            collectable_position.x,
                            collectable_position.y,
                        ),
                        Some(Lifetime::new(Duration::from_secs(1))),
                    );
                    progress.add_exp(add_exp);
                }
                if let Some(health) = healths.get(entity) {
                    lifes.get_mut(char_entity).unwrap().0 += health.0;
                }
                if side_bullet_collectables.get(entity).is_some() {
                    insert_channel.single_write(InsertEvent::SideBulletAbility);
                    add_text(
                        &entities,
                        TextComponent {
                            text: "Triple bullets".to_string(),
                            color: (1.0, 1.0, 1.0, 1.0)
                        },
                        &lazy_update,
                        Point2::new(
                            collectable_position.x,
                            collectable_position.y,
                        ),
                        Some(Lifetime::new(Duration::from_secs(1))),
                    );
                    if let Some(gun) = shotguns.get_mut(char_entity) {
                        gun.side_projectiles_number += 1;
                    }
                    if let Some(multy_lazer) =
                        multiple_lazers.get_mut(char_entity)
                    {
                        multy_lazer.plus_side_lazers();
                    }
                }
                if double_coins_collectable.get(entity).is_some() {
                    add_text(
                        &entities,
                        TextComponent {
                            text: "Double coins".to_string(),
                            color: (1.0, 1.0, 1.0, 1.0)
                        },
                        &lazy_update,
                        Point2::new(
                            collectable_position.x,
                            collectable_position.y,
                        ),
                        Some(Lifetime::new(Duration::from_secs(1))),
                    );
                    insert_channel.single_write(InsertEvent::DoubleCoinsAbility)
                }
                if reflect_bullet_collectable.get(entity).is_some() {
                    add_text(
                        &entities,
                        TextComponent {
                            text: "Reflectable".to_string(),
                            color: (1.0, 1.0, 1.0, 1.0)
                        },
                        &lazy_update,
                        Point2::new(
                            collectable_position.x,
                            collectable_position.y,
                        ),
                        Some(Lifetime::new(Duration::from_secs(1))),
                    );
                    insert_channel.single_write(InsertEvent::ReflectBulletAbility)
                }
                if double_exp_collectable.get(entity).is_some() {
                    add_text(
                        &entities,
                        TextComponent {
                            text: "Double experience".to_string(),
                            color: (1.0, 1.0, 1.0, 1.0)
                        },
                        &lazy_update,
                        Point2::new(
                            collectable_position.x,
                            collectable_position.y,
                        ),
                        Some(Lifetime::new(Duration::from_secs(1))),
                    );
                    insert_channel.single_write(InsertEvent::DoubleExpAbility)
                }
                entities.delete(entity).unwrap();
            }
        }
        let cnt = ships.count();
        let wave = &waves.0[current_wave.id];
        let (add_cnt, const_spawn) = if cnt == 1 {
            current_wave.iteration += 1;
            (wave.ships_number - cnt + 1, true)
        } else {
            (0, false)
        };
        if current_wave.iteration > wave.iterations {
            current_wave.iteration = 0;
            current_wave.id = (waves.0.len() - 1).min(current_wave.id + 1);
            add_screen_text(
                &entities,
                TextComponent {
                    text: format!("Wave {}", current_wave.id).to_string(),
                    color: (1.0, 1.0, 0.7, 1.0)
                },
                &lazy_update,
                Point2::new(
                    w / 2.0,
                    h/ 2.0,
                ),
                Some(Lifetime::new(Duration::from_secs(1))),
            );
        }
        let mut rng = thread_rng();
        fn ships2insert(spawn_pos: Point2, enemy: EnemyKind) -> InsertEvent {
            InsertEvent::Ship {
                iso: Point3::new(spawn_pos.x, spawn_pos.y, 0f32),
                light_shape: Geometry::Circle { radius: 1f32 },
                spin: 0f32,
                kind: enemy.ai_kind,
                gun_kind: enemy.gun_kind,
                ship_stats: enemy.ship_stats,
                size: enemy.size,
                image: enemy.image,
                snake: enemy.snake,
                rift: enemy.rift,
            }
        };
        for _ in 0..add_cnt {
            if wave.distribution.len() > 0 {
                let spawn_pos = spawn_position(
                    character_position,
                    PLAYER_AREA,
                    ACTIVE_AREA,
                );
                // TODO move from loop
                let ships = &description.enemies;
                let ship_id = wave
                    .distribution
                    .choose_weighted(&mut rng, |item| item.1)
                    .unwrap()
                    .0;
                insert_channel.single_write(ships2insert(
                    spawn_pos,
                    ships[ship_id].clone(),
                ));
            }
        }
        if const_spawn {
            for kind in wave.const_distribution.iter() {
                // dbg!(kind);
                for _ in 0..kind.1 {
                    let spawn_pos = spawn_position(
                        character_position,
                        PLAYER_AREA,
                        ACTIVE_AREA,
                    );
                    let ships = &description.enemies;
                    let ship_id = kind.0;
                    insert_channel.single_write(ships2insert(
                        spawn_pos,
                        ships[ship_id].clone(),
                    ));
                }
            }
        }
        info!("asteroids: gameplay ended");
    }
}
