use std::path::Path;
use crate::types::{*};



use sdl2::mixer::{InitFlag, Sdl2MixerContext, AUDIO_S16LSB, DEFAULT_CHANNELS};
use sdl2::{AudioSubsystem, TimerSubsystem};
use specs::prelude::*;


pub struct SoundData(pub sdl2::mixer::Chunk);

pub struct PreloadedSounds {
    pub shot: specs::Entity,
    pub explosion: specs::Entity,
}

pub fn init_sound<'a>(
    sdl: &sdl2::Sdl,
    world: &mut specs::world::World,
) -> Result<
    (
        PreloadedSounds,
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
    #[cfg(any(target_os = "ios", target_os = "android", target_os = "emscripten"))]
    trace!("vlad sound allocation");
    sdl2::mixer::allocate_channels(100);
    #[cfg(any(target_os = "ios", target_os = "android", target_os = "emscripten"))]
    trace!("vlad end of sound allocation");
    let shot = "assets/shot.wav";
    let explosion = "assets/explosion.wav";
    let shot_file_path = Path::new(&shot);
    let explosion_file_path = Path::new(&explosion);
    let shot_sound_chunk = ThreadPin::new(SoundData(
        sdl2::mixer::Chunk::from_file(shot_file_path)
            .map_err(|e| format!("Cannot load sound file: {:?}", e))?
    ));
    #[cfg(any(target_os = "ios", target_os = "android", target_os = "emscripten"))]
    trace!("vlad created chunk");

    let explosion_sound_chunk = ThreadPin::new(SoundData(
        sdl2::mixer::Chunk::from_file(explosion_file_path)
            .map_err(|e| format!("Cannot load sound file: {:?}", e))?
    ));
    let shot_sound = world
        .create_entity()
        .with(shot_sound_chunk)
        .build();
    let explosion_sound = world
        .create_entity()
        .with(explosion_sound_chunk)
        .build();
    // let shot = sounds.add_item("shot".to_string(), shot_sound_chunk);
    // let explosion = sounds.add_item("explosion".to_string(), explosion_sound_chunk);
    let preloaded_sounds = PreloadedSounds {
        shot: shot_sound,
        explosion: explosion_sound,
    };
    sdl2::mixer::Channel::all().set_volume(12);
    Ok((preloaded_sounds, audio, mixer_context, timer))
}
