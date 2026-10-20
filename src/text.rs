//! Every text of the app: window title, labels, messages, file names and default names.
//!
//! One module per area. Fixed texts are constants, texts built from values are small
//! functions, so a format string never appears in the drawing code. Not here: SVG
//! markup, JSON field names, panic messages and test data.

use std::fmt::Display;
use std::path::Path;

pub mod app {
    pub const WINDOW_TITLE: &str = "Metroidvania Map Generator";
}

/// File and folder names, and the dialog filters built on them.
pub mod files {
    /// Folder created in the user's Documents to hold the app files.
    pub const APP_DIR: &str = "Metroidvania Map Generator";
    pub const BIOMES: &str = "biomes.json";
    pub const SETTINGS: &str = "settings.json";
    pub const JSON_FILTER: &str = "JSON";
    pub const JSON_EXT: &str = "json";
    pub const ZIP_FILTER: &str = "Zip";
    pub const ZIP_EXT: &str = "zip";
    /// Files inside the export zip.
    pub const EXPORT_PNG: &str = "map.png";
    pub const EXPORT_SVG: &str = "map.svg";
    pub const EXPORT_JSON: &str = "map.json";

    /// Default name of the export zip.
    pub fn zip_name(seed: u64) -> String {
        format!("map-{seed}.{ZIP_EXT}")
    }
}

/// Names of the room kinds, shared by the room picker and the biome sliders.
pub mod rooms {
    pub const ROOMS: &str = "Rooms";
    pub const BOSS_ROOMS: &str = "Boss rooms";
    pub const SECRET_ROOMS: &str = "Secret rooms";
    pub const SAVE_ROOMS: &str = "Save rooms";
    pub const PORTAL_ROOMS: &str = "Portal rooms";
    pub const CORRIDORS: &str = "Corridors";
}

/// Names of the icons, used as ids in the SVG export.
pub mod icons {
    pub const BOSS: &str = "boss";
    pub const SAVE: &str = "save";
    pub const SECRET: &str = "secret";
    pub const PORTAL: &str = "portal";
}

/// Biome names created by the app.
pub mod biomes {
    pub const CASTLE: &str = "Castle";
    pub const CAVES: &str = "Caves";
    pub const FOREST: &str = "Forest";
    pub const ICE: &str = "Ice";
    pub const LAVA: &str = "Lava";
    pub const ABYSS: &str = "Abyss";
    /// Name of a biome added from the editor.
    pub const NEW: &str = "New";
    pub const NO_VALID_BIOME: &str = "no valid biome in file";
}

/// Overlays drawn on the map: shortcuts, legends, seed label and tooltips.
pub mod hud {
    use super::Display;

    /// Listed in the shortcuts popup, as (key, action).
    pub const SHORTCUTS: &[(&str, &str)] = &[
        ("R", "New map"),
        ("Arrows", "Map size"),
        ("Wheel", "Zoom"),
        ("Middle drag", "Pan"),
        ("Right click", "Place a room"),
        ("Left drag", "Move a room"),
        ("Drag legend icon", "Set room type"),
        ("C", "Recenter"),
        ("G", "Grid"),
        ("B", "Boss room"),
        ("S", "Save room"),
        ("H", "Secret room"),
        ("P", "Portal room"),
        ("Tab", "Editor"),
        ("Esc", "Quit"),
    ];
    /// Glyph on the shortcuts button.
    pub const HELP_BUTTON: &str = "?";

    /// Room type legend.
    pub const LEGEND_BOSS: &str = "Boss";
    pub const LEGEND_SAVE: &str = "Save";
    pub const LEGEND_SECRET: &str = "Secret";
    pub const LEGEND_PORTAL: &str = "Portal";

    /// Room name in the tooltip.
    pub const CORRIDOR: &str = "Corridor";
    pub const BOSS_SUFFIX: &str = " (boss)";
    pub const SAVE_SUFFIX: &str = " (save)";

    pub fn seed(seed: u64, copied: bool) -> String {
        if copied {
            format!("Seed {seed} (copied)")
        } else {
            format!("Seed {seed}")
        }
    }

    /// Room count, followed by the notice when there is one.
    pub fn header(rooms: usize, notice: Option<&str>) -> String {
        match notice {
            Some(notice) => format!("Rooms {rooms}    {notice}"),
            None => format!("Rooms {rooms}"),
        }
    }

    pub fn secret_room(biome: &str) -> String {
        format!("{biome} (secret)")
    }

    pub fn portal_room(biome: &str) -> String {
        format!("{biome} (portal)")
    }

    /// Room name with its size in cells, then in pixels.
    pub fn tooltip(name: &str, (w, h): (u32, u32), (px_w, px_h): (u32, u32)) -> String {
        format!("{name}  {w}x{h} : {px_w}x{px_h} px")
    }

    /// Size in cells and position in pixels of the room being placed or moved.
    pub fn ghost_label(w: u32, h: u32, x: impl Display, y: impl Display) -> String {
        format!("{w}x{h} at {x}, {y} px")
    }
}

/// Labels and buttons of the editor panel. Room names come from `rooms`.
pub mod editor {
    use super::Display;

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
}

/// Feedback messages, shown in the editor panel and in the header.
pub mod status {
    use super::{Display, Path};

    pub const INVALID_SEED: &str = "Invalid seed";
    pub const AT_LEAST_ONE_BIOME: &str = "At least one biome is required";

    pub fn saved_to(path: &Path) -> String {
        format!("Saved to {}", path.display())
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
}
