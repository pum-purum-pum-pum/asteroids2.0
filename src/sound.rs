use std::path::Path;

use al::prelude::*;
use astro_lib as al;

use crate::components::{Sound, Sounds};

use sdl2::mixer::{InitFlag, Sdl2MixerContext, AUDIO_S16LSB, DEFAULT_CHANNELS};
use sdl2::{AudioSubsystem, TimerSubsystem};

pub struct PreloadedSounds {
    pub shot: Sound,
    pub explosion: Sound,
}

pub fn init_sound(
    sdl: &sdl2::Sdl,
) -> Result<
    (
        Sounds,
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
    sdl2::mixer::allocate_channels(100);
    let sound_file_path = Path::new("assets/shot.wav");
    let explosion_file_path = Path::new("assets/explosion.wav");
    let shot_sound_chunk = sdl2::mixer::Chunk::from_file(sound_file_path)
        .map_err(|e| format!("Cannot load sound file: {:?}", e))?;
    let mut sounds = Sounds::new_empty();
    let explosion_sound_chunk = sdl2::mixer::Chunk::from_file(explosion_file_path)
        .map_err(|e| format!("Cannot load sound file: {:?}", e))?;
    let shot = sounds.add_item("shot".to_string(), shot_sound_chunk);
    let explosion = sounds.add_item("explosion".to_string(), explosion_sound_chunk);
    let preloaded_sounds = PreloadedSounds {
        shot: shot,
        explosion: explosion,
    };
    sdl2::mixer::Channel::all().set_volume(12);
    Ok((sounds, preloaded_sounds, audio, mixer_context, timer))
}
