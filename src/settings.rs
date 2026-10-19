//! Generation settings and their storage in a JSON file.

use crate::storage;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::PathBuf;

/// Read at startup and written by the editor, in the app folder.
const SETTINGS_FILE: &str = "settings.json";
pub const DEFAULT_VIEWPORT: (u32, u32) = (640, 360);
pub const DEFAULT_MIN_ROOMS: usize = 5;
pub const DEFAULT_MAX_ROOMS: usize = 50;
/// Allowed room sizes, as (width, height) in cells.
pub const DEFAULT_ROOM_SIZES: &[(u32, u32)] = &[
    (1, 1),
    (2, 1),
    (1, 2),
    (2, 2),
    (3, 1),
    (1, 3),
    (3, 2),
    (3, 3),
    (4, 2),
    (4, 3),
    (1, 5),
];

/// Values are stored as read. The editor validates them when parsing its fields.
/// Fields missing from the file take their default value.
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Size of one in-game screen in pixels.
    pub viewport: (u32, u32),
    pub min_rooms: usize,
    /// 0 means no limit.
    pub max_rooms: usize,
    /// (width, height) in cells.
    pub room_sizes: Vec<(u32, u32)>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            viewport: DEFAULT_VIEWPORT,
            min_rooms: DEFAULT_MIN_ROOMS,
            max_rooms: DEFAULT_MAX_ROOMS,
            room_sizes: DEFAULT_ROOM_SIZES.to_vec(),
        }
    }
}

/// Saved settings, or the defaults if the file is missing or invalid.
pub fn load() -> Settings {
    storage::read_json(&storage::app_dir().join(SETTINGS_FILE)).unwrap_or_default()
}

/// Returns the path written to.
pub fn save(settings: &Settings) -> Result<PathBuf, Box<dyn Error>> {
    storage::write_json(SETTINGS_FILE, settings)
}
