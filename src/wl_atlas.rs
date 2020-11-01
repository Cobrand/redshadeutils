use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WLPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WLRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WLAtlas {
    pub models: Vec<WLModel>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WLModel {
    pub frames: Vec<WLFrame>,
    pub animations: Vec<WLAnimation>,
    pub model_id: String,
    pub anchor_point: WLPoint,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WLFrame {
    pub rect: WLRect,
    pub duration: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WLAnimation {
    pub animation_id: String,
    pub frames: Vec<u32>,
}