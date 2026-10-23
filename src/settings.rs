//! Generation settings and their storage in a JSON file.

use crate::storage;
use crate::text::files;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::PathBuf;

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
/// Allowed boss room sizes.
pub const DEFAULT_BOSS_SIZES: &[(u32, u32)] = &[(2, 2), (2, 3), (3, 3)];
/// Allowed corridor sizes. Long and thin, 3 to 5 cells.
pub const DEFAULT_CORRIDOR_SIZES: &[(u32, u32)] = &[(3, 1), (4, 1), (5, 1), (1, 3), (1, 4), (1, 5)];
/// Autosave starts on. The delay is in seconds.
pub const DEFAULT_AUTOSAVE: bool = true;
pub const DEFAULT_AUTOSAVE_SECS: u64 = 30;
pub const MIN_AUTOSAVE_SECS: u64 = 5;
pub const MAX_AUTOSAVE_SECS: u64 = 3600;
/// Special-room icon size on the map. Share of a cell is `icon_size / 10`.
pub const DEFAULT_ICON_SIZE: u32 = 7;
pub const MIN_ICON_SIZE: u32 = 1;
pub const MAX_ICON_SIZE: u32 = 20;

/// Values are stored as read. The settings panel validates them when parsing its fields.
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
    /// (width, height) in cells.
    pub boss_sizes: Vec<(u32, u32)>,
    /// (width, height) in cells.
    pub corridor_sizes: Vec<(u32, u32)>,
    /// Writes the open project again after `autosave_secs` without edits.
    /// Kept in settings.json only. A project file does not store it.
    pub autosave: bool,
    /// Seconds between the last edit and the next autosave.
    pub autosave_secs: u64,
    /// Dark UI theme when true. Kept in settings.json only.
    pub dark: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            viewport: DEFAULT_VIEWPORT,
            min_rooms: DEFAULT_MIN_ROOMS,
            max_rooms: DEFAULT_MAX_ROOMS,
            room_sizes: DEFAULT_ROOM_SIZES.to_vec(),
            boss_sizes: DEFAULT_BOSS_SIZES.to_vec(),
            corridor_sizes: DEFAULT_CORRIDOR_SIZES.to_vec(),
            autosave: DEFAULT_AUTOSAVE,
            autosave_secs: DEFAULT_AUTOSAVE_SECS,
            dark: true,
        }
    }
}

/// Clamped icon size used for drawing and export.
pub fn clamp_icon_size(size: u32) -> u32 {
    size.clamp(MIN_ICON_SIZE, MAX_ICON_SIZE)
}

/// Cell share used by map icons for a given size setting.
pub fn icon_share(size: u32) -> f32 {
    clamp_icon_size(size) as f32 / 10.0
}

/// Saved settings, or the defaults if the file is missing or invalid.
pub fn load() -> Settings {
    storage::read_json(&storage::app_dir().join(files::SETTINGS)).unwrap_or_default()
}

/// Returns the path written to.
pub fn save(settings: &Settings) -> Result<PathBuf, Box<dyn Error>> {
    storage::write_json(files::SETTINGS, settings)
}
