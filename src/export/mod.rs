//! Export of the current map as a zip holding map.png, map.svg and map.json.

mod json;
mod png;
mod svg;

use crate::biome::Biome;
use crate::icon_lib::IconDef;
use crate::layout::Layout;
use crate::settings::Settings;
use crate::text::{files, status};
use crate::world::World;
use egui::{Vec2, vec2};
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;

/// Image pixels per world unit. One cell is one unit tall.
const PX_PER_UNIT: f32 = 32.0;
/// Free space around the map in the image, in world units.
const PADDING: f32 = 0.5;

/// Everything written to the export.
pub struct MapData<'a> {
    pub seed: u64,
    pub cells: u32,
    pub settings: &'a Settings,
    pub world: &'a World,
    pub biomes: &'a [Biome],
    pub special: &'a [IconDef],
    pub layout: &'a Layout,
}

impl MapData<'_> {
    /// Corners of the map including its padding, in world units.
    fn area(&self) -> (Vec2, Vec2) {
        let (x0, y0, x1, y1) = self.world.bounds();
        let (cw, ch) = (self.layout.cw, self.layout.ch);
        (
            vec2(x0 as f32 * cw, y0 as f32 * ch) - Vec2::splat(PADDING),
            vec2(x1 as f32 * cw, y1 as f32 * ch) + Vec2::splat(PADDING),
        )
    }
}

/// Asks where to save, then writes the zip. Returns the message to show, or None if the
/// dialog was cancelled. Blocks the app until the dialog is closed.
pub fn save_with_dialog(map: &MapData) -> Option<String> {
    let mut path = rfd::FileDialog::new()
        .add_filter(files::ZIP_FILTER, &[files::ZIP_EXT])
        .set_file_name(files::zip_name(map.seed))
        .save_file()?;
    if path.extension().is_none() {
        path.set_extension(files::ZIP_EXT);
    }
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    Some(match write_zip(&path, map) {
        Ok(()) => status::exported(&name),
        Err(e) => status::export_failed(e),
    })
}

/// Small PNG preview of the map for a project archive.
pub fn thumbnail(map: &MapData) -> Result<Vec<u8>, Box<dyn Error>> {
    png::thumbnail(map)
}

/// Writes the zip to `path`.
fn write_zip(path: &Path, map: &MapData) -> Result<(), Box<dyn Error>> {
    let png = png::render(map)?;
    let svg = svg::render(map);
    let json = json::render(map)?;

    let mut zip = zip::ZipWriter::new(File::create(path)?);
    // PNG data is already compressed.
    let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file(files::EXPORT_PNG, stored)?;
    zip.write_all(&png)?;
    zip.start_file(files::EXPORT_SVG, SimpleFileOptions::default())?;
    zip.write_all(svg.as_bytes())?;
    zip.start_file(files::EXPORT_JSON, SimpleFileOptions::default())?;
    zip.write_all(json.as_bytes())?;
    zip.finish()?;
    Ok(())
}
