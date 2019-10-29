use super::rendering::*;
use super::*;
use gfx_h::MenuParticles;
use std::collections::HashMap;
use std::convert::TryFrom;

pub struct MenuRenderingSystem;

impl<'a> System<'a> for MenuRenderingSystem {
    type SystemData = (
        ReadExpect<'a, ThreadPin<red::GL>>,
        WriteExpect<'a, Canvas>,
        ReadExpect<'a, red::Viewport>,
        Write<'a, EventChannel<Primitive>>,
        Write<'a, UI>,
        Write<'a, UIState>,
        Write<'a, EventChannel<InsertEvent>>,
        WriteExpect<'a, PreloadedImages>,
        WriteExpect<'a, ThreadPin<MenuParticles>>,
        Read<'a, Mouse>,
        Write<'a, AppState>,
        ReadExpect<'a, Description>,
        Read<'a, Vec<UpgradeCardRaw>>,
        Write<'a, Vec<UpgradeCard>>,
        Read<'a, HashMap<String, AtlasImage>>,
        WriteExpect<'a, MacroGame>,
        Write<'a, EventChannel<Sound>>,
        ReadExpect<'a, PreloadedSounds>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            gl,
            canvas,
            viewport,
            mut primitives_channel,
            mut ui,
            mut ui_state,
            mut insert_channel,
            preloaded_images,
            mut menu_particles,
            mouse,
            mut app_state,
            // text_data
            description,
            upgrade_cards_raw,
            mut avaliable_upgrades,
            name_to_atlas,
            mut macro_game,
            mut sounds_channel,
            preloaded_sounds,
        ) = data;
        let mut frame = red::Frame::new(&gl);
        // frame.set_clear_color(0.0, 0.0, 0.0, 1.0);
        // frame.clear_color();
        let dims = viewport.dimensions();
        let (w, h) = (dims.0 as f32, dims.1 as f32);
        // return;

        // darker background
        ui.primitives.push(Primitive {
            kind: PrimitiveKind::Picture(Picture {
                position: Point2::new(0f32, 0f32),
                width: w,
                height: h,
                image: preloaded_images.transparent_sqr,
            }),
            with_projection: false,
        });
        ui.primitives.push(Primitive {
            kind: PrimitiveKind::Text(Text {
                position: Point2::new(w - w / 7.0, h / 20.0),
                color: (1.0, 1.0, 1.0, 1.0),
                text: format!("$ {}", macro_game.coins).to_string(),
                font_size: 1.0
            }),
            with_projection: false,
        });

        let button_w = w / 12f32;
        let button_h = button_w;
        let mut buttons = vec![];
        let buttons_names = vec!["", ""];
        let guns = vec![Widgets::BlasterGun, Widgets::LazerGun];
        let locked_guns_ids =
            vec![Widgets::LockedBlasterGun, Widgets::LockedLazerGun];
        let buttons_num = buttons_names.len();
        let button_images = vec![
            preloaded_images.blaster,
            preloaded_images.lazer,
            preloaded_images.shotgun,
        ];
        let shift_between = w / 20f32;
        let shift_init = w / 2.0 - shift_between - button_w - button_w / 2.0;
        // -button_w / 2.0 since start draw from left corner :)
        for i in 0..buttons_num {
            let unlocked = macro_game.guns_unlocked[i];
            let button_position = Point2::new(
                shift_init + i as f32 * (shift_between + button_w),
                button_h / 2f32,
            );
            if unlocked {
                let button = Button::new(
                    button_position,
                    button_w,
                    button_h,
                    None,
                    false,
                    Some(button_images[i]),
                    buttons_names[i].to_string(),
                    guns[i] as usize,
                    Some(Sound(
                        preloaded_sounds.hover,
                        Point2::new(0f32, 0f32),
                    )),
                    Some(Sound(
                        preloaded_sounds.click,
                        Point2::new(0f32, 0f32),
                    )),
                );
                buttons.push(button);
            } else {
                let button = Button::new(
                    button_position,
                    button_w,
                    button_h,
                    None,
                    false,
                    Some(preloaded_images.locked),
                    format!("{} $", description.gun_costs[i]),
                    locked_guns_ids[i] as usize,
                    Some(Sound(
                        preloaded_sounds.hover,
                        Point2::new(0f32, 0f32),
                    )),
                    Some(Sound(preloaded_sounds.deny, Point2::new(0f32, 0f32))),
                );
                buttons.push(button);
            }
        }
        let weapon_selector = Selector {
            buttons: buttons,
            id: Widgets::WeaponSelector as usize,
            mask: Some(macro_game.guns_unlocked.clone()),
        };
        if let Some(selected_id) =
            weapon_selector.place_and_check(&mut ui, &*mouse)
        {
            match Widgets::try_from(selected_id).expect("unknown widget id") {
                Widgets::BlasterGun => {
                    ui_state.chosed_gun =
                        Some(description.player_guns[0].clone());
                    // sounds_channel.single_write(Sound(
                    //         coin_sound,
                    //         Point2::new(collectable_position.x, collectable_position.y)
                    //     )
                    // );
                }
                Widgets::LazerGun => {
                    ui_state.chosed_gun =
                        Some(description.player_guns[1].clone());
                }
                Widgets::ShotGun => {
                    ui_state.chosed_gun =
                        Some(description.player_guns[2].clone());
                }
                Widgets::LockedLazerGun => {
                    if macro_game.coins >= description.gun_costs[1] {
                        macro_game.guns_unlocked[1] = true;
                        sounds_channel.single_write(Sound(
                            preloaded_sounds.buy,
                            Point2::new(0f32, 0f32),
                        ));
                        macro_game.coins -= description.gun_costs[1];
                        ui_state.chosed_gun =
                            Some(description.player_guns[1].clone());
                    }
                }
                _ => (),
            }
        }
        let mut buttons = vec![];
        let ships_ids =
            vec![Widgets::BasicShip, Widgets::HeavyShip, Widgets::SuperShip];
        let ship_images = vec![
            preloaded_images.basic_ship,
            preloaded_images.heavy_ship,
            preloaded_images.super_ship,
        ];
        let locked_ships_ids = vec![
            Widgets::LockedBasicShip,
            Widgets::LockedHeavyShip,
            Widgets::LockedSuperShip,
        ];
        for (i, ship) in description.player_ships.iter().enumerate() {
            let unlocked = macro_game.ships_unlocked[i];
            let button_position = Point2::new(
                shift_init + i as f32 * (shift_between + button_w),
                button_h + button_h,
            );
            if unlocked {
                let button = Button::new(
                    button_position,
                    button_w,
                    button_h,
                    None,
                    false,
                    Some(ship.image),
                    "".to_string(),
                    ships_ids[i] as usize,
                    Some(Sound(
                        preloaded_sounds.hover,
                        Point2::new(0f32, 0f32),
                    )),
                    Some(Sound(
                        preloaded_sounds.click,
                        Point2::new(0f32, 0f32),
                    )),
                );
                buttons.push(button);
            } else {
                let button = Button::new(
                    button_position,
                    button_w,
                    button_h,
                    None,
                    false,
                    Some(preloaded_images.locked),
                    format!("{} $", description.ship_costs[i]),
                    locked_ships_ids[i] as usize,
                    Some(Sound(
                        preloaded_sounds.hover,
                        Point2::new(0f32, 0f32),
                    )),
                    Some(Sound(preloaded_sounds.deny, Point2::new(0f32, 0f32))),
                );
                buttons.push(button);
            }
        }

        let ships_selector = Selector {
            buttons: buttons,
            id: Widgets::ShipsSelector as usize,
            mask: Some(macro_game.ships_unlocked.clone()),
        };
        if let Some(selected_id) =
            ships_selector.place_and_check(&mut ui, &*mouse)
        {
            // ui_state.chosed_ship = Some(selected_id);
            match Widgets::try_from(selected_id).expect("unknown widget id") {
                Widgets::BasicShip => {
                    ui_state.chosed_ship = Some(0);
                }
                Widgets::HeavyShip => {
                    ui_state.chosed_ship = Some(1);
                }
                Widgets::SuperShip => {
                    ui_state.chosed_ship = Some(2);
                }
                Widgets::LockedHeavyShip => {
                    if macro_game.coins >= description.ship_costs[1] {
                        macro_game.ships_unlocked[1] = true;
                        sounds_channel.single_write(Sound(
                            preloaded_sounds.buy,
                            Point2::new(0f32, 0f32),
                        ));
                        macro_game.coins -= description.ship_costs[1];
                    }
                }
                Widgets::LockedSuperShip => {
                    if macro_game.coins >= description.ship_costs[2] {
                        macro_game.ships_unlocked[2] = true;
                        sounds_channel.single_write(Sound(
                            preloaded_sounds.buy,
                            Point2::new(0f32, 0f32),
                        ));
                        macro_game.coins -= description.ship_costs[2];
                    }
                }
                _ => (),
            }
        }
        let button_w = w / 6.0;
        let button_h = button_w;
        let score_table_button = Button::new(
            Point2::new(w / 2.0, 1.5 * button_h + shift_between),
            button_w,
            button_h / 5.0,
            None,
            false,
            Some(preloaded_images.upg_bar),
            "Score Table".to_string(),
            Widgets::ScoreTable as usize,
            Some(Sound(preloaded_sounds.hover, Point2::new(0f32, 0f32))),
            Some(Sound(preloaded_sounds.click, Point2::new(0f32, 0f32))),
        );
        if score_table_button.place_and_check(&mut ui, &*mouse) {
            *app_state = AppState::ScoreTable;
        }
        let button_w = button_w / 2.0;
        let button_h = button_w;
        let button = Button::new(
            Point2::new(w / 2.0 - button_w / 2.0, h - button_h),
            // Point2::new(0f32, 0f32),
            button_w,
            button_h / 4.0,
            None,
            false,
            Some(preloaded_images.upg_bar),
            "Play".to_string(),
            Widgets::Play as usize,
            Some(Sound(preloaded_sounds.hover, Point2::new(0f32, 0f32))),
            Some(Sound(preloaded_sounds.play, Point2::new(0f32, 0f32))),
        );
        if let (Some(ship), Some(gun)) =
            (ui_state.chosed_ship.clone(), ui_state.chosed_gun.clone())
        {
            if button.place_and_check(&mut ui, &*mouse) {
                *app_state = AppState::Play(PlayState::Action);
                insert_channel.single_write(InsertEvent::Character {
                    gun_kind: gun.clone(),
                    ship_stats: description.player_ships[ship].ship_stats,
                    image: ship_images[ship],
                });
                *avaliable_upgrades = get_avaliable_cards(
                    &upgrade_cards_raw,
                    &gun.clone(),
                    &name_to_atlas,
                );
            }
        }
        primitives_channel.iter_write(ui.primitives.drain(..));
        sounds_channel.iter_write(ui.sounds.drain(..));
    }
}
