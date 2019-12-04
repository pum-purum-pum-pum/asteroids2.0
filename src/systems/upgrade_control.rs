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
	);

    fn run(&mut self, data: Self::SystemData) {
    	let (
    		mut key_codes,
    		mut ui,
    		spawned_upgrades,
    		mut ui_state
    	) = data;
    	let upgrades = spawned_upgrades.last();
        let widget_ids = [Widgets::Upgrade1, Widgets::Upgrade2];
        let widget_selector = Widgets::WeaponSelector as usize;
    	swap(&mut self.prev_keys, &mut self.new_keys);
    	self.new_keys.clear();
    	for key in key_codes.drain(..) {
    		self.new_keys.insert(key);
    		// do something here
    	}
    	let new_pressed = &self.new_keys - &self.prev_keys;
    	if let Some(upgrades) = upgrades {
	    	for key in new_pressed.iter() {
	    		match key {
	                Keycode::Left | Keycode::Right => {
	                	if ui.selected(widget_selector, widget_ids[0] as usize) 
	                			|| ui.free_selector(widget_selector) {
	                		dbg!("a");
							ui_state.choosed_upgrade = Some(upgrades[1]);
	                		ui.select(widget_selector, widget_ids[1] as usize)
	                	} else if ui.selected(widget_selector, widget_ids[1] as usize) {
	                		dbg!("b");
							ui_state.choosed_upgrade = Some(upgrades[0]);
	                		ui.select(widget_selector, widget_ids[0] as usize)
	                	}
	                	dbg!("asd");
	                }
	                // Keycode::Right => {
	                // 	dbg!("sd");
	                // }
	                _ => ()
	    		}
	    	}    		
    	}
    }
}