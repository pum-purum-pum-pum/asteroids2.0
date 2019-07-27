use crate::components::*;
use crate::geometry::*;
use crate::nalgebra::Rotation2;
use crate::types::{*};
use sdl2::mixer::{InitFlag, AUDIO_S16LSB, DEFAULT_CHANNELS};
use std::path::Path;

#[test]
fn rotation() {
    let rot1 = Rotation2::new(1.5 * 3.14);
    let rot2 = Rotation2::new(0.5 * 3.14);
    dbg!((rot1.angle(), rot2.angle()));
}

#[test]
fn geom() {
    let mut poly =
        LightningPolygon::new_rectangle(-10f32, -10f32, 10f32, 10f32, Point2::new(3f32, 0f32));
    poly.clip_one(Geometry::Circle { radius: 1f32 }, Point2::new(8.5f32, 0f32));
    dbg!(&poly.points);
    dbg!(poly.points.len());
    poly.clip_one(Geometry::Circle { radius: 1f32 }, Point2::new(9.5f32, 0f32));
    dbg!(poly.points.len());
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




// fn is_precision(word: &str) -> bool {
//     let precision = vec![
//         "vec2",
//         "vec4",
//         "vec3",
//         "mat2",
//         "mat3",
//         "mat4",
//         "mat5",
//         "float",
//     ];
//     for p in precision.iter() {
//         if word == *p {
//             return true;
//         }
//     }
//     false
// }

// #[derive(PartialEq)]
// pub enum ShaderType {
//     Vertex,
//     Fragment
// }

// pub enum Version {
//     V300,
//     V100,
// }

// pub fn glesit(src: &str, shader_type: ShaderType, to_version: Version) -> String {
//     let lines: Vec<_> = src.split("\n").collect();
//     let mut find_and_replace = match to_version {
//         Version::V300 => {
//             match shader_type {
//                 ShaderType::Vertex => {
//                     vec![("#version 130", "#version 300 es")]
//                 }
//                 ShaderType::Fragment => {
//                     vec![
//                         ("#version 130", "#version 300 es\nout mediump vec4  astro_FragColor;"),
//                         ("gl_FragColor", "astro_FragColor")
//                     ]
//                 }
//             }
//         }
//         Version::V100 => {
//             let mut res = vec![("#version 130", "#version 100")];
//             match shader_type {
//                 ShaderType::Fragment => {
//                     res.push(("texture(", "texture2D("));
//                 }
//                 ShaderType::Vertex => ()
//             };
//             res
//         }
//     };
//     let mut subst = HashMap::new();
//     match to_version {
//         Version::V100 => {
//             match shader_type {
//                 ShaderType::Vertex => {
//                     subst.insert("in", "attribute");
//                 }
//                 ShaderType::Fragment => {
//                     subst.insert("in", "varying");
//                 }
//             }
//             subst.insert("out", "varying");
//         }
//         Version::V300 => {}
//     };
//     let mut new_lines = vec!();
//     for line in lines.iter() {
//         let words: Vec<_> = line.split(" ").collect();
//         let mut new_words = vec![];
//         let mut last_word = String::new();
//         for w in words.iter() {
//             if is_precision(w) && (last_word != "in".to_string() || shader_type == ShaderType::Fragment) {
//                 new_words.push("mediump".to_string());
//             }
//             match subst.get(w) {
//                 Some(&new_word) => {
//                     new_words.push(String::from_str(new_word).unwrap())
//                 }
//                 None => {
//                     new_words.push(String::from_str(w).unwrap())
//                 }
//             }
//             last_word = String::from_str(w).unwrap();
//         }
//         let mut new_line = new_words.join(" ");
//         for (f, r) in find_and_replace.iter() {
//             new_line = new_line.replace(f, r);
//         }
//         new_lines.push(new_line)
//     }
//     new_lines.join("\n")
// }

// #[test]
// fn gles() {
//     // use crate::gfx::{glesit, ShaderType};
//     let vertex_shader_src = r#"
//         #version 130
//         in vec2 tex_coords;
//         in vec2 position;
//         out vec2 v_tex_coords;

//         uniform mat4 perspective;
//         uniform mat4 view;
//         uniform mat4 model;
//         uniform float scale;
//         uniform vec2 dim_scales;

//         vec2 position_scaled;

//         void main() {
//             v_tex_coords = tex_coords;
//             position_scaled = scale * dim_scales * position;
//             gl_Position = perspective * view * model * vec4(position_scaled, 0.0, 1.0);
//         }
//     "#;
//     let fragment_shader_src = r#"
//         #version 130
//         in vec2 v_tex_coords;
//         out vec4 color;

//         uniform sampler2D tex;
//         void main() {
//             vec4 texture_colors = vec4(texture(tex, v_tex_coords));
//             color = texture_colors;
//         }
//     "#;
//     eprintln!("{}", glesit(&String::from_str(vertex_shader_src).unwrap(), ShaderType::Vertex));
//     eprintln!("{}", glesit(&String::from_str(fragment_shader_src).unwrap(), ShaderType::Fragment));
// }


// #[test]
// fn size() {
//     dbg!(std::mem::size_of::<u8>());
// }
// #[test]
// fn translate_to_gles() {
//     use std::fs::{self, DirEntry};
//     let dir = Path::new("gl");
//     for entry in fs::read_dir(dir).unwrap() {
//         let entry = entry.unwrap();
//         let path = entry.path();
//         // cb(&entry);
//         eprintln!("{:?}", path);
//         let name = path.file_name().unwrap().to_str().unwrap();
//         let es_file = format!("gles/{}", name);
//         // let shader = path.to_str().unwrap();
//         // dbg!(shader);
//         // let shader = crate::gfx::read_file(&shader).unwrap();
//         let mut shader = fs::File::open(path).unwrap();
//         let mut src = String::new();
//         shader.read_to_string(&mut src);
//         let shader = crate::gfx::glesit(
//             &src, 
//             crate::gfx::ShaderType::Vertex, 
//             crate::gfx::Version::V100
//         );
//         let mut file = fs::File::create(es_file).unwrap();
//         file.write_all(shader.as_bytes());
//     }
//     let name = "";
//     let vertex = format!("gl/v_{}.glsl", name);
//     let fragment = format!("gl/f_{}.glsl", name);
//     let (mut vertex_shader, mut fragment_shader) = (
//         crate::gfx::read_file(&vertex).unwrap(),
//         crate::gfx::read_file(&fragment).unwrap()
//     );
//     vertex_shader = crate::gfx::glesit(&crate::gfx::read_file(&vertex).unwrap(), crate::gfx::ShaderType::Vertex, crate::gfx::Version::V100);
//     fragment_shader = crate::gfx::glesit(&crate::gfx::read_file(&fragment).unwrap(), crate::gfx::ShaderType::Fragment, crate::gfx::Version::V100);
//     eprintln!("{:#?}", vertex_shader);
// }