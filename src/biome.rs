//! Biome definitions and their storage in a JSON file.

use crate::storage;
use macroquad::color::Color;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::{Path, PathBuf};

/// Read at startup and written by the editor, in the app folder.
const BIOMES_FILE: &str = "biomes.json";
pub const DEFAULT_SECRETS: u32 = 1;
pub const MAX_SECRETS: u32 = 5;

pub struct Biome {
    pub name: String,
    pub fill: Color,
    /// Number of secret rooms added to the biome, up to `MAX_SECRETS`.
    pub secrets: u32,
}

impl Biome {
    pub fn new(name: &str, r: u8, g: u8, b: u8) -> Biome {
        Biome {
            name: name.to_owned(),
            fill: Color::from_rgba(r, g, b, 255),
            secrets: DEFAULT_SECRETS,
        }
    }

    /// Fill color blended 55% toward white.
    pub fn border(&self) -> Color {
        let lighten = |c: f32| c + (1.0 - c) * 0.55;
        Color::new(lighten(self.fill.r), lighten(self.fill.g), lighten(self.fill.b), 1.0)
    }
}

/// Used when the file is missing or has no valid line.
pub fn default_biomes() -> Vec<Biome> {
    vec![
        Biome::new("Castle", 70, 80, 120),
        Biome::new("Caves", 100, 70, 45),
        Biome::new("Forest", 40, 95, 50),
        Biome::new("Ice", 60, 120, 140),
        Biome::new("Lava", 130, 40, 25),
        Biome::new("Abyss", 60, 30, 90),
    ]
}

/// One biome as stored in JSON: `{"name": "Castle", "color": "#464E78", "secrets": 1}`.
/// `secrets` may be missing in older files.
#[derive(Serialize, Deserialize)]
struct Entry {
    name: String,
    color: String,
    #[serde(default = "default_secrets")]
    secrets: u32,
}

fn default_secrets() -> u32 {
    DEFAULT_SECRETS
}

/// Reads a JSON list of biomes. Entries with an invalid color are skipped.
/// Fails if the file cannot be read or parsed, or holds no valid biome.
pub fn read(path: &Path) -> Result<Vec<Biome>, Box<dyn Error>> {
    let entries: Vec<Entry> = storage::read_json(path)?;
    let biomes: Vec<Biome> = entries
        .into_iter()
        .filter_map(|e| {
            Some(Biome {
                fill: parse_hex(&e.color)?,
                name: e.name,
                secrets: e.secrets.min(MAX_SECRETS),
            })
        })
        .collect();
    if biomes.is_empty() {
        return Err("no valid biome in file".into());
    }
    Ok(biomes)
}

/// Biomes saved by the user, or the defaults if the file is missing or invalid.
pub fn load() -> Vec<Biome> {
    read(&storage::app_dir().join(BIOMES_FILE)).unwrap_or_else(|_| default_biomes())
}

/// Writes the biomes in the format read by `read`. Returns the path written to.
pub fn save(biomes: &[Biome]) -> Result<PathBuf, Box<dyn Error>> {
    let entries: Vec<Entry> = biomes
        .iter()
        .map(|b| Entry {
            name: b.name.clone(),
            color: to_hex(b.fill),
            secrets: b.secrets,
        })
        .collect();
    storage::write_json(BIOMES_FILE, &entries)
}

/// Formats a color as `#RRGGBB`.
pub fn to_hex(color: Color) -> String {
    let [r, g, b, _]: [u8; 4] = color.into();
    format!("#{r:02X}{g:02X}{b:02X}")
}

/// Parses `#RRGGBB`. Returns None if the text does not match.
pub fn parse_hex(text: &str) -> Option<Color> {
    let hex = text.trim().strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    Some(Color::from_rgba(channel(0)?, channel(2)?, channel(4)?, 255))
}
