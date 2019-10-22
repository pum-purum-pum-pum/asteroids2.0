#[derive(Default, Clone)]
pub struct CommonRespawn;
pub use super::*;

impl<'a> System<'a> for CommonRespawn {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Isometry>,
        WriteStorage<'a, AsteroidMarker>,
        ReadStorage<'a, CharacterMarker>,
        ReadStorage<'a, ShipMarker>,
        WriteStorage<'a, StarsMarker>,
        WriteStorage<'a, FogMarker>,
        WriteStorage<'a, NebulaMarker>,
        WriteStorage<'a, PlanetMarker>,
        Write<'a, EventChannel<InsertEvent>>,
        WriteExpect<'a, FogGrid>,
        WriteExpect<'a, StarsGrid>,
        WriteExpect<'a, NebulaGrid>,
        WriteExpect<'a, PlanetGrid>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            isometries,
            asteroid_markers,
            character_markers,
            ships,
            stars,
            big_star_markers,
            nebulas,
            planets,
            mut insert_channel,
            mut big_star_grid,
            mut stars_grid,
            mut nebula_grid,
            mut planet_grid,
        ) = data;
        let character_position =
            if let Some((_char_entity, char_isometry, _char)) =
                (&entities, &isometries, &character_markers).join().next()
            {
                let char_vec = char_isometry.0.translation.vector;
                Point2::new(char_vec.x, char_vec.y)
            } else {
                Point2::new(0.0, 0.0)
            };

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
            let spawn_pos =
                spawn_position(character_position, PLAYER_AREA, ACTIVE_AREA);
            insert_channel.single_write(InsertEvent::Asteroid {
                iso: Point3::new(spawn_pos.x, spawn_pos.y, 0.0),
                velocity: initial_asteroid_velocity(),
                polygon: poly,
                spin: spin,
            });
        }

        // TOOOOOO MANY COOPY PASTE %-P
        big_star_grid.grid.reset();
        for (isometry, _star) in (&isometries, &big_star_markers).join() {
            let position = isometry.0.translation.vector;
            let point = Point2::new(position.x, position.y);
            match big_star_grid.grid.update(point, true) {
                Ok(_) => (),
                Err(_) => (),
            }
        }

        for i in 0..big_star_grid.grid.size {
            for j in 0..big_star_grid.grid.size {
                let value = *big_star_grid.grid.get_cell_value(i, j);
                if !value {
                    let ((min_w, max_w), (min_h, max_h)) =
                        big_star_grid.grid.get_rectangle(i, j);
                    let spawn_pos =
                        spawn_in_rectangle(min_w, max_w, min_h, max_h);
                    let mut rng = thread_rng();
                    let angle = rng.gen_range(0.0, 2.0 * std::f32::consts::PI);
                    insert_channel.single_write(InsertEvent::Fog {
                        iso: Point3::new(spawn_pos.x, spawn_pos.y, angle),
                    })
                }
            }
        }

        for (entity, isometry, _star) in
            (&entities, &isometries, &big_star_markers).join()
        {
            let pos3d = isometry.0.translation.vector;
            if (pos3d.x - character_position.x).abs() > big_star_grid.grid.max_w
                || (pos3d.y - character_position.y).abs()
                    > big_star_grid.grid.max_h
            {
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
                Err(_) => (),
            }
        }

        for i in 0..stars_grid.grid.size {
            for j in 0..stars_grid.grid.size {
                let value = *stars_grid.grid.get_cell_value(i, j);
                if !value {
                    let ((min_w, max_w), (min_h, max_h)) =
                        stars_grid.grid.get_rectangle(i, j);
                    let spawn_pos =
                        spawn_in_rectangle(min_w, max_w, min_h, max_h);
                    let mut rng = thread_rng();
                    let angle = rng.gen_range(0.0, 2.0 * std::f32::consts::PI);
                    insert_channel.single_write(InsertEvent::Stars {
                        iso: Point3::new(spawn_pos.x, spawn_pos.y, angle),
                    })
                }
            }
        }

        for (entity, isometry, _stars) in
            (&entities, &isometries, &stars).join()
        {
            let pos3d = isometry.0.translation.vector;
            if (pos3d.x - character_position.x).abs() > stars_grid.grid.max_w
                || (pos3d.y - character_position.y).abs()
                    > stars_grid.grid.max_h
            {
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
                Err(_) => (),
            }
        }

        for i in 0..planet_grid.grid.size {
            for j in 0..planet_grid.grid.size {
                let value = *planet_grid.grid.get_cell_value(i, j);
                if !value {
                    let ((min_w, max_w), (min_h, max_h)) =
                        planet_grid.grid.get_rectangle(i, j);
                    let spawn_pos =
                        spawn_in_rectangle(min_w, max_w, min_h, max_h);
                    let mut rng = thread_rng();
                    let angle = rng.gen_range(0.0, 2.0 * std::f32::consts::PI);
                    insert_channel.single_write(InsertEvent::Planet {
                        iso: Point3::new(spawn_pos.x, spawn_pos.y, angle),
                    })
                }
            }
        }

        for (entity, isometry, _planet) in
            (&entities, &isometries, &planets).join()
        {
            let pos3d = isometry.0.translation.vector;
            if (pos3d.x - character_position.x).abs() > planet_grid.grid.max_w
                || (pos3d.y - character_position.y).abs()
                    > planet_grid.grid.max_h
            {
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
                Err(_) => (),
            }
        }
        for i in 0..nebula_grid.grid.size {
            for j in 0..nebula_grid.grid.size {
                let value = *nebula_grid.grid.get_cell_value(i, j);
                if !value {
                    let ((min_w, max_w), (min_h, max_h)) =
                        nebula_grid.grid.get_rectangle(i, j);
                    let spawn_pos =
                        spawn_in_rectangle(min_w, max_w, min_h, max_h);
                    insert_channel.single_write(InsertEvent::Nebula {
                        iso: Point3::new(spawn_pos.x, spawn_pos.y, 0f32),
                    })
                }
            }
        }

        for (entity, isometry, _nebula) in
            (&entities, &isometries, &nebulas).join()
        {
            let pos3d = isometry.0.translation.vector;
            if (pos3d.x - character_position.x).abs() > nebula_grid.grid.max_w
                || (pos3d.y - character_position.y).abs()
                    > nebula_grid.grid.max_h
            {
                entities.delete(entity).unwrap();
            }
        }
        for (entity, isometry, _asteroid) in
            (&entities, &isometries, &asteroid_markers).join()
        {
            let pos3d = isometry.0.translation.vector;
            if !is_active(
                character_position,
                Point2::new(pos3d.x, pos3d.y),
                ACTIVE_AREA,
            ) {
                entities.delete(entity).unwrap();
            }
        }
        for (entity, isometry, _ship) in (&entities, &isometries, &ships).join()
        {
            let pos3d = isometry.0.translation.vector;
            if !is_active(
                character_position,
                Point2::new(pos3d.x, pos3d.y),
                ACTIVE_AREA,
            ) {
                entities.delete(entity).unwrap();
            }
        }
        dbg!((&entities).join().count());
    }
}
