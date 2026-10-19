//! Colors and sizes shared by the drawing code, and the color of each room.

use crate::biome::Biome;
use crate::world::{Door, Marker, Room, RoomKind, World};
use macroquad::prelude::*;

pub const BACKGROUND: Color = Color::from_rgba(15, 15, 22, 255);
pub const CORRIDOR_BORDER: Color = Color::from_rgba(150, 150, 150, 255);
/// Color of doors between two different biomes.
pub const TRANSITION_DOOR: Color = Color::from_rgba(240, 230, 180, 255);
/// Rooms with a boss or save marker use these instead of their biome colors.
pub const BOSS_FILL: Color = Color::from_rgba(0x80, 0x00, 0x00, 255);
pub const BOSS_BORDER: Color = Color::from_rgba(0xFF, 0x00, 0x00, 255);
pub const SAVE_FILL: Color = Color::from_rgba(0x00, 0x80, 0x00, 255);
pub const SAVE_BORDER: Color = Color::from_rgba(0x00, 0xFF, 0x00, 255);
/// Secret and portal rooms use these too.
pub const SECRET_FILL: Color = Color::from_rgba(0x80, 0x80, 0x00, 255);
pub const SECRET_BORDER: Color = Color::from_rgba(0xFF, 0xFF, 0x00, 255);
pub const PORTAL_FILL: Color = Color::from_rgba(0x00, 0x80, 0x80, 255);
pub const PORTAL_BORDER: Color = Color::from_rgba(0x00, 0xFF, 0xFF, 255);
pub const BUTTON_BG: Color = Color::from_rgba(35, 35, 50, 255);
pub const BUTTON_HOVER: Color = Color::from_rgba(55, 55, 75, 255);

/// Inner spacing of the buttons, popups and legends.
pub const PAD: f32 = 8.0;
/// Row height of the popups and the legends.
pub const ROW_H: f32 = 24.0;
pub const LEGEND_TEXT: u16 = 20;

fn darker(c: Color) -> Color {
    Color::new(c.r * 0.6, c.g * 0.6, c.b * 0.6, c.a)
}

/// Biome color for a room. Corridors use a darker shade.
pub fn room_fill(r: &Room, biomes: &[Biome]) -> Color {
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
pub fn room_border(r: &Room, biomes: &[Biome]) -> (Color, bool) {
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
pub fn door_color(world: &World, biomes: &[Biome], d: &Door) -> Color {
    let (a, b) = (&world.rooms[d.a], &world.rooms[d.b]);
    if a.biome == b.biome {
        room_fill(b, biomes)
    } else {
        TRANSITION_DOOR
    }
}
