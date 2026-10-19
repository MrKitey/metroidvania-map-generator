//! Drawing of the map, the header and the tooltip.

use crate::biome::Biome;
use crate::view::View;
use crate::world::{Marker, Room, RoomKind, World};
use macroquad::prelude::*;

/// Height of the header bar, in screen pixels.
pub const TOP: f32 = 60.0;
/// Free space kept around the map when fitting it, in screen pixels.
pub const MARGIN: f32 = 20.0;
const BACKGROUND: Color = Color::from_rgba(15, 15, 22, 255);
const CORRIDOR_BORDER: Color = Color::from_rgba(150, 150, 150, 255);
/// Color of doors between two different biomes.
const TRANSITION_DOOR: Color = Color::from_rgba(240, 230, 180, 255);
const BOSS_COLOR: Color = Color::from_rgba(200, 30, 40, 255);
const SAVE_COLOR: Color = Color::from_rgba(40, 110, 220, 255);
const SEED_SIZE: u16 = 20;
/// Seconds the "copied" feedback stays on the seed label.
pub const COPIED_TIME: f64 = 1.5;

/// Listed in the shortcuts popup, as (key, action).
const SHORTCUTS: &[(&str, &str)] = &[
    ("R", "New map"),
    ("Arrows", "Map size"),
    ("Wheel", "Zoom"),
    ("Middle drag", "Pan"),
    ("C", "Recenter"),
    ("Left click", "Boss marker"),
    ("Right click", "Save marker"),
    ("Tab", "Editor"),
    ("Esc", "Quit"),
];
const HELP_TEXT: u16 = 24;
const HELP_H: f32 = 28.0;
/// Radius of the "?" icon circle.
const ICON_R: f32 = 10.0;
/// Inner spacing of the shortcuts button and popup.
const PAD: f32 = 8.0;
const ROW_H: f32 = 24.0;
const BUTTON_BG: Color = Color::from_rgba(35, 35, 50, 255);
const BUTTON_HOVER: Color = Color::from_rgba(55, 55, 75, 255);

/// Cell size in world units. Cells keep the viewport aspect ratio.
pub struct Layout {
    pub cw: f32,
    pub ch: f32,
    /// Size of one in-game screen in pixels. One cell stands for one screen.
    pub viewport: (u32, u32),
}

impl Layout {
    /// A cell is one unit tall and as wide as the viewport aspect ratio.
    pub fn new(viewport: (u32, u32)) -> Layout {
        Layout {
            cw: viewport.0 as f32 / viewport.1 as f32,
            ch: 1.0,
            viewport,
        }
    }

    /// Cell containing world point `p`. Uses floor so negative cells are correct.
    pub fn cell_at(&self, p: Vec2) -> (i32, i32) {
        ((p.x / self.cw).floor() as i32, (p.y / self.ch).floor() as i32)
    }
}

fn darker(c: Color) -> Color {
    Color::new(c.r * 0.6, c.g * 0.6, c.b * 0.6, c.a)
}

/// Biome color for a room. Corridors use a darker shade.
fn room_fill(world: &World, biomes: &[Biome], idx: usize) -> Color {
    let r = &world.rooms[idx];
    let fill = biomes[r.biome].fill;
    match r.kind {
        RoomKind::Normal => fill,
        RoomKind::Corridor => darker(fill),
    }
}

/// Everything about the current frame that the renderer needs besides the world.
pub struct Frame<'a> {
    pub layout: &'a Layout,
    pub view: &'a View,
    pub camera: &'a Camera2D,
    pub seed: u64,
    /// True while the seed label shows the "copied" feedback.
    pub copied: bool,
    /// Screen width taken by the editor panel on the right.
    pub reserved: f32,
    pub hovered: Option<usize>,
    pub help_open: bool,
}

/// Square shortcuts button, in the top right corner of the header, left of `reserved`.
pub fn help_button_rect(reserved: f32) -> Rect {
    Rect::new(screen_width() - reserved - MARGIN - HELP_H, (TOP - HELP_H) / 2.0, HELP_H, HELP_H)
}

/// Widths of the key column and the action column of the popup.
fn help_columns() -> (f32, f32) {
    let width = |text: &str| measure_text(text, None, HELP_TEXT, 1.0).width;
    let key_w = SHORTCUTS.iter().map(|s| width(s.0)).fold(0.0, f32::max);
    let action_w = SHORTCUTS.iter().map(|s| width(s.1)).fold(0.0, f32::max);
    (key_w, action_w)
}

/// Shortcuts popup, just below the button and right aligned with it.
pub fn help_popup_rect(reserved: f32) -> Rect {
    let button = help_button_rect(reserved);
    let (key_w, action_w) = help_columns();
    let w = PAD + key_w + 2.0 * PAD + action_w + PAD;
    let h = PAD + SHORTCUTS.len() as f32 * ROW_H + PAD;
    Rect::new(button.right() - w, button.bottom() + 6.0, w, h)
}

/// Seed label text, clickable area and text baseline. The label sits in the
/// bottom right corner of the map area, left of `reserved`.
fn seed_label(seed: u64, copied: bool, reserved: f32) -> (String, Rect, f32) {
    let text = if copied {
        format!("Seed {seed} (copied)")
    } else {
        format!("Seed {seed}")
    };
    let dim = measure_text(&text, None, SEED_SIZE, 1.0);
    let x = screen_width() - reserved - MARGIN - dim.width;
    let y = screen_height() - MARGIN - dim.height;
    (text, Rect::new(x, y, dim.width, dim.height), y + dim.offset_y)
}

/// Clickable area of the seed label, in screen pixels.
pub fn seed_rect(seed: u64, copied: bool, reserved: f32) -> Rect {
    seed_label(seed, copied, reserved).1
}

/// Draws one frame. Rooms and doors use the map camera, the rest uses screen pixels.
pub fn draw_world(world: &World, biomes: &[Biome], f: &Frame) {
    clear_background(BACKGROUND);
    set_camera(f.camera);
    let Layout { cw, ch, .. } = *f.layout;
    // Border and door thickness, in world units.
    let t = cw.min(ch) * 0.12;

    for (i, r) in world.rooms.iter().enumerate() {
        let (x, y) = (r.x as f32 * cw, r.y as f32 * ch);
        let (w, h) = (r.w as f32 * cw, r.h as f32 * ch);
        let border = match r.kind {
            RoomKind::Normal => biomes[r.biome].border(),
            RoomKind::Corridor => CORRIDOR_BORDER,
        };
        draw_rectangle(x, y, w, h, room_fill(world, biomes, i));
        draw_rectangle_lines(x, y, w, h, t, border);
        if f.hovered == Some(i) {
            draw_rectangle(x, y, w, h, Color::new(1.0, 1.0, 1.0, 0.15));
        }
    }

    // Doors are drawn over the borders to open a gap in them. A door inside a biome
    // takes the room color so it blends in, a door between biomes stands out.
    for d in &world.doors {
        let color = if world.rooms[d.a].biome == world.rooms[d.b].biome {
            room_fill(world, biomes, d.b)
        } else {
            TRANSITION_DOOR
        };
        let (x, y) = (d.x as f32 * cw, d.y as f32 * ch);
        if d.vertical {
            draw_rectangle(x - t, y + ch * 0.3, 2.0 * t, ch * 0.4, color);
        } else {
            draw_rectangle(x + cw * 0.3, y - t, cw * 0.4, 2.0 * t, color);
        }
    }

    // Markers are drawn in screen space so the letters stay sharp at any zoom.
    set_default_camera();
    for r in &world.rooms {
        let Some(marker) = r.marker else { continue };
        let (letter, color) = match marker {
            Marker::Boss => ("B", BOSS_COLOR),
            Marker::Save => ("S", SAVE_COLOR),
        };
        let center = vec2((r.x as f32 + r.w as f32 / 2.0) * cw, (r.y as f32 + r.h as f32 / 2.0) * ch);
        let c = f.camera.world_to_screen(center);
        let radius = (cw.min(ch) * 0.35 * f.view.scale).max(6.0);
        draw_circle(c.x, c.y, radius, color);
        draw_circle_lines(c.x, c.y, radius, 1.5, WHITE);
        let size = (radius * 1.4) as u16;
        let dim = measure_text(letter, None, size, 1.0);
        draw_text(letter, c.x - dim.width / 2.0, c.y + dim.height / 2.0, size as f32, WHITE);
    }

    // Opaque header so the map does not show through the text when panned under it.
    draw_rectangle(0.0, 0.0, screen_width(), TOP, BACKGROUND);
    draw_text(format!("Rooms {}", world.rooms.len()), MARGIN, 22.0, 20.0, WHITE);

    // Legend of the biomes used in this map.
    let mut lx = MARGIN;
    for &b in &world.biomes {
        let biome = &biomes[b];
        draw_rectangle(lx, 34.0, 14.0, 14.0, biome.fill);
        draw_rectangle_lines(lx, 34.0, 14.0, 14.0, 2.0, biome.border());
        let dim = draw_text(&biome.name, lx + 20.0, 46.0, 20.0, biome.border());
        lx += dim.width + 40.0;
    }

    // Backdrop keeps the label readable over the map. Brighter on hover to show it is clickable.
    let (text, rect, baseline) = seed_label(f.seed, f.copied, f.reserved);
    let hover = rect.contains(Vec2::from(mouse_position()));
    draw_rectangle(rect.x - 6.0, rect.y - 6.0, rect.w + 12.0, rect.h + 12.0, BACKGROUND);
    let color = if hover { WHITE } else { LIGHTGRAY };
    draw_text(&text, rect.x, baseline, SEED_SIZE as f32, color);

    if let Some(i) = f.hovered {
        draw_tooltip(&world.rooms[i], biomes, f.layout);
    }
    draw_help(f.reserved, f.help_open);
}

/// Shortcuts button with a "?" icon, and the popup listing the shortcuts when open.
fn draw_help(reserved: f32, open: bool) {
    let button = help_button_rect(reserved);
    let hover = button.contains(Vec2::from(mouse_position()));
    let bg = if open || hover { BUTTON_HOVER } else { BUTTON_BG };
    draw_rectangle(button.x, button.y, button.w, button.h, bg);

    let c = button.center();
    draw_circle_lines(c.x, c.y, ICON_R, 1.5, WHITE);
    let q = measure_text("?", None, HELP_TEXT, 1.0);
    draw_text("?", c.x - q.width / 2.0, c.y + q.height / 2.0, HELP_TEXT as f32, WHITE);

    if !open {
        return;
    }
    let popup = help_popup_rect(reserved);
    draw_rectangle(popup.x, popup.y, popup.w, popup.h, Color::new(0.0, 0.0, 0.0, 0.9));
    draw_rectangle_lines(popup.x, popup.y, popup.w, popup.h, 1.0, CORRIDOR_BORDER);
    let action_x = popup.x + PAD + help_columns().0 + 2.0 * PAD;
    for (i, (key, action)) in SHORTCUTS.iter().enumerate() {
        let baseline = popup.y + PAD + i as f32 * ROW_H + ROW_H * 0.7;
        draw_text(key, popup.x + PAD, baseline, HELP_TEXT as f32, TRANSITION_DOOR);
        draw_text(action, action_x, baseline, HELP_TEXT as f32, WHITE);
    }
}

/// Shows the room name and its size, in cells and in pixels, next to the cursor.
fn draw_tooltip(r: &Room, biomes: &[Biome], layout: &Layout) {
    let name = match r.kind {
        RoomKind::Normal => biomes[r.biome].name.as_str(),
        RoomKind::Corridor => "Corridor",
    };
    let text = format!(
        "{}  {}x{} : {}x{} px",
        name,
        r.w,
        r.h,
        r.w * layout.viewport.0,
        r.h * layout.viewport.1
    );
    let (mx, my) = mouse_position();
    let dim = measure_text(&text, None, 18, 1.0);
    draw_rectangle(mx + 12.0, my + 8.0, dim.width + 12.0, 24.0, Color::new(0.0, 0.0, 0.0, 0.8));
    draw_text(&text, mx + 18.0, my + 25.0, 18.0, WHITE);
}
