//! Room picker popup and the preview of the room being placed.

use super::map::line_thickness;
use super::theme::{LEGEND_TEXT, PAD, ROW_H};
use crate::layout::Layout;
use crate::world::{Choice, Pick, Sizes};
use macroquad::prelude::*;

/// Height in pixels of one cell in the room picker. The width follows the cell aspect.
const PICKER_CELL: f32 = 20.0;
const PICKER_TEXT: u16 = 16;
/// Widest the picker can grow, as a share of the screen width.
const PICKER_MAX_W: f32 = 0.6;

/// Room picker popup, with screen positions already computed.
pub struct Picker {
    pub rect: Rect,
    titles: Vec<(&'static str, Vec2)>,
    pub items: Vec<(Choice, Rect)>,
}

/// Room being placed, snapped to the cell grid. `free` is false if it overlaps a room.
pub struct Ghost {
    pub x: i32,
    pub y: i32,
    pub choice: Choice,
    pub free: bool,
}

/// Lays out the picker with its top-left corner at `anchor`, moved to stay on screen.
/// Rooms are drawn at the same aspect as on the map, and wrap to the next row when the
/// popup is wide enough.
pub fn picker_layout(sizes: &Sizes, layout: &Layout, anchor: Vec2) -> Picker {
    let sections = [
        ("Rooms", Pick::Room, &sizes.rooms),
        ("Boss rooms", Pick::Boss, &sizes.boss),
        ("Corridors", Pick::Corridor, &sizes.corridor),
    ];
    let max_w = screen_width() * PICKER_MAX_W;
    let mut titles = Vec::new();
    let mut items = Vec::new();
    let (mut y, mut width) = (PAD, 0.0f32);
    for (title, pick, list) in sections {
        titles.push((title, vec2(PAD, y)));
        width = width.max(PAD + measure_text(title, None, LEGEND_TEXT, 1.0).width + PAD);
        y += ROW_H;
        let (mut x, mut row_h) = (PAD, 0.0f32);
        for &(w, h) in list {
            let size = vec2(
                w as f32 * layout.cw * PICKER_CELL,
                h as f32 * layout.ch * PICKER_CELL,
            );
            if x > PAD && x + size.x > max_w {
                x = PAD;
                y += row_h + PAD;
                row_h = 0.0;
            }
            items.push((Choice { pick, w, h }, Rect::new(x, y, size.x, size.y)));
            x += size.x + PAD;
            row_h = row_h.max(size.y);
            width = width.max(x);
        }
        y += row_h + PAD;
    }

    let origin = vec2(
        anchor.x.min(screen_width() - width).max(0.0),
        anchor.y.min(screen_height() - y).max(0.0),
    );
    for (_, p) in &mut titles {
        *p += origin;
    }
    for (_, r) in &mut items {
        r.move_to(r.point() + origin);
    }
    Picker {
        rect: Rect::new(origin.x, origin.y, width, y),
        titles,
        items,
    }
}

/// Black translucent popup listing every size, drawn as a white outline with its size in
/// the middle.
pub fn draw_picker(p: &Picker) {
    let mouse = Vec2::from(mouse_position());
    draw_rectangle(
        p.rect.x,
        p.rect.y,
        p.rect.w,
        p.rect.h,
        Color::new(0.0, 0.0, 0.0, 0.8),
    );
    for (title, pos) in &p.titles {
        draw_text(
            title,
            pos.x,
            pos.y + ROW_H * 0.7,
            LEGEND_TEXT as f32,
            LIGHTGRAY,
        );
    }
    for (choice, r) in &p.items {
        if r.contains(mouse) {
            draw_rectangle(r.x, r.y, r.w, r.h, Color::new(1.0, 1.0, 1.0, 0.15));
        }
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, WHITE);
        let text = format!("{}x{}", choice.w, choice.h);
        let dim = measure_text(&text, None, PICKER_TEXT, 1.0);
        let c = r.center();
        draw_text(
            &text,
            c.x - dim.width / 2.0,
            c.y + dim.height / 2.0,
            PICKER_TEXT as f32,
            WHITE,
        );
    }
}

/// Room being placed, in world units with the map camera set. Green if free, red if not.
pub fn draw_ghost(g: &Ghost, layout: &Layout) {
    let (x, y) = (g.x as f32 * layout.cw, g.y as f32 * layout.ch);
    let (w, h) = (g.choice.w as f32 * layout.cw, g.choice.h as f32 * layout.ch);
    let fill = if g.free {
        Color::new(0.0, 1.0, 0.0, 0.35)
    } else {
        Color::new(1.0, 0.0, 0.0, 0.35)
    };
    draw_rectangle(x, y, w, h, fill);
    draw_rectangle_lines(x, y, w, h, line_thickness(layout), WHITE);
}
