use super::*;

pub struct SoundSystem {
    reader: ReaderId<Sound>,
}

impl SoundSystem {
    pub fn new(reader: ReaderId<Sound>) -> Self {
        SoundSystem { reader: reader }
    }
}

impl<'a> System<'a> for SoundSystem {
    type SystemData = (
        ReadStorage<'a, ThreadPin<SoundData>>,
        ReadStorage<'a, CharacterMarker>,
        ReadStorage<'a, MultyLazer>,
        WriteStorage<'a, SoundPlacement>,
        ReadExpect<'a, PreloadedSounds>,
        Write<'a, EventChannel<Sound>>,
        Write<'a, LoopSound>,
        ReadExpect<'a, ThreadPin<MusicData<'static>>>,
        Write<'a, Music>,
        Read<'a, AppState>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            sounds, 
            character_markers,
            multy_lazers,
            mut sound_placements,
            preloaded_sounds,
            sounds_channel,
            mut loop_sound,
            music_data,
            mut music,
            app_state
        ) = data;
        for s in sounds_channel.read(&mut self.reader) {
            let sound = &sounds.get(s.0).unwrap().0;
            let position = s.1;
            let placement = sound_placements.get_mut(s.0).unwrap();
            for i in placement.start..placement.end {
                let current_channel = sdl2::mixer::Channel(i as i32);
                if !current_channel.is_playing() && 
                    Instant::now()
                        .duration_since(placement.last_upd) >= placement.gap {
                    placement.last_upd = Instant::now();
                    current_channel.play(sound, 0).unwrap();
                    let n = position.coords.norm();
                    // let smooth = 1.0; // more value less depend on l
                    let l = 1.0 + n;
                    let mut fade = 1.0 / (l.ln());
                    if n < 10f32 {
                        fade = 1.0;
                    }
                    current_channel.set_volume(
                        (EFFECT_MAX_VOLUME as f32 * fade) as i32
                    );
                    break;
                }
            }
        }
        for (lazer, _character) in (&multy_lazers, &character_markers).join() {
            if lazer.active() {
                if loop_sound.player_lazer_channel.is_none() {
                    let channel = sdl2::mixer::Channel::all().play(
                        &sounds.get(preloaded_sounds.lazer).unwrap().0,
                        -1
                    ).unwrap();
                    music.menu_play = false; // hacky
                    loop_sound.player_lazer_channel = Some(channel);
                }
            } else {
                if let Some(lazer) = loop_sound.player_lazer_channel {
                    lazer.halt();
                    loop_sound.player_lazer_channel = None;
                }
            }
        }
        match *app_state {
            AppState::Play(_) => {
                if music.current_battle.is_none() {
                    let mut rng = thread_rng();
                    let music_id = rng.gen_range(0, music_data.battle_music.len());
                    sdl2::mixer::Music::halt();
                    music.menu_play = false;
                    music_data.battle_music[music_id].play(-1).unwrap();
                    music.current_battle = Some(music_id);
                }
            }
            AppState::Menu | AppState::DeadScreen => {
                loop_sound.player_lazer_channel = None; // hacky
                if let Some(_music_id) = music.current_battle {
                    sdl2::mixer::Music::halt();
                    music.current_battle = None;
                }
                if !music.menu_play {
                    music_data.menu_music.play(-1).unwrap();
                    music.menu_play = true;
                }
            }
            AppState::ScoreTable => {
                
            }
        }
    }
}
