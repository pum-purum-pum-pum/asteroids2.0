use common::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

use sdl2::mixer::{
    InitFlag, Music, Sdl2MixerContext, AUDIO_S16LSB, DEFAULT_CHANNELS,
};
use sdl2::rwops::RWops;
use sdl2::{AudioSubsystem, TimerSubsystem};
use specs::prelude::*;
use specs_derive::Component;

const SOUND_CHANNELS: i32 = 100;
pub const EFFECT_MAX_VOLUME: i32 = 15;
pub const MUSIC_MAX_VOLUME: i32 = 100;

// pub struct SoundChannels {
//     used: [bool; SOUND_CHANNELS as usize]
// }

#[derive(Component)]
pub struct SoundPlacement {
    pub start: usize,
    pub end: usize,
    pub gap: Duration,
    pub last_upd: Instant,
}

impl SoundPlacement {
    pub fn new(start: usize, end: usize, gap: Duration) -> Self {
        SoundPlacement {
            start,
            end,
            gap,
            last_upd: Instant::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundSave {
    name: String,
    count: usize,
    gap: Duration,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundsSave(Vec<SoundSave>);

pub struct SoundData(pub sdl2::mixer::Chunk);

pub struct PreloadedSounds {
    pub shot: specs::Entity,
    pub asteroid_explosion: specs::Entity,
    pub ship_explosion: specs::Entity,
    pub blast: specs::Entity,
    pub lazer: specs::Entity,
    pub enemy_blaster: specs::Entity,
    pub enemy_shotgun: specs::Entity,
    pub collision: specs::Entity,
    pub coin: specs::Entity,
    pub coin2: specs::Entity,
    pub exp: specs::Entity,
    pub hover: specs::Entity,
    pub click: specs::Entity,
    pub play: specs::Entity,
    pub deny: specs::Entity,
    pub buy: specs::Entity,
}

pub struct MusicData<'a> {
    pub menu_music: Music<'a>,
    pub battle_music: Vec<Music<'a>>,
}

pub fn init_sound<'a>(
    sdl: &sdl2::Sdl,
    world: &mut specs::world::World,
) -> Result<
    (
        PreloadedSounds,
        MusicData<'a>,
        AudioSubsystem,
        Sdl2MixerContext,
        TimerSubsystem,
    ),
    String,
> {
    let audio = sdl.audio()?;
    let timer = sdl.timer()?;
    let frequency = 44_100;
    let format = AUDIO_S16LSB; // signed 16 bit samples, in little-endian byte order
    let channels = DEFAULT_CHANNELS; // Stereo
    let chunk_size = 1_024;
    sdl2::mixer::open_audio(frequency, format, channels, chunk_size)?;
    let mixer_context = sdl2::mixer::init(
        InitFlag::MP3
            | InitFlag::FLAC
            | InitFlag::MOD
            | InitFlag::OGG
            | InitFlag::MID,
    )?;
    sdl2::mixer::allocate_channels(SOUND_CHANNELS);
    let mut name_to_sound: HashMap<String, specs::Entity> = HashMap::new();

    {
        use ron::de::from_str;
        let file = include_str!("../../rons/sounds.ron");
        let sounds_save: SoundsSave = match from_str(file) {
            Ok(x) => x,
            Err(e) => {
                println!("Failed to load config: {}", e);
                std::process::exit(1);
            }
        };
        let mut id = 0usize;
        for sound_save in sounds_save.0.iter() {
            let name = &sound_save.name;
            let file = format!("assets/music/{}.wav", name);
            let sound_placement =
                SoundPlacement::new(id, id + sound_save.count, sound_save.gap);
            id += sound_save.count;
            let path = Path::new(&file);
            let sound_chunk = ThreadPin::new(SoundData(
                sdl2::mixer::Chunk::from_file(path)
                    .map_err(|e| format!("Cannot load sound file: {:?}", e))?,
            ));
            let sound = world
                .create_entity()
                .with(sound_chunk)
                .with(sound_placement)
                .build();
            name_to_sound.insert(name.to_string(), sound);
        }
        eprintln!("{:?}", sounds_save);
    }
    let preloaded_sounds = PreloadedSounds {
        shot: name_to_sound["shot"],
        blast: name_to_sound["explosion"],
        ship_explosion: name_to_sound["explosion2"],
        asteroid_explosion: name_to_sound["explosion_"],
        lazer: name_to_sound["lazer"],
        enemy_blaster: name_to_sound["shot2"],
        enemy_shotgun: name_to_sound["shot3"],
        collision: name_to_sound["collision"],
        coin: name_to_sound["coin"],
        coin2: name_to_sound["coin2"],
        exp: name_to_sound["exp"],
        hover: name_to_sound["hover"],
        click: name_to_sound["click"],
        play: name_to_sound["play"],
        deny: name_to_sound["deny"],
        buy: name_to_sound["buy"],
    };
    let mut name_to_music: HashMap<String, Music> = HashMap::new();
    {
        // load music
        let names = ["menu", "short_bells"];
        for name in names.iter() {
            let file = format!("assets/music/{}.ogg", name);
            let path = Path::new(&file);
            let music = sdl2::mixer::Music::from_file(path)
                .expect(&format!("failed to load {}", &name).to_string());
            name_to_music.insert(name.to_string(), music);
        }
    }
    let music_data = MusicData {
        menu_music: name_to_music.remove("menu").unwrap(),
        battle_music: vec![name_to_music.remove("short_bells").unwrap()],
    };
    sdl2::mixer::Channel::all().set_volume(EFFECT_MAX_VOLUME);
    sdl2::mixer::Music::set_volume(MUSIC_MAX_VOLUME);
    Ok((preloaded_sounds, music_data, audio, mixer_context, timer))
}
