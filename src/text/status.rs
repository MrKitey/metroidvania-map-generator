//! Feedback messages, shown in the settings panel and in the header.

use std::fmt::Display;
use std::path::Path;

pub const INVALID_SEED: &str = "Invalid seed";
pub const AT_LEAST_ONE_BIOME: &str = "At least one biome is required";
pub const NO_VALID_ICON: &str = "No valid special room icon";
pub const AT_LEAST_ONE_ICON: &str = "At least one special room is required";
pub const BAD_VERSION: &str = "Unsupported project version";
pub const EMPTY_MAP: &str = "Empty map";
pub const EMPTY_ROOM: &str = "Empty room";
pub const OVERLAP: &str = "Rooms overlap";
pub const BAD_INDEX: &str = "Invalid project index";
pub const NOT_MMG: &str = "Not a project file";
pub const MISSING: &str = "Missing project file";
pub const DISCARD_TITLE: &str = "Unsaved changes";
pub const DISCARD: &str = "Discard unsaved changes?";

pub fn saved_to(path: &Path) -> String {
    format!("Saved to {}", path.display())
}

pub fn loaded_from(path: &Path) -> String {
    format!("Loaded {}", path.display())
}

pub fn imported(path: &Path) -> String {
    format!("Imported {}", path.display())
}

pub fn error(e: impl Display) -> String {
    format!("Error: {e}")
}

pub fn exported(name: &str) -> String {
    format!("Exported {name}")
}

pub fn export_failed(e: impl Display) -> String {
    format!("Export failed: {e}")
}
