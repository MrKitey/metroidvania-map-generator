//! Screen overlays: header, legends, seed label, buttons, shortcuts popup and tooltip.
//! Everything here is in screen pixels.

use super::picker::Ghost;
use super::theme::*;
use crate::biome::Biome;
use crate::icons::{IconId, Icons};
use crate::layout::{Layout, MARGIN, TOP};
use crate::world::{Marker, Room, RoomKind, World};
use macroquad::prelude::*;
use std::sync::OnceLock;

const SEED_SIZE: u16 = 20;
/// Seconds the "copied" feedback stays on the seed label.
pub const COPIED_TIME: f64 = 1.5;

/// Listed in the shortcuts popup, as (key, action).
const SHORTCUTS: &[(&str, &str)] = &[
    ("R", "New map"),
    ("Arrows", "Map size"),
    ("Wheel", "Zoom"),
    ("Middle drag", "Pan"),
    ("Right click", "Place a room"),
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
/// Side of the biome color square in the legend.
const SWATCH: f32 = 14.0;
/// Side of the room type icon box in the legend, and of the icon inside it.
const ICON_BOX: f32 = 20.0;
const ICON_PX: u32 = 16;
/// Space between the biome legend and the room type legend.
const LEGEND_GAP: f32 = 8.0;
const TOOLTIP_TEXT: u16 = 18;

/// Room types listed in the legend, as (name, fill, border, icon).
const LEGEND_ROOMS: &[(&str, Color, Color, IconId)] = &[
    ("Boss", BOSS_FILL, BOSS_BORDER, IconId::Boss),
    ("Save", SAVE_FILL, SAVE_BORDER, IconId::Save),
    ("Secret", SECRET_FILL, SECRET_BORDER, IconId::Secret),
    ("Portal", PORTAL_FILL, PORTAL_BORDER, IconId::Portal),
];

/// Widths of the key column and the action column of the shortcuts popup.
/// The texts never change, so they are measured once.
fn help_columns() -> (f32, f32) {
    static COLUMNS: OnceLock<(f32, f32)> = OnceLock::new();
    *COLUMNS.get_or_init(|| {
        let width = |text: &str| measure_text(text, None, HELP_TEXT, 1.0).width;
        let key_w = SHORTCUTS.iter().map(|s| width(s.0)).fold(0.0, f32::max);
        let action_w = SHORTCUTS.iter().map(|s| width(s.1)).fold(0.0, f32::max);
        (key_w, action_w)
    })
}

/// Positions of the clickable overlays for one frame. They sit left of `reserved`,
/// the screen width taken by the editor panel. Built once, so hit tests and drawing agree.
pub struct Hud {
    seed_text: String,
    /// Clickable area of the seed label, in the bottom right corner of the map area.
    pub seed: Rect,
    seed_baseline: f32,
    /// Square shortcuts button, in the top right corner of the header.
    pub help: Rect,
    /// Shortcuts popup, just below the button and right aligned with it.
    pub popup: Rect,
    /// Export button, just left of the shortcuts button.
    pub export: Rect,
}

impl Hud {
    pub fn new(seed: u64, copied: bool, reserved: f32) -> Hud {
        let seed_text = if copied {
            format!("Seed {seed} (copied)")
        } else {
            format!("Seed {seed}")
        };
        let dim = measure_text(&seed_text, None, SEED_SIZE, 1.0);
        let x = screen_width() - reserved - MARGIN - dim.width;
        let y = screen_height() - MARGIN - dim.height;

        let help = Rect::new(
            screen_width() - reserved - MARGIN - HELP_H,
            (TOP - HELP_H) / 2.0,
            HELP_H,
            HELP_H,
        );
        let (key_w, action_w) = help_columns();
        let w = PAD + key_w + 2.0 * PAD + action_w + PAD;
        let h = PAD + SHORTCUTS.len() as f32 * ROW_H + PAD;
        Hud {
            seed_text,
            seed: Rect::new(x, y, dim.width, dim.height),
            seed_baseline: y + dim.offset_y,
            help,
            popup: Rect::new(help.right() - w, help.bottom() + 6.0, w, h),
            export: Rect::new(help.x - PAD - HELP_H, help.y, HELP_H, HELP_H),
        }
    }
}

/// Opaque header with the room count and the notice, then the biome legend and the
/// room type legend on the left. `legend_icons` is a second icon set used at a fixed
/// size, so the map zoom does not make the legend icons rasterize again every frame.
pub fn draw_header(
    world: &World,
    biomes: &[Biome],
    notice: Option<&str>,
    legend_icons: &mut Icons,
) {
    // Opaque so the map does not show through the text when panned under it.
    draw_rectangle(0.0, 0.0, screen_width(), TOP, BACKGROUND);
    let mut info = format!("Rooms {}", world.rooms.len());
    if let Some(notice) = notice {
        info = format!("{info}    {notice}");
    }
    draw_text(info, MARGIN, TOP / 2.0 + 7.0, 20.0, WHITE);
    let biomes_bottom = draw_legend(world, biomes);
    draw_room_legend(biomes_bottom + LEGEND_GAP, legend_icons);
}

/// Seed label over a backdrop, so it stays readable over the map.
/// Brighter on hover to show it is clickable.
pub fn draw_seed(hud: &Hud) {
    let rect = hud.seed;
    let hover = rect.contains(Vec2::from(mouse_position()));
    draw_rectangle(
        rect.x - 6.0,
        rect.y - 6.0,
        rect.w + 12.0,
        rect.h + 12.0,
        BACKGROUND,
    );
    let color = if hover { WHITE } else { LIGHTGRAY };
    draw_text(
        &hud.seed_text,
        rect.x,
        hud.seed_baseline,
        SEED_SIZE as f32,
        color,
    );
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
        let side = ICON_PX as f32;
        let params = DrawTextureParams {
            dest_size: Some(vec2(side, side)),
            ..Default::default()
        };
        let offset = (ICON_BOX - side) / 2.0;
        draw_texture_ex(
            icons.get(*icon, ICON_PX),
            x + PAD + offset,
            box_y + offset,
            WHITE,
            params,
        );
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

/// Square button with a download arrow icon.
pub fn draw_export_button(hud: &Hud) {
    let b = hud.export;
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

/// Shortcuts button with a "?" icon, and the popup listing the shortcuts when open.
pub fn draw_help(hud: &Hud, open: bool) {
    let button = hud.help;
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
    let popup = hud.popup;
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
pub fn draw_tooltip(r: &Room, biomes: &[Biome], layout: &Layout) {
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
    draw_cursor_label(&text);
}

/// Size and position in pixels of the room being placed, next to the cursor.
pub fn draw_ghost_label(g: &Ghost, layout: &Layout) {
    let (vw, vh) = layout.viewport;
    draw_cursor_label(&format!(
        "{}x{} at {}, {} px",
        g.choice.w,
        g.choice.h,
        g.x * vw as i32,
        g.y * vh as i32
    ));
}

/// Text on a dark backdrop, next to the cursor.
fn draw_cursor_label(text: &str) {
    let (mx, my) = mouse_position();
    let dim = measure_text(text, None, TOOLTIP_TEXT, 1.0);
    draw_rectangle(
        mx + 12.0,
        my + 8.0,
        dim.width + 12.0,
        24.0,
        Color::new(0.0, 0.0, 0.0, 0.8),
    );
    draw_text(text, mx + 18.0, my + 25.0, TOOLTIP_TEXT as f32, WHITE);
}
