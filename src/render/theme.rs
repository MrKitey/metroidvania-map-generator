//! Colors and sizes shared by the drawing code, and the color of each room.

use crate::biome::Biome;
use crate::world::{Door, Marker, Room, RoomKind, World};
use egui::Color32;

pub const BACKGROUND: Color32 = Color32::from_rgb(15, 15, 22);
pub const CORRIDOR_BORDER: Color32 = Color32::from_rgb(150, 150, 150);
/// Color of doors between two different biomes.
pub const TRANSITION_DOOR: Color32 = Color32::from_rgb(240, 230, 180);
/// Rooms with a boss or save marker use these instead of their biome colors.
pub const BOSS_FILL: Color32 = Color32::from_rgb(0x80, 0x00, 0x00);
pub const BOSS_BORDER: Color32 = Color32::from_rgb(0xFF, 0x00, 0x00);
pub const SAVE_FILL: Color32 = Color32::from_rgb(0x00, 0x80, 0x00);
pub const SAVE_BORDER: Color32 = Color32::from_rgb(0x00, 0xFF, 0x00);
/// Secret and portal rooms use these too.
pub const SECRET_FILL: Color32 = Color32::from_rgb(0x80, 0x80, 0x00);
pub const SECRET_BORDER: Color32 = Color32::from_rgb(0xFF, 0xFF, 0x00);
pub const PORTAL_FILL: Color32 = Color32::from_rgb(0x00, 0x80, 0x80);
pub const PORTAL_BORDER: Color32 = Color32::from_rgb(0x00, 0xFF, 0xFF);
pub const BUTTON_BG: Color32 = Color32::from_rgb(35, 35, 50);
pub const BUTTON_HOVER: Color32 = Color32::from_rgb(55, 55, 75);

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

/// Biome color for a room. Corridors use a darker shade.
pub fn room_fill(r: &Room, biomes: &[Biome]) -> Color32 {
    match r.marker {
        Some(Marker::Boss) => return BOSS_FILL,
        Some(Marker::Save) => return SAVE_FILL,
        None => {}
    }
    let fill = biomes[r.biome].fill;
    match r.kind {
        RoomKind::Normal => fill,
        RoomKind::Secret => SECRET_FILL,
        RoomKind::Portal => PORTAL_FILL,
        RoomKind::Corridor => darker(fill),
    }
}

/// Border color of a room, and whether the border is thicker than usual.
/// Rooms with a marker or a special kind stand out.
pub fn room_border(r: &Room, biomes: &[Biome]) -> (Color32, bool) {
    match (r.marker, r.kind) {
        (Some(Marker::Boss), _) => (BOSS_BORDER, true),
        (Some(Marker::Save), _) => (SAVE_BORDER, true),
        (_, RoomKind::Secret) => (SECRET_BORDER, true),
        (_, RoomKind::Portal) => (PORTAL_BORDER, true),
        (_, RoomKind::Normal) => (biomes[r.biome].border(), false),
        (_, RoomKind::Corridor) => (CORRIDOR_BORDER, false),
    }
}

/// Color of a door. A door inside a biome takes the room color so it blends in,
/// a door between biomes stands out.
pub fn door_color(world: &World, biomes: &[Biome], d: &Door) -> Color32 {
    let (a, b) = (&world.rooms[d.a], &world.rooms[d.b]);
    if a.biome == b.biome {
        room_fill(b, biomes)
    } else {
        TRANSITION_DOOR
    }
}
