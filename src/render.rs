//! Drawing of the map, the header and the tooltip.

use crate::biome::Biome;
use crate::icons::Icons;
use crate::view::View;
use crate::world::{Marker, Room, RoomKind, World};
use macroquad::prelude::*;

/// Height of the header bar, in screen pixels.
pub const TOP: f32 = 40.0;
/// Free space kept around the map when fitting it, in screen pixels.
pub const MARGIN: f32 = 20.0;
const BACKGROUND: Color = Color::from_rgba(15, 15, 22, 255);
const CORRIDOR_BORDER: Color = Color::from_rgba(150, 150, 150, 255);
/// Color of doors between two different biomes.
const TRANSITION_DOOR: Color = Color::from_rgba(240, 230, 180, 255);
/// Rooms with a boss or save marker use these instead of their biome colors.
const BOSS_FILL: Color = Color::from_rgba(0x80, 0x00, 0x00, 255);
const BOSS_BORDER: Color = Color::from_rgba(0xFF, 0x00, 0x00, 255);
const SAVE_FILL: Color = Color::from_rgba(0x00, 0x80, 0x00, 255);
const SAVE_BORDER: Color = Color::from_rgba(0x00, 0xFF, 0x00, 255);
/// Secret and portal rooms use these too.
const SECRET_FILL: Color = Color::from_rgba(0x80, 0x80, 0x00, 255);
const SECRET_BORDER: Color = Color::from_rgba(0xFF, 0xFF, 0x00, 255);
const PORTAL_FILL: Color = Color::from_rgba(0x00, 0x80, 0x80, 255);
const PORTAL_BORDER: Color = Color::from_rgba(0x00, 0xFF, 0xFF, 255);
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
    ("B", "Boss room"),
    ("S", "Save room"),
    ("H", "Secret room"),
    ("P", "Portal room"),
    ("Tab", "Editor"),
    ("Esc", "Quit"),
];
const HELP_TEXT: u16 = 24;
const HELP_H: f32 = 28.0;
/// Radius of the "?" icon circle.
const ICON_R: f32 = 10.0;
/// Inner spacing of the shortcuts button and popup.
const PAD: f32 = 8.0;
/// Row height of the shortcuts popup and the biome legend.
const ROW_H: f32 = 24.0;
const LEGEND_TEXT: u16 = 20;
/// Side of the biome color square in the legend.
const SWATCH: f32 = 14.0;
/// Side of the room type icon box in the legend, and of the icon inside it.
const ICON_BOX: f32 = 20.0;
const ICON_PX: u32 = 16;
/// Space between the biome legend and the room type legend.
const LEGEND_GAP: f32 = 8.0;

/// Icon of a legend entry.
enum LegendIcon {
    Marker(Marker),
    Kind(RoomKind),
}

/// Room types listed in the legend, as (name, fill, border, icon).
const LEGEND_ROOMS: &[(&str, Color, Color, LegendIcon)] = &[
    (
        "Boss",
        BOSS_FILL,
        BOSS_BORDER,
        LegendIcon::Marker(Marker::Boss),
    ),
    (
        "Save",
        SAVE_FILL,
        SAVE_BORDER,
        LegendIcon::Marker(Marker::Save),
    ),
    (
        "Secret",
        SECRET_FILL,
        SECRET_BORDER,
        LegendIcon::Kind(RoomKind::Secret),
    ),
    (
        "Portal",
        PORTAL_FILL,
        PORTAL_BORDER,
        LegendIcon::Kind(RoomKind::Portal),
    ),
];
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
        (
            (p.x / self.cw).floor() as i32,
            (p.y / self.ch).floor() as i32,
        )
    }
}

fn darker(c: Color) -> Color {
    Color::new(c.r * 0.6, c.g * 0.6, c.b * 0.6, c.a)
}

/// Biome color for a room. Corridors use a darker shade.
fn room_fill(world: &World, biomes: &[Biome], idx: usize) -> Color {
    let r = &world.rooms[idx];
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
    /// Short message shown in the header, such as the export result.
    pub notice: Option<&'a str>,
}

/// Square shortcuts button, in the top right corner of the header, left of `reserved`.
pub fn help_button_rect(reserved: f32) -> Rect {
    Rect::new(
        screen_width() - reserved - MARGIN - HELP_H,
        (TOP - HELP_H) / 2.0,
        HELP_H,
        HELP_H,
    )
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
    (
        text,
        Rect::new(x, y, dim.width, dim.height),
        y + dim.offset_y,
    )
}

/// Clickable area of the seed label, in screen pixels.
pub fn seed_rect(seed: u64, copied: bool, reserved: f32) -> Rect {
    seed_label(seed, copied, reserved).1
}

/// Draws one frame. The map uses the map camera, the rest uses screen pixels.
/// `legend_icons` is a second set used at a fixed size, so the map zoom does not
/// make the legend icons rasterize again every frame.
pub fn draw_world(
    world: &World,
    biomes: &[Biome],
    f: &Frame,
    icons: &mut Icons,
    legend_icons: &mut Icons,
) {
    clear_background(BACKGROUND);
    set_camera(f.camera);
    draw_map(world, biomes, f.layout, f.view.scale, f.hovered, icons);
    set_default_camera();

    // Opaque header so the map does not show through the text when panned under it.
    draw_rectangle(0.0, 0.0, screen_width(), TOP, BACKGROUND);
    let mut info = format!("Rooms {}", world.rooms.len());
    if let Some(notice) = f.notice {
        info = format!("{info}    {notice}");
    }
    draw_text(info, MARGIN, TOP / 2.0 + 7.0, 20.0, WHITE);
    let biomes_bottom = draw_legend(world, biomes);
    draw_room_legend(biomes_bottom + LEGEND_GAP, legend_icons);

    // Backdrop keeps the label readable over the map. Brighter on hover to show it is clickable.
    let (text, rect, baseline) = seed_label(f.seed, f.copied, f.reserved);
    let hover = rect.contains(Vec2::from(mouse_position()));
    draw_rectangle(
        rect.x - 6.0,
        rect.y - 6.0,
        rect.w + 12.0,
        rect.h + 12.0,
        BACKGROUND,
    );
    let color = if hover { WHITE } else { LIGHTGRAY };
    draw_text(&text, rect.x, baseline, SEED_SIZE as f32, color);

    if let Some(i) = f.hovered {
        draw_tooltip(&world.rooms[i], biomes, f.layout);
    }
    draw_export_button(f.reserved);
    draw_help(f.reserved, f.help_open);
}

/// Draws rooms, doors and icons in world units, with the camera already set.
/// `scale` is pixels per world unit, used to rasterize icons at their drawn size and
/// to size the boss border in pixels.
pub fn draw_map(
    world: &World,
    biomes: &[Biome],
    layout: &Layout,
    scale: f32,
    hovered: Option<usize>,
    icons: &mut Icons,
) {
    let Layout { cw, ch, .. } = *layout;
    // Border and door thickness, in world units.
    let t = cw.min(ch) * 0.12;

    for (i, r) in world.rooms.iter().enumerate() {
        let (x, y) = (r.x as f32 * cw, r.y as f32 * ch);
        let (w, h) = (r.w as f32 * cw, r.h as f32 * ch);
        let (border, thickness) = match (r.marker, r.kind) {
            // One screen pixel thicker than the other rooms, at any zoom.
            (Some(Marker::Boss), _) => (BOSS_BORDER, t + 1.0 / scale),
            (Some(Marker::Save), _) => (SAVE_BORDER, t + 1.0 / scale),
            (_, RoomKind::Secret) => (SECRET_BORDER, t + 1.0 / scale),
            (_, RoomKind::Portal) => (PORTAL_BORDER, t + 1.0 / scale),
            (_, RoomKind::Normal) => (biomes[r.biome].border(), t),
            (_, RoomKind::Corridor) => (CORRIDOR_BORDER, t),
        };
        draw_rectangle(x, y, w, h, room_fill(world, biomes, i));
        draw_rectangle_lines(x, y, w, h, thickness, border);
        if hovered == Some(i) {
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

    // Icons are rasterized at their size in pixels, so they stay sharp at any zoom.
    // One raster size for all icons, otherwise the cache would rasterize every frame.
    let side_px = (cw.min(ch) * 0.7 * scale).max(12.0);
    let px = side_px.ceil() as u32;
    let side = side_px / scale;
    for r in &world.rooms {
        let has_kind_icon = matches!(r.kind, RoomKind::Secret | RoomKind::Portal);
        if !has_kind_icon && r.marker.is_none() {
            continue;
        }
        let c = vec2(
            (r.x as f32 + r.w as f32 / 2.0) * cw,
            (r.y as f32 + r.h as f32 / 2.0) * ch,
        );
        // A secret or portal room with a marker shows both icons side by side, smaller.
        let both = has_kind_icon && r.marker.is_some();
        let s = if both { side * 0.6 } else { side };
        let draw = |texture: &Texture2D, x: f32| {
            let params = DrawTextureParams {
                dest_size: Some(vec2(s, s)),
                ..Default::default()
            };
            draw_texture_ex(texture, x, c.y - s / 2.0, WHITE, params);
        };
        let mut x = if both { c.x - s } else { c.x - s / 2.0 };
        if let Some(texture) = icons.kind(r.kind, px) {
            draw(texture, x);
            x += s;
        }
        if let Some(marker) = r.marker {
            draw(icons.get(marker, px), x);
        }
    }
}

/// Export button, just left of the shortcuts button.
pub fn export_button_rect(reserved: f32) -> Rect {
    let help = help_button_rect(reserved);
    Rect::new(help.x - PAD - HELP_H, help.y, HELP_H, HELP_H)
}

/// Square button with a download arrow icon.
fn draw_export_button(reserved: f32) {
    let b = export_button_rect(reserved);
    let hover = b.contains(Vec2::from(mouse_position()));
    draw_rectangle(
        b.x,
        b.y,
        b.w,
        b.h,
        if hover { BUTTON_HOVER } else { BUTTON_BG },
    );
    let c = b.center();
    let (top, tip, half) = (c.y - 8.0, c.y + 4.0, 5.0);
    draw_line(c.x, top, c.x, tip, 2.0, WHITE);
    draw_line(c.x - half, tip - half, c.x, tip, 2.0, WHITE);
    draw_line(c.x + half, tip - half, c.x, tip, 2.0, WHITE);
    draw_line(c.x - 8.0, c.y + 8.0, c.x + 8.0, c.y + 8.0, 2.0, WHITE);
}

/// Biomes used in this map, one per row, in a column on the left, vertically centered
/// below the header. Drawn over the map on a translucent backdrop.
/// Returns the y of the bottom edge of the block.
fn draw_legend(world: &World, biomes: &[Biome]) -> f32 {
    let names_w = world
        .biomes
        .iter()
        .map(|&b| measure_text(&biomes[b].name, None, LEGEND_TEXT, 1.0).width)
        .fold(0.0, f32::max);
    let w = PAD + SWATCH + PAD + names_w + PAD;
    let h = PAD + world.biomes.len() as f32 * ROW_H + PAD;
    let (x, y) = (MARGIN, TOP + (screen_height() - TOP - h) / 2.0);
    draw_rectangle(x, y, w, h, Color::new(0.0, 0.0, 0.0, 0.6));

    for (i, &b) in world.biomes.iter().enumerate() {
        let biome = &biomes[b];
        let row_y = y + PAD + i as f32 * ROW_H;
        let swatch_y = row_y + (ROW_H - SWATCH) / 2.0;
        draw_rectangle(x + PAD, swatch_y, SWATCH, SWATCH, biome.fill);
        draw_rectangle_lines(x + PAD, swatch_y, SWATCH, SWATCH, 2.0, biome.border());
        let text_x = x + PAD + SWATCH + PAD;
        draw_text(
            &biome.name,
            text_x,
            row_y + ROW_H * 0.7,
            LEGEND_TEXT as f32,
            biome.border(),
        );
    }
    y + h
}

/// Room types, one per row with its icon and name, in a block starting at `y`
/// on the left, under the biome legend.
fn draw_room_legend(y: f32, icons: &mut Icons) {
    let names_w = LEGEND_ROOMS
        .iter()
        .map(|r| measure_text(r.0, None, LEGEND_TEXT, 1.0).width)
        .fold(0.0, f32::max);
    let w = PAD + ICON_BOX + PAD + names_w + PAD;
    let h = PAD + LEGEND_ROOMS.len() as f32 * ROW_H + PAD;
    let x = MARGIN;
    draw_rectangle(x, y, w, h, Color::new(0.0, 0.0, 0.0, 0.6));

    for (i, (name, fill, border, icon)) in LEGEND_ROOMS.iter().enumerate() {
        let row_y = y + PAD + i as f32 * ROW_H;
        let box_y = row_y + (ROW_H - ICON_BOX) / 2.0;
        draw_rectangle(x + PAD, box_y, ICON_BOX, ICON_BOX, *fill);
        draw_rectangle_lines(x + PAD, box_y, ICON_BOX, ICON_BOX, 2.0, *border);
        let texture = match icon {
            LegendIcon::Marker(m) => Some(icons.get(*m, ICON_PX)),
            LegendIcon::Kind(k) => icons.kind(*k, ICON_PX),
        };
        if let Some(texture) = texture {
            let side = ICON_PX as f32;
            let params = DrawTextureParams {
                dest_size: Some(vec2(side, side)),
                ..Default::default()
            };
            let offset = (ICON_BOX - side) / 2.0;
            draw_texture_ex(texture, x + PAD + offset, box_y + offset, WHITE, params);
        }
        let text_x = x + PAD + ICON_BOX + PAD;
        draw_text(
            name,
            text_x,
            row_y + ROW_H * 0.7,
            LEGEND_TEXT as f32,
            *border,
        );
    }
}

/// Shortcuts button with a "?" icon, and the popup listing the shortcuts when open.
fn draw_help(reserved: f32, open: bool) {
    let button = help_button_rect(reserved);
    let hover = button.contains(Vec2::from(mouse_position()));
    let bg = if open || hover {
        BUTTON_HOVER
    } else {
        BUTTON_BG
    };
    draw_rectangle(button.x, button.y, button.w, button.h, bg);

    let c = button.center();
    draw_circle_lines(c.x, c.y, ICON_R, 1.5, WHITE);
    let q = measure_text("?", None, HELP_TEXT, 1.0);
    draw_text(
        "?",
        c.x - q.width / 2.0,
        c.y + q.height / 2.0,
        HELP_TEXT as f32,
        WHITE,
    );

    if !open {
        return;
    }
    let popup = help_popup_rect(reserved);
    draw_rectangle(
        popup.x,
        popup.y,
        popup.w,
        popup.h,
        Color::new(0.0, 0.0, 0.0, 0.9),
    );
    draw_rectangle_lines(popup.x, popup.y, popup.w, popup.h, 1.0, CORRIDOR_BORDER);
    let action_x = popup.x + PAD + help_columns().0 + 2.0 * PAD;
    for (i, (key, action)) in SHORTCUTS.iter().enumerate() {
        let baseline = popup.y + PAD + i as f32 * ROW_H + ROW_H * 0.7;
        draw_text(
            key,
            popup.x + PAD,
            baseline,
            HELP_TEXT as f32,
            TRANSITION_DOOR,
        );
        draw_text(action, action_x, baseline, HELP_TEXT as f32, WHITE);
    }
}

/// Shows the room name and its size, in cells and in pixels, next to the cursor.
fn draw_tooltip(r: &Room, biomes: &[Biome], layout: &Layout) {
    let biome = &biomes[r.biome].name;
    let mut name = match r.kind {
        RoomKind::Normal => biome.clone(),
        RoomKind::Corridor => "Corridor".to_owned(),
        RoomKind::Secret => format!("{biome} (secret)"),
        RoomKind::Portal => format!("{biome} (portal)"),
    };
    match r.marker {
        Some(Marker::Boss) => name.push_str(" (boss)"),
        Some(Marker::Save) => name.push_str(" (save)"),
        None => {}
    }
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
    draw_rectangle(
        mx + 12.0,
        my + 8.0,
        dim.width + 12.0,
        24.0,
        Color::new(0.0, 0.0, 0.0, 0.8),
    );
    draw_text(&text, mx + 18.0, my + 25.0, 18.0, WHITE);
}
