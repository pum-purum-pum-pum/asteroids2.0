use gfx_h::MenuParticles;
use gfx_h::TextData;
use std::collections::HashMap;
use std::convert::TryFrom;
use super::*;
use super::rendering::*;

pub struct MenuRenderingSystem {
    reader: ReaderId<Primitive>,
}

impl MenuRenderingSystem {
    pub fn new(reader: ReaderId<Primitive>) -> Self {
        MenuRenderingSystem{
            reader: reader
        }
    }
}

impl<'a> System<'a> for MenuRenderingSystem {
    type SystemData = (
        ReadStorage<'a, ThreadPin<ImageData>>,
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
        WriteExpect<'a, ThreadPin<TextData<'static>>>,
        ReadExpect<'a, Description>,
        Read<'a, Vec<UpgradeCardRaw>>,
        Write<'a, Vec<UpgradeCard>>,
        Read<'a, HashMap<String, specs::Entity>>,
        WriteExpect<'a, MacroGame>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            image_datas,
            gl,
            mut canvas,
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
            mut text_data,
            description,
            upgrade_cards_raw,
            mut avaliable_upgrades,
            name_to_image,
            mut macro_game
        ) = data;
        let mut frame = red::Frame::new(&gl);
        frame.set_clear_color(0.0, 0.0, 0.0, 1.0);
        frame.clear_color();
        let dims = viewport.dimensions();
        let (w, h) = (dims.0 as f32, dims.1 as f32);
        // return;

        ui.primitives.push(
            Primitive {
                kind: PrimitiveKind::Text(Text {
                    position: Point2::new(w - w/7.0, h / 20.0), 
                    text: format!(
                        "$ {}", 
                        macro_game.coins
                    ).to_string()
                }),
                with_projection: false,
            }
        );

        let button_w = w/12f32;
        let button_h = button_w;
        let mut buttons = vec![];
        let buttons_names = vec!["", "", ""];
        let guns = vec![Widgets::LazerGun, Widgets::BlasterGun, Widgets::ShotGun];
        let buttons_num = buttons_names.len();
        let button_images = vec![
            preloaded_images.lazer, 
            preloaded_images.blaster, 
            preloaded_images.shotgun
        ];
        let shift_between = w / 20f32;
        let shift_init = w / 2.0 - shift_between - button_w - button_w / 2.0; 
                // -button_w / 2.0 since start draw from left corner :)
        for i in 0..buttons_num {
            let button = Button::new(
                Point2::new(
                    shift_init + i as f32 * (shift_between + button_w), 
                    button_h / 2f32
                ),
                button_w,
                button_h,
                None,
                false,
                Some(Image(button_images[i])),
                buttons_names[i].to_string(),
                guns[i] as usize
            );
            buttons.push(button);
        }
        let weapon_selector = Selector {
            buttons: buttons,
            id: Widgets::WeaponSelector as usize,
            mask: None
        };
        if let Some(selected_id) = weapon_selector.place_and_check(
            &mut ui,
            &*mouse
        ) {
            match Widgets::try_from(selected_id).expect("unknown widget id") {
                Widgets::LazerGun => {
                    ui_state.chosed_gun = Some(description.player_guns[0].clone());
                }
                Widgets::BlasterGun => {
                    ui_state.chosed_gun = Some(description.player_guns[1].clone());
                }
                Widgets::ShotGun => {
                    ui_state.chosed_gun = Some(description.player_guns[2].clone());
                }
                _ => ()
            }
        }
        let mut buttons = vec![];
        let ships_ids = vec![Widgets::BasicShip, Widgets::HeavyShip];
        let locked_ships_ids = vec![Widgets::LockedBasicShip, Widgets::LockedHeavyShip];
        for (i, ship) in description.player_ships.iter().enumerate() {
            let unlocked = macro_game.ships_unlocked[i];
            let button_position = 
                Point2::new(
                    shift_init + i as f32 * (shift_between + button_w), 
                    button_h + button_h
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
                );
                buttons.push(button);
            } else {
                let button = Button::new(
                    button_position,
                    button_w,
                    button_h,
                    None,
                    false,
                    Some(ship.image),
                    format!("{} $", description.ship_costs[i]),
                    locked_ships_ids[i] as usize,
                );
                buttons.push(button);
            }
        }

        let ships_selector = Selector {
            buttons: buttons,
            id: Widgets::ShipsSelector as usize,
            mask: Some(macro_game.ships_unlocked.clone())
        };
        if let Some(selected_id) = ships_selector.place_and_check(
            &mut ui,
            &* mouse
        ) {
            // ui_state.chosed_ship = Some(selected_id);
            match Widgets::try_from(selected_id).expect("unknown widget id") {
                Widgets::BasicShip => {
                    ui_state.chosed_ship = Some(0);
                }
                Widgets::HeavyShip => {
                    ui_state.chosed_ship = Some(1);
                }
                Widgets::LockedHeavyShip => {
                    if macro_game.coins >= description.ship_costs[1] {
                        macro_game.ships_unlocked[1] = true;
                        macro_game.coins -= description.ship_costs[1];
                    }
                }
                _ => ()
            }
        }
        // for i in 0..buttons.len() {
        //     if buttons[i].place_and_check(&mut ui, &*mouse) {
        //         dbg!(&format!("ship button {}", i));
        //     }
        // }

        let button_w = w / 6.0;
        let button_h = button_w;
        let score_table_button = Button::new(
            Point2::new(w / 2.0, 1.5 * button_h + shift_between),
            button_w,
            button_h / 5.0,
            None,
            false,
            Some(Image(preloaded_images.upg_bar)),
            "Score Table".to_string(),
            Widgets::ScoreTable as usize
        );
        if score_table_button.place_and_check(&mut ui, &*mouse) {
            *app_state = AppState::ScoreTable;
        }
        menu_particles.update(0.5);
        canvas
            .render_instancing(
                &gl,
                &viewport,
                &mut frame,
                &menu_particles.instancing_data,
                &Isometry3::new(
                    Vector3::new(0f32, 0f32, 0f32),
                    Vector3::new(0f32, 0f32, 0f32),
                )
            );
        let button_w = button_w / 2.0;
        let button_h = button_w;
        let button = Button::new(
            Point2::new(w/2.0 - button_w / 2.0, h - button_h), 
            // Point2::new(0f32, 0f32),
            button_w, 
            button_h, 
            None,
            false, 
            Some(Image(preloaded_images.play)),
            "".to_string(),
            Widgets::Play as usize
        );
        if let (Some(ship), Some(gun)) = (ui_state.chosed_ship.clone(), ui_state.chosed_gun.clone()) {
            if button.place_and_check(&mut ui, &*mouse) {
                *app_state = AppState::Play(PlayState::Action);
                insert_channel.single_write(InsertEvent::Character{ 
                    gun_kind: gun.clone(), 
                    ship_stats: description.player_ships[ship].ship_stats
                });
                // ui_state.chosed_gun = None;
                *avaliable_upgrades = get_avaliable_cards(
                    &upgrade_cards_raw,
                    &gun.clone(),
                    &name_to_image
                );
            }
        }
        primitives_channel.iter_write(ui.primitives.drain(..));
        // render_primitives(
        //     &mouse,
        //     &mut self.reader,
        //     &mut frame,
        //     &image_datas,
        //     &gl,
        //     &mut canvas,
        //     &viewport,
        //     &mut primitives_channel,
        //     &mut text_data,
        // );
    }
}
