//! JSON description of the map: settings, biomes, rooms and doors.

use super::MapData;
use crate::biome::Entry;
use crate::settings::Settings;
use crate::world::{Door, Room};
use serde::Serialize;
use std::error::Error;

/// Rooms and doors use cell coordinates. `biome` is an index into `biomes`, and the
/// door ends `a` and `b` are indices into `rooms`.
#[derive(Serialize)]
struct MapFile<'a> {
    seed: u64,
    cells: u32,
    settings: &'a Settings,
    biomes: Vec<Entry>,
    /// Indices into `biomes`, in placement order.
    used_biomes: &'a [usize],
    rooms: &'a [Room],
    doors: &'a [Door],
}

pub fn render(map: &MapData) -> Result<String, Box<dyn Error>> {
    let file = MapFile {
        seed: map.seed,
        cells: map.cells,
        settings: map.settings,
        biomes: map.biomes.iter().map(Entry::from).collect(),
        used_biomes: &map.world.biomes,
        rooms: &map.world.rooms,
        doors: &map.world.doors,
    };
    Ok(serde_json::to_string_pretty(&file)?)
}
