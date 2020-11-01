use serde::{Serialize, Deserialize};
use anyhow::{Result, Context, anyhow};
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

mod aseprite;
mod wl_atlas;

use aseprite::AsepriteDataFile;
use wl_atlas::{WLModel, WLPoint, WLFrame, WLRect, WLAnimation};

#[derive(Debug, Deserialize)]
struct IndexFile(Vec<IndexEntry>);

#[derive(Debug, Deserialize)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug, Deserialize)]
struct IndexEntry {
    model_id: String,
    data_file: String,
    anchor_point: Point,
}

fn gen_wl_model(ase_data: AsepriteDataFile, model_id: String, anchor_point: Point) -> WLModel {
    WLModel {
        anchor_point: WLPoint { x: anchor_point.x, y: anchor_point.y },
        model_id,
        frames: ase_data.frames.into_iter().map(|frame| WLFrame {
            duration: frame.duration,
            rect: WLRect {
                x: frame.rect.x,
                y: frame.rect.y,
                w: frame.rect.w,
                h: frame.rect.h,
            }
        }).collect::<Vec<_>>(),
        animations: ase_data.meta.tags.into_iter()
            .filter_map(|tag| tag.name.strip_prefix("a_")
                .map(|s| s.to_string())
                .map(|s| (s, tag)) // 2 maps to make borrow checker happy
            )
            .map(|(anim_id, tag)| WLAnimation {
                animation_id: anim_id,
                frames: (tag.from..=tag.to).collect(),
            })
            .collect::<Vec<_>>()
    }
}

fn write_zip(zip_file: &mut File, image_paths: &[PathBuf], atlas_content: &wl_atlas::WLAtlas) -> Result<()> {
    let mut zip_writer = zip::ZipWriter::new(zip_file);

    zip_writer.start_file("index.json", Default::default())?;
    serde_json::to_writer_pretty(&mut zip_writer, atlas_content)?;

    for (model, image_path) in atlas_content.models.iter().zip(image_paths.iter()) {
        if model.animations.len() == 0 {
            return Err(anyhow!("model_id \"{}\" has a data file but 0 animations", model.model_id));
        }
        let animations = model.animations.iter()
            .map(|a| a.animation_id.clone())
            .collect::<Vec<_>>();
        println!("Packing model_id={} with {} animations: {}", model.model_id, animations.len(), animations.join(", "));

        zip_writer.start_file(format!("{}.png", model.model_id), Default::default())?;
        let mut image_file = File::open(image_path)
            .with_context(|| format!("failed to open image file at {}", image_path.display()))?;
        
        std::io::copy(&mut image_file, &mut zip_writer)
            .with_context(|| format!("failed to write image file {} inside zip", model.model_id))?;
    }

    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();

    if args.len() < 3 {
        return Err(anyhow!("Not enough arguments. Usage: {} <index.yaml> <output zip>", &args[0]))
    }


    let index_file_path = &args[1];
    let output_zip_file_path = &args[2];

    let index_file = File::open(index_file_path)
        .context("failed to open index json file")?;

    let mut zip_file = File::create(output_zip_file_path)
        .context("failed to open zip file for writing")?;

    let parsed_index_file: IndexFile = serde_yaml::from_reader(&index_file)?;

    let mut wl_models = Vec::new();
    let mut image_paths: Vec<PathBuf> = Vec::new();

    println!("Packing {} models in {}", parsed_index_file.0.iter().count(), output_zip_file_path);
    for index_entry in parsed_index_file.0.into_iter() {
        let mut json_data_path = PathBuf::from(index_file_path);
        json_data_path.pop();
        json_data_path.push(&index_entry.data_file);

        let json_file = File::open(&json_data_path)
            .with_context(|| format!("failed to read json file at {}", json_data_path.display()))?;

        let aseprite_data: AsepriteDataFile = serde_json::from_reader(&json_file)
            .with_context(|| format!("failed to read json file at {}", json_data_path.display()))?;

        let mut image_path = json_data_path;
        image_path.pop();
        image_path.push(&aseprite_data.meta.image_path);

        image_paths.push(image_path);
        let wl_model = gen_wl_model(aseprite_data, index_entry.model_id, index_entry.anchor_point);
        wl_models.push(wl_model);
    }

    write_zip(&mut zip_file, image_paths.as_slice(), &wl_atlas::WLAtlas { models: wl_models })
        .context("failed to write zip file")?;
    
    println!("Done!");

    Ok(())
}