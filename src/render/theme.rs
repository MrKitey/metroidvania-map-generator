//! Colors and sizes shared by the drawing code, and the color of each room.

use crate::biome::Biome;
use crate::icon_lib::{self, IconDef};
use crate::icons::{fallback_colors, room_icon_ids};
use crate::world::{Door, Room, RoomKind, World};
use egui::Color32;

pub const BACKGROUND: Color32 = Color32::from_rgb(15, 15, 22);
pub const LIGHT_BACKGROUND: Color32 = Color32::from_rgb(236, 236, 242);
pub const CORRIDOR_BORDER: Color32 = Color32::from_rgb(150, 150, 150);
/// Color of doors between two different biomes.
pub const TRANSITION_DOOR: Color32 = Color32::from_rgb(240, 230, 180);
pub const BUTTON_BG: Color32 = Color32::from_rgb(35, 35, 50);
pub const BUTTON_HOVER: Color32 = Color32::from_rgb(55, 55, 75);

/// Map canvas fill for the current UI theme.
pub fn background(dark: bool) -> Color32 {
    if dark { BACKGROUND } else { LIGHT_BACKGROUND }
}

/// Inner spacing of the buttons, popups and legends.
pub const PAD: f32 = 8.0;
/// Row height of the popups and the legends.
pub const ROW_H: f32 = 24.0;
pub const LEGEND_TEXT: f32 = 16.0;

fn darker(c: Color32) -> Color32 {
    Color32::from_rgb(
        (c.r() as f32 * 0.6) as u8,
        (c.g() as f32 * 0.6) as u8,
        (c.b() as f32 * 0.6) as u8,
    )
}

/// Fill and border of the first icon of a room, from the library or the fallbacks.
fn special_colors(r: &Room, icons: &[IconDef]) -> Option<(Color32, Color32)> {
    let id = room_icon_ids(r).next()?;
    if let Some(def) = icon_lib::find(icons, id) {
        return Some((def.fill, def.border));
    }
    fallback_colors(id)
}

/// Fill, border and whether the border is thicker than usual, in one icon lookup.
/// Rooms with a marker or a special kind stand out. Corridors use a darker fill.
pub fn room_style(r: &Room, biomes: &[Biome], icons: &[IconDef]) -> (Color32, Color32, bool) {
    if let Some((fill, border)) = special_colors(r, icons) {
        return (fill, border, true);
    }
    let biome = &biomes[r.biome];
    match r.kind {
        RoomKind::Normal => (biome.fill, biome.border(), false),
        RoomKind::Corridor => (darker(biome.fill), CORRIDOR_BORDER, false),
        RoomKind::Secret | RoomKind::Portal => (biome.fill, biome.border(), true),
    }
}

/// Biome color for a room. Corridors use a darker shade.
pub fn room_fill(r: &Room, biomes: &[Biome], icons: &[IconDef]) -> Color32 {
    room_style(r, biomes, icons).0
}

/// Color of a door. A door inside a biome takes the room color so it blends in,
/// a door between biomes stands out.
pub fn door_color(world: &World, biomes: &[Biome], icons: &[IconDef], d: &Door) -> Color32 {
    let (a, b) = (&world.rooms[d.a], &world.rooms[d.b]);
    if a.biome == b.biome {
        room_fill(b, biomes, icons)
    } else {
        TRANSITION_DOOR
    }
}
