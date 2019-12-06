pub use super::*;

#[derive(Debug, Default)]
pub struct Upgrader;

impl<'a> System<'a> for Upgrader {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, CharacterMarker>,
        WriteStorage<'a, ShipStats>,
        WriteStorage<'a, MultyLazer>,
        WriteStorage<'a, ShotGun>,
        WriteExpect<'a, Vec<UpgradeType>>,
	);

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            character_markers,
            mut ships_stats,
            mut multiple_lazers,
            mut shotguns,
            mut upgrade_types
        ) = data;
        let (character, ship_stats, _) =
            (&entities, &mut ships_stats, &character_markers)
                .join()
                .next()
                .unwrap();
        for choosed_upgrade in upgrade_types.drain(..) {
        	dbg!("upgrading");
            match choosed_upgrade {
                UpgradeType::AttackSpeed => {
                    if let Some(gun) = shotguns.get_mut(character) {
                        let recharge_time_millis =
                            (gun.recharge_time.as_millis() as f32 * 0.9) as u64;
                        gun.recharge_time =
                            Duration::from_millis(recharge_time_millis);
                    }
                }
                UpgradeType::BulletSpeed => {
                    if let Some(gun) = shotguns.get_mut(character) {
                        gun.bullet_speed += 0.1 * BULLET_SPEED_INIT;
                    }
                }
                // UpgradeType::BulletReflection => {
                //     if let Some(gun) = shotguns.get_mut(character) {
                //         if let Some(ref mut reflection) = gun.reflection {
                //             // reflection.speed += 0.5;
                //             reflection.lifetime += Duration::from_millis(200);
                //         } else {
                //             gun.reflection = Some(Reflection {
                //                 speed: 0.4,
                //                 lifetime: Duration::from_millis(1500),
                //                 times: None,
                //             })
                //         }
                //     }
                // }
                UpgradeType::LazerLength => {
                    if let Some(multy_lazer) =
                        multiple_lazers.get_mut(character)
                    {
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
                    ship_stats.max_health = ship_stats.max_health + (0.05 * ship_stats.max_health as f32) as usize;
                }
                UpgradeType::ShieldSize => {
                    ship_stats.max_shield = ship_stats.max_shield + (0.05 * ship_stats.max_shield as f32) as usize;
                }
                UpgradeType::Maneuverability => {
                    *ship_stats.maneuverability.as_mut().unwrap() += 1.0;
                }
            }
        }
    }
}