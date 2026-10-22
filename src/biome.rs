//! Biome definitions and their storage in a JSON file.

use crate::color::{parse_hex, to_hex};
use crate::storage;
use crate::text::{biomes as text, files};
use crate::world::Extras;
use egui::Color32;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::{Path, PathBuf};

/// Default and maximum number of secret, boss, save and portal rooms per biome.
pub const DEFAULT_COUNT: u32 = 1;
pub const MAX_COUNT: u32 = 5;
/// Lowest corridor count. The corridor to the parent biome is always built, so 0 and 1
/// give the same map, and only the counts above 1 add corridors.
pub const MIN_CORRIDORS: u32 = 0;

#[derive(Clone)]
pub struct Biome {
    pub name: String,
    pub fill: Color32,
    /// Number of corridors, from `MIN_CORRIDORS` to `MAX_COUNT`.
    pub corridors: u32,
    /// Number of secret rooms added to the biome, up to `MAX_COUNT`.
    pub secrets: u32,
    /// Number of portal rooms, up to `MAX_COUNT`.
    pub portals: u32,
    /// Number of boss rooms, up to `MAX_COUNT`.
    pub bosses: u32,
    /// Number of save rooms, up to `MAX_COUNT`.
    pub saves: u32,
}

impl Biome {
    pub fn new(name: &str, r: u8, g: u8, b: u8) -> Biome {
        Biome {
            name: name.to_owned(),
            fill: Color32::from_rgb(r, g, b),
            corridors: DEFAULT_COUNT,
            secrets: DEFAULT_COUNT,
            portals: DEFAULT_COUNT,
            bosses: DEFAULT_COUNT,
            saves: DEFAULT_COUNT,
        }
    }

    pub fn extras(&self) -> Extras {
        Extras {
            corridors: self.corridors,
            secrets: self.secrets,
            bosses: self.bosses,
            saves: self.saves,
            portals: self.portals,
        }
    }

    /// Fill color blended 55% toward white.
    pub fn border(&self) -> Color32 {
        let lighten = |c: u8| (c as f32 + (255.0 - c as f32) * 0.55).round() as u8;
        Color32::from_rgb(
            lighten(self.fill.r()),
            lighten(self.fill.g()),
            lighten(self.fill.b()),
        )
    }
}

/// Used when the file is missing or has no valid line.
pub fn default_biomes() -> Vec<Biome> {
    vec![
        Biome::new(text::CASTLE, 70, 80, 120),
        Biome::new(text::CAVES, 100, 70, 45),
        Biome::new(text::FOREST, 40, 95, 50),
        Biome::new(text::ICE, 60, 120, 140),
        Biome::new(text::LAVA, 130, 40, 25),
        Biome::new(text::ABYSS, 60, 30, 90),
    ]
}

/// One biome as stored in JSON:
/// `{"name": "Castle", "color": "#464E78", "corridors": 1, "secrets": 1, "bosses": 1,
/// "saves": 1, "portals": 1}`. The counts may be missing in older files.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    name: String,
    color: String,
    #[serde(default = "default_count")]
    corridors: u32,
    #[serde(default = "default_count")]
    secrets: u32,
    #[serde(default = "default_count")]
    bosses: u32,
    #[serde(default = "default_count")]
    saves: u32,
    #[serde(default = "default_count")]
    portals: u32,
}

impl From<&Biome> for Entry {
    fn from(b: &Biome) -> Entry {
        Entry {
            name: b.name.clone(),
            color: to_hex(b.fill),
            corridors: b.corridors,
            secrets: b.secrets,
            bosses: b.bosses,
            saves: b.saves,
            portals: b.portals,
        }
    }
}

fn default_count() -> u32 {
    DEFAULT_COUNT
}

/// One stored biome. A bad color fails. Counts are clamped to the editor limits.
pub fn from_entry(entry: Entry) -> Result<Biome, &'static str> {
    let Some(fill) = parse_hex(&entry.color) else {
        return Err(text::NO_VALID_BIOME);
    };
    Ok(Biome {
        name: entry.name,
        fill,
        corridors: entry.corridors.clamp(MIN_CORRIDORS, MAX_COUNT),
        secrets: entry.secrets.min(MAX_COUNT),
        bosses: entry.bosses.min(MAX_COUNT),
        saves: entry.saves.min(MAX_COUNT),
        portals: entry.portals.min(MAX_COUNT),
    })
}

/// Reads a JSON list of biomes. Entries with an invalid color are skipped.
/// Fails if the file cannot be read or parsed, or holds no valid biome.
pub fn read(path: &Path) -> Result<Vec<Biome>, Box<dyn Error>> {
    let entries: Vec<Entry> = storage::read_json(path)?;
    let biomes: Vec<Biome> = entries
        .into_iter()
        .filter_map(|entry| from_entry(entry).ok())
        .collect();
    if biomes.is_empty() {
        return Err(text::NO_VALID_BIOME.into());
    }
    Ok(biomes)
}

/// Biomes saved by the user, or the defaults if the file is missing or invalid.
pub fn load() -> Vec<Biome> {
    read(&storage::app_dir().join(files::BIOMES)).unwrap_or_else(|_| default_biomes())
}

/// Writes the biomes in the format read by `read`. Returns the path written to.
pub fn save(biomes: &[Biome]) -> Result<PathBuf, Box<dyn Error>> {
    let entries: Vec<Entry> = biomes.iter().map(Entry::from).collect();
    storage::write_json(files::BIOMES, &entries)
}
