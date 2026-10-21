//! Labels and buttons of the editor panel. Room names come from `rooms`.

use std::fmt::Display;

pub const WINDOW_TITLE: &str = "Editor";
pub const TABS: &[&str] = &["Settings", "Biomes"];

pub const USE_SEED: &str = "Use seed";
pub const GENERATE: &str = "Generate";
pub const SAVE_SETTINGS: &str = "Save settings";
pub const ADD: &str = "Add";
pub const IMPORT: &str = "Import";
pub const SAVE: &str = "Save";
pub const DELETE: &str = "Delete";
pub const ADD_SIZE: &str = "Add size";
pub const RESET: &str = "Reset";

pub const NAME: &str = "Name";
pub const COLOR: &str = "Color";
pub const WIDTH: &str = "Width";
pub const HEIGHT: &str = "Height";
pub const MINIMUM: &str = "Minimum";
pub const MAXIMUM: &str = "Maximum";

pub const SCREEN_SIZE: &str = "Screen size (px)";
pub const MAX_ROOMS_LABEL: &str = "Max rooms per biome (0 = no limit)";
pub const AUTOSAVE: &str = "Autosave";
pub const AUTOSAVE_DELAY: &str = "Autosave delay (s)";

pub fn min_rooms_label(max: impl Display) -> String {
    format!("Min rooms per biome (1 to {max})")
}

pub fn room_sizes_label(max_side: impl Display) -> String {
    format!("Room sizes (1 to {max_side})")
}

pub fn boss_sizes_label(max_side: impl Display) -> String {
    format!("Boss room sizes (1 to {max_side})")
}

pub fn corridor_sizes_label(max_side: impl Display) -> String {
    format!("Corridor room sizes (1 to {max_side})")
}
