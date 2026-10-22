//! Seed label, shortcuts popup and cursor labels. Everything here is in screen points.

use super::picker::Ghost;
use super::text_width;
use super::theme::{BACKGROUND, TRANSITION_DOOR};
use crate::biome::Biome;
use crate::icon_lib::{self, IconDef};
use crate::layout::{Layout, MARGIN};
use crate::text::hud::{self as t, SHORTCUTS};
use crate::world::{Marker, Room, RoomKind};
use egui::{Align2, Color32, Context, FontId, Rect, Ui, pos2, vec2};

/// Seconds the "copied" feedback stays on the seed label.
pub const COPIED_TIME: f64 = 1.5;

const SEED_SIZE: f32 = 16.0;
const TOOLTIP_TEXT: f32 = 14.0;

/// Positions of the clickable overlays for one frame.
pub struct Hud {
    seed_text: String,
    /// Clickable area of the seed label, in the bottom right corner of the map area.
    pub seed: Rect,
}

impl Hud {
    pub fn new(ctx: &Context, seed: u64, copied: bool, map: Rect) -> Hud {
        let seed_text = t::seed(seed, copied);
        let w = text_width(ctx, &seed_text, SEED_SIZE);
        let h = SEED_SIZE + 4.0;
        let x = map.max.x - MARGIN - w;
        let y = map.max.y - MARGIN - h;
        Hud {
            seed_text,
            seed: Rect::from_min_size(pos2(x, y), vec2(w, h)),
        }
    }
}

/// Seed label over a backdrop, so it stays readable over the map.
pub fn draw_seed(ui: &Ui, hud: &Hud) {
    let painter = ui.painter();
    let hover = ui
        .input(|i| i.pointer.hover_pos())
        .is_some_and(|p| hud.seed.contains(p));
    painter.rect_filled(hud.seed.expand(6.0), 4.0, BACKGROUND);
    painter.text(
        hud.seed.min,
        Align2::LEFT_TOP,
        &hud.seed_text,
        FontId::proportional(SEED_SIZE),
        if hover {
            Color32::WHITE
        } else {
            Color32::LIGHT_GRAY
        },
    );
}

/// Shortcuts listing, as a floating window.
pub fn draw_help(ctx: &Context, open: &mut bool) {
    egui::Window::new("Shortcuts")
        .open(open)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::RIGHT_TOP, vec2(-12.0, 48.0))
        .show(ctx, |ui| {
            for (key, action) in SHORTCUTS {
                ui.horizontal(|ui| {
                    ui.colored_label(TRANSITION_DOOR, *key);
                    ui.label(*action);
                });
            }
        });
}

/// Shows the room name and its size, in cells and in pixels, next to the cursor.
pub fn draw_tooltip(
    ui: &Ui,
    r: &Room,
    biomes: &[Biome],
    special: &[IconDef],
    layout: &Layout,
) {
    let biome = &biomes[r.biome].name;
    let mut name = match r.kind {
        RoomKind::Normal => biome.clone(),
        RoomKind::Corridor => t::CORRIDOR.to_owned(),
        RoomKind::Secret => t::secret_room(biome),
        RoomKind::Portal => t::portal_room(biome),
    };
    match r.marker {
        Some(Marker::Boss) => name.push_str(t::BOSS_SUFFIX),
        Some(Marker::Save) => name.push_str(t::SAVE_SUFFIX),
        None => {}
    }
    if let Some(id) = &r.icon {
        let label = icon_lib::find(special, id)
            .map(|d| d.label.as_str())
            .unwrap_or(id);
        name.push_str(&format!(" ({label})"));
    }
    let px = (r.w * layout.viewport.0, r.h * layout.viewport.1);
    draw_cursor_label(ui, &t::tooltip(&name, (r.w, r.h), px));
}

/// Size and position in pixels of the room being placed, next to the cursor.
pub fn draw_ghost_label(ui: &Ui, g: &Ghost, layout: &Layout) {
    let (vw, vh) = layout.viewport;
    draw_cursor_label(
        ui,
        &t::ghost_label(g.choice.w, g.choice.h, g.x * vw as i32, g.y * vh as i32),
    );
}

fn draw_cursor_label(ui: &Ui, text: &str) {
    let Some(pos) = ui.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let painter = ui.painter();
    let w = text_width(ui.ctx(), text, TOOLTIP_TEXT);
    let rect = Rect::from_min_size(pos + vec2(12.0, 8.0), vec2(w + 12.0, 24.0));
    painter.rect_filled(rect, 4.0, Color32::from_rgba_unmultiplied(0, 0, 0, 204));
    painter.text(
        rect.min + vec2(6.0, 12.0),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(TOOLTIP_TEXT),
        Color32::WHITE,
    );
}
