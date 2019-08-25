

use crate::nalgebra::Rotation2;

use sdl2::mixer::{InitFlag, AUDIO_S16LSB, DEFAULT_CHANNELS};
use std::path::Path;

#[test]
fn rotation() {
    let rot1 = Rotation2::new(1.5 * 3.14);
    let rot2 = Rotation2::new(0.5 * 3.14);
    dbg!((rot1.angle(), rot2.angle()));
}

#[test]
fn sound() -> Result<(), String> {
    let sdl = sdl2::init()?;
    let _audio = sdl.audio()?;
    let _timer = sdl.timer()?;
    let frequency = 44_100;
    let format = AUDIO_S16LSB; // signed 16 bit samples, in little-endian byte order
    let channels = DEFAULT_CHANNELS; // Stereo
    let chunk_size = 1_024;
    sdl2::mixer::open_audio(frequency, format, channels, chunk_size)?;
    let _mixer_context =
        sdl2::mixer::init(InitFlag::MP3 | InitFlag::FLAC | InitFlag::MOD | InitFlag::OGG)?;
    sdl2::mixer::allocate_channels(4);
    println!("query spec => {:?}", sdl2::mixer::query_spec());
    let sound_file_path = Path::new("assets/shot.wav");
    let sound_chunk = sdl2::mixer::Chunk::from_file(sound_file_path)
        .map_err(|e| format!("Cannot load sound file: {:?}", e))?;
    sdl2::mixer::Channel::all().play(&sound_chunk, 0)?;
    Ok(())
}