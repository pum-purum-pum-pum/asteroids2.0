use std::path::Path;
use std::collections::HashMap;
use crate::types::{*};



use sdl2::mixer::{InitFlag, Sdl2MixerContext, AUDIO_S16LSB, DEFAULT_CHANNELS, Music};
use sdl2::{AudioSubsystem, TimerSubsystem};
use specs::prelude::*;


pub struct SoundData(pub sdl2::mixer::Chunk);

pub struct PreloadedSounds {
    pub shot: specs::Entity,
    pub explosion: specs::Entity,
    pub lazer: specs::Entity,
    pub enemy_blaster: specs::Entity,
    pub enemy_shotgun: specs::Entity,
    pub collision: specs::Entity,
}

pub struct MusicData<'a> {
    pub menu_music: Music<'a>,
    pub battle_music: Vec<Music<'a>>
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
    let mixer_context =
        sdl2::mixer::init(InitFlag::MP3 | InitFlag::FLAC | InitFlag::MOD | InitFlag::OGG)?;
    sdl2::mixer::allocate_channels(100);
    let mut name_to_sound: HashMap<String, specs::Entity> = HashMap::new();
    {   // load sounds
        let names = [
            "shot",
            "explosion",
            "lazer",
            "collision",
            "shot2",
            "shot3",
        ];
        for name in names.iter() {
            let file = format!("assets/music/{}.wav", name);
            let path = Path::new(&file);
            let sound_chunk = ThreadPin::new(SoundData(
                sdl2::mixer::Chunk::from_file(path)
                    .map_err(|e| format!("Cannot load sound file: {:?}", e))?
            ));
            let sound = world
                .create_entity()
                .with(sound_chunk)
                .build();
            name_to_sound.insert(name.to_string(), sound);
        }
    }

    let preloaded_sounds = PreloadedSounds {
        shot: name_to_sound["shot"],
        explosion: name_to_sound["explosion"],
        lazer: name_to_sound["lazer"],
        enemy_blaster: name_to_sound["shot2"],
        enemy_shotgun: name_to_sound["shot3"],
        collision: name_to_sound["collision"]

    };
    let mut name_to_music: HashMap<String, Music> = HashMap::new();
    {   // load music
        let names = [
            "level2",
            "level5"
        ];
        for name in names.iter() {
            let file = format!("assets/music/{}.ogg", name);
            let path = Path::new(&file);
            let music = sdl2::mixer::Music::from_file(path)
                .expect(&format!("failed to load {}", &name).to_string());
            name_to_music.insert(name.to_string(), music);
        }
    }
    let music_data = MusicData {
        menu_music: name_to_music.remove("level2").unwrap(),
        battle_music: vec![name_to_music.remove("level5").unwrap()]
    };
    sdl2::mixer::Channel::all().set_volume(12);
    sdl2::mixer::Music::set_volume(30);
    Ok((preloaded_sounds, music_data, audio, mixer_context, timer))
}
