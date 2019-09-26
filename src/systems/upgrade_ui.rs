use std::convert::TryFrom;
pub use super::*;

#[derive(Default)]
pub struct UpgradeGUI;

impl<'a> System<'a> for UpgradeGUI {
    type SystemData = (
        (
            Entities<'a>,
            ReadStorage<'a, CharacterMarker>,
            WriteStorage<'a, ShipStats>,
            WriteStorage<'a, MultyLazer>,
            WriteStorage<'a, ShotGun>,
            ReadExpect<'a, red::Viewport>,
        ),
        Write<'a, UI>,
        Write<'a, AppState>,
        Read<'a, Mouse>,
        WriteExpect<'a, PreloadedImages>,
        Read<'a, AvaliableUpgrades>,
        Write<'a, SpawnedUpgrades>,
        WriteExpect<'a, UIState>,
        ReadExpect<'a, Pallete>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            (
                entities,
                character_markers,
                mut ships_stats,
                mut multiple_lazers,
                mut shotguns,
                viewport,
            ),
            // preloaded_particles,
            mut ui,
            mut app_state,
            mouse,
            preloaded_images,
            avaliable_upgrades,
            mut spawned_upgrades,
            mut ui_state,
            _pallete
        ) = data;
        let dims = viewport.dimensions();
        let (w, h) = (dims.0 as f32, dims.1 as f32);
        let (character, ship_stats, _) = (&entities, &mut ships_stats, &character_markers).join().next().unwrap();
        // upgrade UI
        let mut current_upgrade = None;
        let upgrade_button_w = (w/4f32).min(h/2f32);
        let upgrade_button_h = upgrade_button_w;
        let (choose_button_w, choose_button_h) = (w/6f32, h/10f32);
        let shift = upgrade_button_h / 10f32;
        // dark background
        ui.primitives.push(
            Primitive {
                kind: PrimitiveKind::Picture(Picture{
                    position: Point2::new(0f32, 0f32),
                    width: w, 
                    height: h,
                    image: Image(preloaded_images.transparent_sqr)
                }),
                with_projection: false,
            }
        );
        let mut buttons = vec![];
        let upgrades = spawned_upgrades.last();
        // dbg!(&upgrades);
        if let Some(upgrades) = upgrades {
            let widget_ids = [Widgets::Upgrade1, Widgets::Upgrade2];
            for (i, upg_id) in upgrades.iter().enumerate() {
                let upg = &avaliable_upgrades[*upg_id];
                let current_point = 
                    Point2::new(
                        w / 2.0 - upgrade_button_w - shift 
                        + i as f32 * (upgrade_button_w + shift), 
                        shift
                    );
                let upgrade_button = Button::new(
                    current_point,
                    upgrade_button_w, upgrade_button_h, 
                    None,
                    false,
                    Some(upg.image),
                    "".to_string(),
                    widget_ids[i] as usize,
                    None,
                    None,
                );
                ui.primitives.push(
                    Primitive {
                        kind: PrimitiveKind::Text(Text {
                            position: Point2::new(current_point.x + upgrade_button_h / 2.0, upgrade_button_h + 2.0 * shift),
                            text: upg.name.clone()
                        }),
                        with_projection: false,
                    }
                );
                buttons.push(upgrade_button);
            }
            let upgrade_selector = Selector {
                buttons: buttons,
                id: Widgets::WeaponSelector as usize,
                mask: None,
            };
            if let Some(selected_id) = upgrade_selector.place_and_check(
                &mut ui,
                &*mouse
            ) {
                match Widgets::try_from(selected_id).expect("unknown widget id") {
                    Widgets::Upgrade1 => {
                        ui_state.choosed_upgrade = Some(upgrades[0]);
                    }
                    Widgets::Upgrade2 => {
                        ui_state.choosed_upgrade = Some(upgrades[1]);
                    }
                    _ => ()
                }
            }
        }
        let select_upgrade = Button::new(
            Point2::new(w / 2.0 - choose_button_w - shift, h - 1.0 * choose_button_h),
            choose_button_w, choose_button_h, 
            None,
            false,
            Some(Image(preloaded_images.upg_bar)),
            "Upgrade!".to_string(),
            Widgets::Upgrade as usize,
            None,
            None,
        );

        if spawned_upgrades.len() > 0 {
            if let Some(upgrade) = ui_state.choosed_upgrade {
                ui.primitives.push(
                    Primitive {
                        kind: PrimitiveKind::Text(Text {
                            position: Point2::new(w / 2.0, upgrade_button_h + 4.0 * shift),
                            text: avaliable_upgrades[upgrade].description.clone()
                        }),
                        with_projection: false,
                    }
                );
                if select_upgrade.place_and_check(&mut ui, &*mouse) {
                    current_upgrade = Some(avaliable_upgrades[upgrade].upgrade_type);
                    ui_state.choosed_upgrade = None;
                    spawned_upgrades.pop();
                }
            }
        }
        let done_button = Button::new(
            Point2::new(w / 2.0 + shift, h - 1.0 * choose_button_h),
            choose_button_w, choose_button_h, 
            None,
            false,
            Some(Image(preloaded_images.upg_bar)),
            "Done".to_string(),
            Widgets::Done as usize,
            None,
            None,
        );
        if done_button.place_and_check(&mut ui, &*mouse) {
            *app_state = AppState::Play(PlayState::Action);
        }

        match current_upgrade {
            Some(choosed_upgrade) => {
                match choosed_upgrade {
                    UpgradeType::AttackSpeed => {
                        match shotguns.get_mut(character) {
                            Some(gun) => {
                                let recharge_time_millis = (gun.recharge_time.as_millis() as f32 * 0.9) as u64;
                                gun.recharge_time = Duration::from_millis(recharge_time_millis);
                            }
                            None => ()
                        }
                    }
                    UpgradeType::BulletSpeed => {
                        match shotguns.get_mut(character) {
                            Some(gun) => {
                                gun.bullet_speed += 0.1 * BULLET_SPEED_INIT;
                            }
                            None => ()
                        }
                    }
                    UpgradeType::BulletReflection => {
                        if let Some(gun) = shotguns.get_mut(character) {
                            if let Some(ref mut reflection) = gun.reflection {
                                // reflection.speed += 0.5;
                                reflection.lifetime += Duration::from_millis(200);
                            } else {
                                gun.reflection = Some(Reflection{speed: 0.4, lifetime: Duration::from_millis(1500), times: None})
                            }
                        }
                    }
                    UpgradeType::LazerLength => {
                        if let Some(multy_lazer) = multiple_lazers.get_mut(character) {
                            multy_lazer.upgrade_length(0.3);
                        }
                    }                    
                    UpgradeType::ShipSpeed => {
                        ship_stats.thrust_force += 0.1 * THRUST_FORCE_INIT;
                    }
                    UpgradeType::ShipRotationSpeed => {
                        ship_stats.torque += 0.1 * SHIP_ROTATION_SPEED_INIT;
                    }

                    UpgradeType::ShieldRegen => {
                        ship_stats.shield_regen += 1;
                    }
                    UpgradeType::HealthSize => {
                        ship_stats.max_health += 20;
                    }
                    UpgradeType::ShieldSize => {
                        ship_stats.max_shield += 20;
                    }
                    UpgradeType::Maneuverability => {
                        *ship_stats.maneuverability.as_mut().unwrap() += 1.0;
                    }
                }
            }
            None => ()
        }
    }
}
