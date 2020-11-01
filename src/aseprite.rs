use serde::{Deserialize};

#[derive(Debug, Deserialize)]
pub struct AsepriteRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Deserialize)]
pub struct AsepriteDataFrame {
    #[serde(rename = "frame")]
    pub rect: AsepriteRect,
    pub duration: u32,
}

#[derive(Debug, Deserialize)]
pub struct AsepriteFrameTag {
    pub name: String,
    pub from: u32,
    pub to: u32,
}

#[derive(Debug, Deserialize)]
pub struct AsepriteDataMeta {
    #[serde(rename = "image")]
    pub image_path: String,
    #[serde(rename = "frameTags")]
    pub tags: Vec<AsepriteFrameTag>,
}

#[derive(Debug, Deserialize)]
pub struct AsepriteDataFile {
    pub frames: Vec<AsepriteDataFrame>,
    pub meta: AsepriteDataMeta,
}