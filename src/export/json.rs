//! JSON description of the map: settings, biomes, rooms and doors.

use super::MapData;
use crate::biome::Entry;
use crate::layer::SavedLayer;
use crate::settings::Settings;
use serde::Serialize;
use std::error::Error;

#[derive(Serialize)]
struct MapFile<'a> {
    seed: u64,
    cells: u32,
    settings: &'a Settings,
    biomes: Vec<Entry>,
    active: usize,
    layers: Vec<SavedLayer>,
}

pub fn render(map: &MapData) -> Result<String, Box<dyn Error>> {
    let layers = if let Some(i) = map.only_layer {
        map.layers
            .to_saved()
            .into_iter()
            .enumerate()
            .filter(|(idx, _)| *idx == i)
            .map(|(_, l)| l)
            .collect()
    } else {
        map.layers
            .to_saved()
            .into_iter()
            .zip(map.layers.layers())
            .filter(|(_, live)| live.visible)
            .map(|(saved, _)| saved)
            .collect()
    };
    let file = MapFile {
        seed: map.seed,
        cells: map.cells,
        settings: map.settings,
        biomes: map.biomes.iter().map(Entry::from).collect(),
        active: map.only_layer.unwrap_or(map.layers.active()),
        layers,
    };
    Ok(serde_json::to_string_pretty(&file)?)
}
