use super::*;
use super::*;
use crate::gui::*;
use physics::*;
use physics::*;

#[derive(Default, Clone)]
pub struct ControllingSystem;

impl<'a> System<'a> for ControllingSystem {
    type SystemData = (WriteExpect<'a, Touches>, Write<'a, AppState>);

    fn run(&mut self, data: Self::SystemData) {
        let (touches, mut app_state) = data;
        #[cfg(any(target_os = "android"))]
        {
            let controlling =
                touches.iter().filter(|x| x.is_some()).count() > 1;
            if let AppState::Play(ref mut play_state) = *app_state {
                match play_state {
                    PlayState::Action => {
                        if !controlling {
                            *play_state = PlayState::Upgrade;
                        }
                    }
                    PlayState::Upgrade => {
                        if controlling {
                            *play_state = PlayState::Action
                        }
                    }
                }
            }
        }
    }
}
