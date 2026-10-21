//! Room picker popup and the preview of the room being placed.

use super::map::line_thickness;
use super::pack::pack_rooms;
use super::text_width;
use super::theme::{LEGEND_TEXT, PAD, ROW_H};
use crate::layout::Layout;
use crate::text::rooms;
use crate::view::View;
use crate::world::{Choice, Pick, Sizes};
use egui::{
    Align2, Color32, Context, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, Ui, pos2, vec2,
};

const PICKER_TEXT: f32 = 14.0;
/// Widest the picker can grow, as a share of the map width.
const PICKER_MAX_W: f32 = 0.6;

/// Room picker popup, with screen positions already computed.
pub struct Picker {
    pub rect: Rect,
    titles: Vec<(&'static str, Pos2)>,
    pub items: Vec<(Choice, Rect)>,
}

/// Room being placed, snapped to the cell grid. `free` is false if it overlaps a room.
pub struct Ghost {
    pub x: i32,
    pub y: i32,
    pub choice: Choice,
    pub free: bool,
}

/// Lays out the picker with its top-left corner at `anchor`, moved to stay in `map`.
/// Rooms keep the map cell aspect. The tallest go down first, and each later room
/// fills the lowest free gap that can hold it.
pub fn picker_layout(
    ctx: &Context,
    sizes: &Sizes,
    layout: &Layout,
    anchor: Pos2,
    map: Rect,
) -> Picker {
    let sections = [
        (rooms::ROOMS, Pick::Room, &sizes.rooms),
        (rooms::BOSS_ROOMS, Pick::Boss, &sizes.boss),
        (rooms::CORRIDORS, Pick::Corridor, &sizes.corridor),
    ];
    let max_w = map.width() * PICKER_MAX_W;
    let mut titles = Vec::new();
    let mut items = Vec::new();
    let (mut y, mut width) = (PAD, 0.0f32);
    for (title, pick, list) in sections {
        titles.push((title, pos2(PAD, y)));
        width = width.max(PAD + text_width(ctx, title, LEGEND_TEXT) + PAD);
        y += ROW_H;
        let (placed, bottom) = pack_rooms(list, pick, layout, max_w);
        for (choice, r) in placed {
            let r = r.translate(vec2(PAD, y));
            width = width.max(r.max.x + PAD);
            items.push((choice, r));
        }
        y += bottom + PAD;
    }

    let origin = pos2(
        anchor.x.min(map.max.x - width).max(map.min.x),
        anchor.y.min(map.max.y - y).max(map.min.y),
    );
    for (_, p) in &mut titles {
        *p += origin.to_vec2();
    }
    for (_, r) in &mut items {
        *r = r.translate(origin.to_vec2());
    }
    Picker {
        rect: Rect::from_min_size(origin, vec2(width, y)),
        titles,
        items,
    }
}

/// Black translucent popup listing every size, drawn as a white outline with its size in
/// the middle.
pub fn draw_picker(ui: &Ui, p: &Picker) {
    let painter = ui.painter();
    let mouse = ui.input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::ZERO);
    painter.rect_filled(p.rect, 4.0, Color32::from_rgba_unmultiplied(0, 0, 0, 204));
    for (title, pos) in &p.titles {
        painter.text(
            *pos + vec2(0.0, ROW_H * 0.7),
            Align2::LEFT_BOTTOM,
            *title,
            FontId::proportional(LEGEND_TEXT),
            Color32::LIGHT_GRAY,
        );
    }
    for (choice, r) in &p.items {
        if r.contains(mouse) {
            painter.rect_filled(*r, 0.0, Color32::from_rgba_unmultiplied(255, 255, 255, 38));
        }
        painter.rect_stroke(
            *r,
            0.0,
            Stroke::new(2.0, Color32::WHITE),
            StrokeKind::Inside,
        );
        let text = format!("{}x{}", choice.w, choice.h);
        painter.text(
            r.center(),
            Align2::CENTER_CENTER,
            text,
            FontId::proportional(PICKER_TEXT),
            Color32::WHITE,
        );
    }
}

/// Room being placed, in the map camera. Green if free, red if not.
pub fn draw_ghost(painter: &Painter, g: &Ghost, layout: &Layout, view: &View, map: Rect) {
    let world = Rect::from_min_size(
        pos2(g.x as f32 * layout.cw, g.y as f32 * layout.ch),
        vec2(g.choice.w as f32 * layout.cw, g.choice.h as f32 * layout.ch),
    );
    let rect = Rect::from_min_max(
        view.world_to_screen(world.min.to_vec2(), map),
        view.world_to_screen(world.max.to_vec2(), map),
    );
    let fill = if g.free {
        Color32::from_rgba_unmultiplied(0, 255, 0, 89)
    } else {
        Color32::from_rgba_unmultiplied(255, 0, 0, 89)
    };
    painter.rect_filled(rect, 0.0, fill);
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(line_thickness(layout) * view.scale, Color32::WHITE),
        StrokeKind::Middle,
    );
}
