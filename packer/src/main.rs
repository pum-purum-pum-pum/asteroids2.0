#[macro_use]
extern crate serde_derive;
extern crate image;
extern crate sheep;

mod format;

use format::TwentyFormat;
use image::DynamicImage;
use sheep::Format;
use sheep::{
    AmethystFormat, InputSprite, MaxrectsOptions, MaxrectsPacker, SimplePacker,
    SpriteAnchor,
};
use std::fs::{self, DirEntry};
use std::io::Result;
use std::path::Path;
use std::{fs::File, io::prelude::*};
use texture_packer::exporter::ImageExporter;
use texture_packer::importer::ImageImporter;
use texture_packer::texture::Texture;
use texture_packer::{TexturePacker, TexturePackerConfig};

type ImgBuf = image::ImageBuffer<image::Rgba<u8>, Vec<u8>>;

/// return files names and image buffers
fn images_from_dir(
    dir: &Path,
    prefix: &str,
) -> Result<(Vec<String>, Vec<String>, Vec<ImgBuf>)> {
    let mut images = vec![];
    let mut names = vec![];
    let mut paths = vec![];
    for entry in fs::read_dir(dir)? {
        let p = entry?.path();
        if let Some(extension) = p.extension() {
            if extension == "png" {
                paths.push(p.to_str().unwrap().to_string());
                let image = image::open(p.to_str().unwrap());
                let name = p.file_stem().unwrap().to_str().unwrap().to_string();
                names.push(prefix.to_string() + &name);
                images.push(
                    image
                        .unwrap()
                        .as_rgba8()
                        .expect("Failed to convert image to rgba8")
                        .clone(),
                );
            }
        }
    }
    Ok((names, paths, images))
}

/// convert image buffers into sprites and write them
fn write_sprites<T>(sprites: &mut Vec<InputSprite>, images: T)
where
    T: IntoIterator<Item = ImgBuf>,
{
    for img in images {
        let dimensions = img.dimensions();
        println!("{:?}", dimensions);
        let bytes = img
            .pixels()
            .flat_map(|it| it.data.iter().map(|it| *it))
            .collect::<Vec<u8>>();
        sprites.push(InputSprite {
            dimensions,
            bytes: bytes.clone(),
        });
    }
}

fn mimic_sheep(
    atlas_path: &str,
    meta_path: &str,
    sprites_paths: &[String],
    names: &[String],
) {
    let config = TexturePackerConfig {
        max_width: 3000,
        max_height: 3000,
        allow_rotation: false,
        // texture_outlines: true,
        // border_padding: 2,
        ..Default::default()
    };

    let mut packer = TexturePacker::new_skyline(config);
    for (path, name) in sprites_paths.iter().zip(names.iter()) {
        let path = Path::new(&path);
        let texture = ImageImporter::import_from_file(&path).unwrap();
        packer.pack_own(name.to_string(), texture);
    }
    let exporter = ImageExporter::export(&packer).unwrap();
    let mut file = File::create(atlas_path).unwrap();
    exporter.write_to(&mut file, image::PNG).unwrap();
    let twenty_format = {
        let dimensions = (packer.width(), packer.height());
        let mut sprites = vec![];
        let mut sprite_names = vec![];
        for (i, (name, frame)) in packer.get_frames().iter().enumerate() {
            let sprite = SpriteAnchor::new(
                i,
                (frame.frame.x, frame.frame.y),
                (frame.frame.w, frame.frame.h),
            );
            sprites.push(sprite);
            sprite_names.push(name.clone());
        }
        TwentyFormat::encode(dimensions, &sprites, sprite_names)
    };
    let meta = twenty_format;
    let mut meta_file =
        File::create(meta_path).expect("Failed to create meta file");
    let meta_str =
        ron::ser::to_string(&meta).expect("Failed to encode meta file");

    meta_file
        .write_all(meta_str.as_bytes())
        .expect("Failed to write meta file");
}

fn main() {
    let assets = Path::new("../assets/twenty_assets");
    let mut animations = vec![];
    let mut animations_names = vec![];
    let mut animations_paths = vec![];
    for entry in fs::read_dir(assets).unwrap() {
        let e = entry.unwrap();
        if e.path().is_dir() {
            let dir_name =
                e.path().file_stem().unwrap().to_str().unwrap().to_string();
            if dir_name == "trash" {
                continue;
            };
            let (anim_name, anim_paths, anim) =
                images_from_dir(&e.path(), &(dir_name + "_anim_")).unwrap();
            animations.push(anim);
            animations_names.push(anim_name);
            animations_paths.push(anim_paths);
        }
    }
    let mut sprites = vec![];
    let mut names = vec![];
    let mut paths = vec![];
    let (static_names, static_paths, static_images) =
        images_from_dir(&assets, "").unwrap();
    names.extend(static_names.iter().cloned());
    paths.extend(static_paths);
    write_sprites(&mut sprites, static_images);
    // for animation in animations {
    //     write_sprites(&mut sprites, &animation)
    // }
    write_sprites(&mut sprites, animations.iter().flatten().cloned());
    names.extend(animations_names.iter().flatten().cloned());
    paths.extend(animations_paths.iter().flatten().cloned());
    mimic_sheep("../assets/atlas.png", "../assets/out.ron", &paths, &names);

    //// real ship
    // {
    //     let maxrects: MaxrectsOptions = Default::default();
    //     let results = sheep::pack::<MaxrectsPacker>(
    //         sprites,
    //         4,
    //         maxrects
    //     );

    //     // SimplePacker always returns a single result. Other packers can return
    //     // multiple sheets; should they, for example, choose to enforce a maximum
    //     // texture size per sheet.
    //     let sprite_sheet = results
    //         .into_iter()
    //         .next()
    //         .expect("Should have returned a spritesheet");

    //     // Now, we can encode the sprite sheet in a format of our choosing to
    //     // save things such as offsets, positions of the sprites and so on.
    //     let meta = sheep::encode::<TwentyFormat>(&sprite_sheet, names);

    //     // Next, we save the output to a file using the image crate again.
    //     let outbuf = image::RgbaImage::from_vec(
    //         sprite_sheet.dimensions.0,
    //         sprite_sheet.dimensions.1,
    //         sprite_sheet.bytes,
    //     )
    //     .expect("Failed to construct image from sprite sheet bytes");

    //     outbuf
    //         .save("../assets/atlas.png")
    //         .expect("Failed to save image");

    //     // Lastly, we serialize the meta info using serde. This can be any format
    //     // you want, just implement the trait and pass it to encode.
    //     let mut meta_file =
    //         File::create("../assets/out.ron").expect("Failed to create meta file");
    //     let meta_str =
    //         ron::ser::to_string(&meta).expect("Failed to encode meta file");

    //     meta_file
    //         .write_all(meta_str.as_bytes())
    //         .expect("Failed to write meta file");
    // }
}
