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
            WriteStorage<'a, StarsMarker>,
            WriteStorage<'a, BigStarMarker>,
            WriteStorage<'a, NebulaMarker>,
            WriteStorage<'a, PlanetMarker>,
            WriteStorage<'a, Shield>,
            WriteStorage<'a, Lifes>,
            ReadStorage<'a, ShipStats>,
            ReadStorage<'a, Coin>,
            ReadStorage<'a, Exp>,
            ReadStorage<'a, Health>,
            ReadStorage<'a, SideBulletCollectable>,
            ReadStorage<'a, SideBulletAbility>,
            ReadStorage<'a, DoubleCoinsCollectable>,
            ReadStorage<'a, DoubleCoinsAbility>,
            ReadStorage<'a, DoubleExpCollectable>,
            ReadStorage<'a, CollectableMarker>,
        ),
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
        WriteExpect<'a, BigStarGrid>,
        WriteExpect<'a, StarsGrid>,
        WriteExpect<'a, NebulaGrid>,
        WriteExpect<'a, PlanetGrid>,
        WriteExpect<'a, MacroGame>,
        WriteExpect<'a, GlobalParams>
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
                stars,
                big_star_markers,
                nebulas,
                planets,
                mut shields,
                mut lifes,
                ships_stats,
                coins,
                exps,
                healths,
                side_bullet_collectables,
                side_bullet_ability,
                double_coins_collectable,
                _double_coins_ability,
                double_exp_collectable,
                collectables,
            ),
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
            mut big_star_grid,
            mut stars_grid,
            mut nebula_grid,
            mut planet_grid,
            mut macro_game,
            mut global_params
        ) = data;
        info!("asteroids: gameplay started");
        if let Some((shield, life, ship_stats, _character)) = (&mut shields, &mut lifes, &ships_stats, &character_markers).join().next() {
            shield.0 = (shield.0 + ship_stats.shield_regen).min(ship_stats.max_shield);
            life.0 = (life.0 + ship_stats.health_regen).min(ship_stats.max_health);
        } else {
            return
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
        let (char_entity, char_isometry, _char) = (&entities, &isometries, &character_markers).join().next().unwrap();
        let char_isometry = char_isometry.clone(); // to avoid borrow
        let pos3d = char_isometry.0.translation.vector;
        let character_position = Point2::new(pos3d.x, pos3d.y);
        for (entity, lifetime) in (&entities, &mut lifetimes).join() {
            if lifetime.delete() {
                if side_bullet_ability.get(entity).is_some() {
                    if let Some(gun) = shotguns.get_mut(char_entity) {
                        gun.side_projectiles_number -= 1;
                    }
                    if let Some(multy_lazer) = multiple_lazers.get_mut(char_entity) {
                        multy_lazer.minus_side_lazers();
                    }
                }
                if let Some(blast) = blasts.get(entity) {
                    let owner = if let Some(projectile) = projectiles.get(entity) {
                        projectile.owner
                    } else {
                        entity
                    };
                    let position = isometries.get(entity).unwrap().0.translation.vector;
                    blast_explode(
                        Point2::new(position.x, position.y),
                        &mut insert_channel,
                        &mut sounds_channel,
                        &preloaded_sounds,
                        &preloaded_images,
                        blast.blast_radius
                    );
                    
                    // process_blast_damage
                    let blast_position = isometries.get(entity).unwrap().0.translation.vector;
                    for (entity, life, isometry) in (&entities, &mut lifes, &isometries).join() {
                        let position = isometry.0.translation.vector;
                        let is_character = entity == char_entity;
                        let is_asteroid = asteroid_markers.get(entity).is_some(); 
                        let affected = 
                            is_character && owner != char_entity ||
                            entity != char_entity && (owner == char_entity || is_asteroid);
                        if affected && (blast_position - position).norm() < blast.blast_radius {
                            if is_character {
                                global_params.damaged(DAMAGED_RED);
                            }
                            if process_damage(life, shields.get_mut(entity), blast.blast_damage) {
                                if is_asteroid {
                                    let polygon = polygons.get(entity).unwrap();
                                    asteroid_explode(
                                        Point2::new(position.x, position.y),
                                        &mut insert_channel,
                                        &mut sounds_channel,
                                        &preloaded_sounds,
                                        &preloaded_images,
                                        polygon.max_r
                                    );
                                    spawn_asteroids(
                                        isometry.0, 
                                        polygons.get(entity).unwrap(), 
                                        &mut insert_channel,
                                        None
                                    );
                                }
                                if is_character {
                                    // *app_state = AppState::Menu;
                                    to_menu(&mut app_state, &mut progress, &mut macro_game.score_table);
                                }
                                // delete character
                                entities.delete(entity).unwrap();
                                // dbg!("dead");
                            }
                        }
                    }
                }
                entities.delete(entity).unwrap()
            }
        }
        for (entity, iso, _collectable) in (&entities, &mut isometries, &collectables).join() {
            let collectable_position = iso.0.translation.vector;
            if (pos3d - collectable_position).norm() < MAGNETO_RADIUS {
                let vel = 0.3 * (pos3d - collectable_position).normalize();
                iso.0.translation.vector += vel;
            }
            if (pos3d - collectable_position).norm() < COLLECT_RADIUS {
                let mut rng = thread_rng();
                if let Some(coin) = coins.get(entity) {
                    let coin_number = rng.gen_range(0, 2);
                    let coin_sound = if coin_number == 0 {
                        preloaded_sounds.coin
                    } else {
                        preloaded_sounds.coin2
                    };
                    sounds_channel.single_write(Sound(
                            coin_sound,
                            Point2::new(collectable_position.x, collectable_position.y)
                        )
                    );
                    progress.add_coins(coin.0);
                    progress.add_score(coin.0);
                    macro_game.coins += coin.0;
                }
                if let Some(exp) = exps.get(entity) {
                    sounds_channel.single_write(
                        Sound(
                            preloaded_sounds.exp,
                            Point2::new(collectable_position.x, collectable_position.y)
                        )
                    );
                    progress.add_score(3 * exp.0);
                    progress.add_exp(exp.0);
                }
                if let Some(health) = healths.get(entity) {
                    lifes.get_mut(char_entity).unwrap().0 += health.0;
                    // dbg!("wow");
                }
                if side_bullet_collectables.get(entity).is_some() {
                    insert_channel.single_write(InsertEvent::SideBulletAbility);
                    if let Some(gun) = shotguns.get_mut(char_entity) {
                        gun.side_projectiles_number += 1;
                    }
                    if let Some(multy_lazer) = multiple_lazers.get_mut(char_entity) {
                        multy_lazer.plus_side_lazers();
                    }
                }
                if double_coins_collectable.get(entity).is_some() {
                    insert_channel.single_write(InsertEvent::DoubleCoinsAbility)
                }
                if double_exp_collectable.get(entity).is_some() {
                    insert_channel.single_write(InsertEvent::DoubleExpAbility)
                }
                entities.delete(entity).unwrap();
            }
        }
        let cnt = asteroid_markers.count();
        let add_cnt = if ASTEROIDS_MIN_NUMBER > cnt {
            ASTEROIDS_MIN_NUMBER - cnt
        } else {
            0
        };
        for _ in 0..add_cnt {
            let mut rng = thread_rng();
            let size = rng.gen_range(ASTEROID_MIN_RADIUS, ASTEROID_MAX_RADIUS);
            let r = size;
            let poly = generate_convex_polygon(10, r);
            let spin = rng.gen_range(-1E-2, 1E-2);
            // let ball = ncollide2d::shape::Ball::new(r);
            let spawn_pos = spawn_position(character_position, PLAYER_AREA, ACTIVE_AREA);
            insert_channel.single_write(InsertEvent::Asteroid {
                iso: Point3::new(
                    spawn_pos.x,
                    spawn_pos.y,
                    0.0,
                ),
                velocity: initial_asteroid_velocity(),
                polygon: poly,
                spin: spin,
            });
        }
        let cnt = ships.count();
        let wave = &waves.0[current_wave.id];
        let (add_cnt, const_spawn) = if cnt == 1 {
            current_wave.iteration += 1;
            (
                wave.ships_number - cnt + 1,
                true
            )
        } else {
            (
                0,
                false
            )
        };
        if current_wave.iteration > wave.iterations {
            current_wave.iteration = 0;
            current_wave.id = (waves.0.len() - 1).min(current_wave.id + 1);
        }
        let mut rng = thread_rng();
        fn ships2insert(
            spawn_pos: Point2,
            enemy: EnemyKind
        ) -> InsertEvent {
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
                let spawn_pos = spawn_position(character_position, PLAYER_AREA, ACTIVE_AREA);
                // TODO move from loop 
                let ships = &description.enemies;
                let ship_id = wave.distribution.choose_weighted(&mut rng, |item| item.1).unwrap().0;
                insert_channel.single_write(ships2insert(spawn_pos, ships[ship_id].clone()));
            }
        };
        if const_spawn {
            for kind in wave.const_distribution.iter() {
                // dbg!(kind);
                for _ in 0..kind.1 {
                    let spawn_pos = spawn_position(character_position, PLAYER_AREA, ACTIVE_AREA);
                    let ships = &description.enemies;
                    let ship_id = kind.0;
                    insert_channel.single_write(ships2insert(spawn_pos, ships[ship_id].clone()));
                }
            }
        }
        // TOOOOOO MANY COOPY PASTE %-P
        big_star_grid.grid.reset();
        for (isometry, _star) in (&isometries, &big_star_markers).join() {
            let position = isometry.0.translation.vector;
            let point = Point2::new(position.x, position.y);
            match big_star_grid.grid.update(point, true) {
                Ok(_) => (),
                Err(_) => ()
            }
        }

        for i in 0..big_star_grid.grid.size {
            for j in 0..big_star_grid.grid.size {
                let value = *big_star_grid.grid.get_cell_value(i, j);
                if !value {
                    let ((min_w, max_w), (min_h, max_h)) = big_star_grid.grid.get_rectangle(i, j);
                    let spawn_pos = spawn_in_rectangle(min_w, max_w, min_h, max_h);
                    let mut rng = thread_rng();
                    let angle = rng.gen_range(0.0, 2.0 * std::f32::consts::PI);
                    insert_channel.single_write(InsertEvent::BigStar {
                        iso: Point3::new(spawn_pos.x, spawn_pos.y, angle)
                    })
                }
            }
        }

        for (entity, isometry, _star) in (&entities, &isometries, &big_star_markers).join() {
            let pos3d = isometry.0.translation.vector;
            if  (pos3d.x - character_position.x).abs() > big_star_grid.grid.max_w ||
                (pos3d.y - character_position.y).abs() > big_star_grid.grid.max_h {
                entities.delete(entity).unwrap();
            }
        }        

        // TOOOOOO MANY COOPY PASTE %-P
        stars_grid.grid.reset();
        for (isometry, _stars) in (&isometries, &stars).join() {
            let position = isometry.0.translation.vector;
            let point = Point2::new(position.x, position.y);
            match stars_grid.grid.update(point, true) {
                Ok(_) => (),
                Err(_) => ()
            }
        }

        for i in 0..stars_grid.grid.size {
            for j in 0..stars_grid.grid.size {
                let value = *stars_grid.grid.get_cell_value(i, j);
                if !value {
                    let ((min_w, max_w), (min_h, max_h)) = stars_grid.grid.get_rectangle(i, j);
                    let spawn_pos = spawn_in_rectangle(min_w, max_w, min_h, max_h);
                    let mut rng = thread_rng();
                    let angle = rng.gen_range(0.0, 2.0 * std::f32::consts::PI);
                    insert_channel.single_write(InsertEvent::Stars {
                        iso: Point3::new(spawn_pos.x, spawn_pos.y, angle)
                    })
                }
            }
        }

        for (entity, isometry, _stars) in (&entities, &isometries, &stars).join() {
            let pos3d = isometry.0.translation.vector;
            if  (pos3d.x - character_position.x).abs() > stars_grid.grid.max_w ||
                (pos3d.y - character_position.y).abs() > stars_grid.grid.max_h {
                entities.delete(entity).unwrap();
            }
        }

        // TOOOOOO MANY COOPY PASTE %-P
        planet_grid.grid.reset();
        for (isometry, _planet) in (&isometries, &planets).join() {
            let position = isometry.0.translation.vector;
            let point = Point2::new(position.x, position.y);
            match planet_grid.grid.update(point, true) {
                Ok(_) => (),
                Err(_) => ()
            }
        }

        for i in 0..planet_grid.grid.size {
            for j in 0..planet_grid.grid.size {
                let value = *planet_grid.grid.get_cell_value(i, j);
                if !value {
                    let ((min_w, max_w), (min_h, max_h)) = planet_grid.grid.get_rectangle(i, j);
                    let spawn_pos = spawn_in_rectangle(min_w, max_w, min_h, max_h);
                    let mut rng = thread_rng();
                    let angle = rng.gen_range(0.0, 2.0 * std::f32::consts::PI);
                    insert_channel.single_write(InsertEvent::Planet {
                        iso: Point3::new(spawn_pos.x, spawn_pos.y, angle)
                    })
                }
            }
        }

        for (entity, isometry, _planet) in (&entities, &isometries, &planets).join() {
            let pos3d = isometry.0.translation.vector;
            if  (pos3d.x - character_position.x).abs() > planet_grid.grid.max_w ||
                (pos3d.y - character_position.y).abs() > planet_grid.grid.max_h {
                entities.delete(entity).unwrap();
            }
        }

        // TOOOOOO MANY COOPY PASTE %-P
        nebula_grid.grid.reset();
        for (isometry, _nebula) in (&isometries, &nebulas).join() {
            let position = isometry.0.translation.vector;
            let point = Point2::new(position.x, position.y);
            match nebula_grid.grid.update(point, true) {
                Ok(_) => (),
                Err(_) => ()
            }
        }
        for i in 0..nebula_grid.grid.size {
            for j in 0..nebula_grid.grid.size {
                let value = *nebula_grid.grid.get_cell_value(i, j);
                if !value {
                    let ((min_w, max_w), (min_h, max_h)) = nebula_grid.grid.get_rectangle(i, j);
                    let spawn_pos = spawn_in_rectangle(min_w, max_w, min_h, max_h);
                    insert_channel.single_write(InsertEvent::Nebula {
                        iso: Point3::new(spawn_pos.x, spawn_pos.y, 0f32)
                    })
                }
            }
        }

        for (entity, isometry, _nebula) in (&entities, &isometries, &nebulas).join() {
            let pos3d = isometry.0.translation.vector;
            if  (pos3d.x - character_position.x).abs() > nebula_grid.grid.max_w ||
                (pos3d.y - character_position.y).abs() > nebula_grid.grid.max_h {
                entities.delete(entity).unwrap();
            }
        }
        for (entity, isometry, _asteroid) in (&entities, &isometries, &asteroid_markers).join() {
            let pos3d = isometry.0.translation.vector;
            if !is_active(character_position, Point2::new(pos3d.x, pos3d.y), ACTIVE_AREA) {
                entities.delete(entity).unwrap();
            }
        }
        for (entity, isometry, _ship) in (&entities, &isometries, &ships).join() {
            let pos3d = isometry.0.translation.vector;
            if !is_active(character_position, Point2::new(pos3d.x, pos3d.y), ACTIVE_AREA) {
                entities.delete(entity).unwrap();
            }
        }
        info!("asteroids: gameplay ended");
    }
}
