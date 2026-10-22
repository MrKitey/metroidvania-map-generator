//! Overlays drawn on the map: shortcuts, legends, seed label and tooltips.

use std::fmt::Display;

/// Listed in the shortcuts popup, as (key, action).
pub const SHORTCUTS: &[(&str, &str)] = &[
    ("Ctrl+N", "New project"),
    ("Ctrl+O", "Load project"),
    ("Ctrl+S", "Save project"),
    ("Wheel", "Zoom"),
    ("Middle drag", "Pan"),
    ("Drag from Rooms", "Place a room"),
    ("Left drag empty", "Select rooms"),
    ("Left drag", "Move room or group"),
    ("Click biome", "Select its rooms"),
    ("Del / Backspace", "Delete room(s)"),
    ("Drag legend icon", "Set room type"),
    ("C", "Recenter"),
    ("G", "Grid"),
    ("B", "Boss room"),
    ("S", "Save room"),
    ("H", "Secret room"),
    ("P", "Portal room"),
    ("L", "Rooms"),
    ("M", "Biomes"),
    ("Tab", "Editor"),
    ("Esc", "Quit"),
];
/// Glyph on the shortcuts button.
pub const HELP_BUTTON: &str = "?";
/// Label of the editor button.
pub const EDITOR: &str = "Editor";
/// Label of the regenerate-map button.
pub const REGENERATE: &str = "Regenerate Map";
/// Label of the export button.
pub const EXPORT: &str = "Export";
pub const NEW: &str = "New";
pub const LOAD: &str = "Load";
pub const RECENT: &str = "Recent";
pub const SAVE: &str = "Save";
pub const BIOMES: &str = "Biomes";
/// Shown when the recent list has no path.
pub const NO_RECENT: &str = "None";

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

/// Room count in the header.
pub fn rooms(rooms: usize) -> String {
    format!("Rooms {rooms}")
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
