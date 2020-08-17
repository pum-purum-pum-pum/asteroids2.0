use sheep::{Format, SpriteAnchor};
use std::collections::HashMap;

pub struct TwentyFormat;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpritePosition {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub offsets: Option<[f32; 2]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SerializedSpriteSheet {
    pub texture_width: f32,
    pub texture_height: f32,
    pub sprites: HashMap<String, SpritePosition>,
}

impl Format for TwentyFormat {
    type Data = SerializedSpriteSheet;
    type Options = Vec<String>;

    fn encode(
        dimensions: (u32, u32),
        sprites: &[SpriteAnchor],
        options: Self::Options,
    ) -> Self::Data {
        let sprite_positions = sprites.iter().map(|it| SpritePosition {
            x: it.position.0 as f32,
            y: it.position.1 as f32,
            width: it.dimensions.0 as f32,
            height: it.dimensions.1 as f32,
            offsets: None,
        });
        let sprites = sprites
            .iter()
            .map(|anchor| options[anchor.id].clone())
            .zip(sprite_positions)
            .collect::<HashMap<String, SpritePosition>>();
        SerializedSpriteSheet {
            texture_width: dimensions.0 as f32,
            texture_height: dimensions.1 as f32,
            sprites,
        }
    }
}
