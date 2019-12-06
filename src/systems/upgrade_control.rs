use sdl2::keyboard::Keycode;
use std::collections::HashSet;
use std::mem::swap;

pub use super::*;

#[derive(Debug, Default)]
pub struct UpgradeControlSystem {
    prev_keys: HashSet<Keycode>,
    new_keys: HashSet<Keycode>,
}

impl<'a> System<'a> for UpgradeControlSystem {
	type SystemData = (
		Write<'a, Vec<Keycode>>,
        Write<'a, UI>,
	    Write<'a, SpawnedUpgrades>,
        WriteExpect<'a, UIState>,
        Read<'a, AvaliableUpgrades>,
	    WriteExpect<'a, Vec<UpgradeType>>,
	    Write<'a, AppState>,
	);

    fn run(&mut self, data: Self::SystemData) {
    	let (
    		mut key_codes,
    		mut ui,
    		mut spawned_upgrades,
    		mut ui_state,
    		avaliable_upgrades,
    		mut upgrade_types,
    		mut app_state
    	) = data;
    	let upgrades = spawned_upgrades.last().map(|x| x.clone());
        let widget_ids = [Widgets::Upgrade1, Widgets::Upgrade2];
        let widget_selector = Widgets::UpgradeSelector as usize;
    	swap(&mut self.prev_keys, &mut self.new_keys);
    	self.new_keys.clear();
    	for key in key_codes.drain(..) {
    		self.new_keys.insert(key);
    		// do something here
    	}
    	let new_pressed = &self.new_keys - &self.prev_keys;
    	for key in new_pressed.iter() {
    		match key {
                Keycode::Left | Keycode::Right => {
			    	if let Some(upgrades) = upgrades {
	                	if ui.selected(widget_selector, widget_ids[0] as usize) 
	                			|| ui.free_selector(widget_selector) {
							ui_state.choosed_upgrade = Some(upgrades[1]);
	                		ui.select(widget_selector, widget_ids[1] as usize)
	                	} else if ui.selected(widget_selector, widget_ids[1] as usize) {
							ui_state.choosed_upgrade = Some(upgrades[0]);
	                		ui.select(widget_selector, widget_ids[0] as usize)
	                	}
			    	}
                }
                Keycode::Space => {
    	            if let Some(upgrade) = ui_state.choosed_upgrade {
	                    ui_state.choosed_upgrade = None;
	                    spawned_upgrades.pop();
			            upgrade_types.push(avaliable_upgrades[upgrade].upgrade_type);
    	            } else {
        	            *app_state = AppState::Play(PlayState::Action);
    	            }
                }
                // Keycode::Right => {
                // 	dbg!("sd");
                // }
                _ => ()
    		}
    	}    		
    }
}