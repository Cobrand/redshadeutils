use serde::{Deserialize};
use anyhow::{Result, Context, anyhow};
use std::fs::File;
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
#[serde(untagged)]
enum IndexEntry {
    Pixel {
        model_id: String,
        data_file: String,
        anchor_point: Point,
    },
    Graphic {
        model_id: String,
        image_file: String,
    },
}

fn gen_wl_model_from_png(model_id: String, image_path: &PathBuf) -> Result<WLModel> {
    let png_decoder = png::Decoder::new(File::open(image_path)
        .with_context(|| format!("failed to open png file at {}", image_path.display()))?
    );

    let (info, _) = png_decoder.read_info()
        .with_context(|| format!("faield to decode png file at {}", image_path.display()))?;

    Ok(WLModel {
        anchor_point: WLPoint { x: 0, y: 0},
        model_id,
        frames: vec![ WLFrame {
            duration: 1000,
            rect: WLRect {
                x: 0,
                y: 0,
                w: info.width as i32,
                h: info.height as i32,
            }
        }],
        animations: vec![WLAnimation {
            animation_id: String::from("idle"),
            frames: vec![0]
        }],
    })
}

fn gen_wl_model_from_ase_data(ase_data: AsepriteDataFile, model_id: String, anchor_point: Point) -> Result<WLModel> {
    let animations = ase_data.meta.tags.into_iter()
        .filter_map(|tag| tag.name.strip_prefix("a_")
            .map(|s| s.to_string())
            .map(|s| (s, tag)) // 2 maps to make borrow checker happy
        )
        .map(|(anim_id, tag)| WLAnimation {
            animation_id: anim_id,
            frames: (tag.from..=tag.to).collect(),
        })
        .collect::<Vec<_>>();
    
    let animations = if animations.is_empty() {
        if ase_data.frames.len() > 1 {
            return Err(anyhow!("model_id {} has {} frames but 0 animations, can't use default idle with 1 frame", model_id, ase_data.frames.len()))
        }
        vec![WLAnimation {
            animation_id: String::from("idle"),
            frames: vec![0]
        }]
    } else {
        animations
    };

    Ok(WLModel {
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
        animations,
    })
}

fn write_zip(zip_file: &mut File, image_paths: &[PathBuf], atlas_content: &wl_atlas::WLAtlas) -> Result<()> {
    let mut zip_writer = zip::ZipWriter::new(zip_file);

    zip_writer.start_file("index.json", Default::default())?;
    serde_json::to_writer_pretty(&mut zip_writer, atlas_content)?;

    for (model, image_path) in atlas_content.models.iter().zip(image_paths.iter()) {
        if model.animations.len() == 0 {
            if model.frames.len() > 1 {
                // if frames = 1 and animations = 0, the only frame will be idle
                return Err(anyhow!("model_id \"{}\" has a data file with {} frames but 0 animations", model.model_id, model.frames.len()));
            }
            println!("Packing model_id={} with one default \"idle\" animation", model.model_id);
        } else {
            let animations = model.animations.iter()
                .map(|a| a.animation_id.clone())
                .collect::<Vec<_>>();
            println!("Packing model_id={} with {} animations: {}", model.model_id, animations.len(), animations.join(", "));
        }
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
        let wl_model = match index_entry {
            IndexEntry::Pixel { model_id, data_file, anchor_point } => {
                let mut json_data_path = PathBuf::from(index_file_path);
                json_data_path.pop();
                json_data_path.push(&data_file);

                let json_file = File::open(&json_data_path)
                    .with_context(|| format!("failed to read json file at {}", json_data_path.display()))?;

                let aseprite_data: AsepriteDataFile = serde_json::from_reader(&json_file)
                    .with_context(|| format!("failed to read json file at {}", json_data_path.display()))?;

                let mut image_path = json_data_path;
                image_path.pop();
                image_path.push(&aseprite_data.meta.image_path);

                image_paths.push(image_path);
                gen_wl_model_from_ase_data(aseprite_data, model_id, anchor_point)?
            },
            IndexEntry::Graphic { model_id, image_file } => {
                let mut image_path = PathBuf::from(index_file_path);
                image_path.pop();
                image_path.push(&image_file);
                
                let model = gen_wl_model_from_png(model_id, &image_path)?;
                image_paths.push(image_path);
                model
            }
        };
        wl_models.push(wl_model);
    }

    write_zip(&mut zip_file, image_paths.as_slice(), &wl_atlas::WLAtlas { models: wl_models })
        .context("failed to write zip file")?;
    
    println!("Done!");

    Ok(())
}