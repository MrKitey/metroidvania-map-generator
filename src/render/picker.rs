//! Room picker popup and the preview of the room being placed.

use super::map::line_thickness;
use super::theme::{LEGEND_TEXT, PAD, ROW_H};
use crate::layout::Layout;
use crate::text::rooms;
use crate::view::View;
use crate::world::{Choice, Pick, Sizes};
use egui::{
    Align2, Color32, Context, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};

/// Height in points of one cell in the room picker. The width follows the cell aspect.
const PICKER_CELL: f32 = 20.0;
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

fn text_width(ctx: &Context, text: &str, size: f32) -> f32 {
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(text.to_owned(), FontId::proportional(size), Color32::WHITE)
            .size()
            .x
    })
}

/// One step of the skyline: from `x` rightward, the next free top is `y`.
#[derive(Clone)]
struct Step {
    x: f32,
    y: f32,
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

/// Packs one section. Returns rects in list order, and the height they reach.
fn pack_rooms(
    list: &[(u32, u32)],
    pick: Pick,
    layout: &Layout,
    max_w: f32,
) -> (Vec<(Choice, Rect)>, f32) {
    let sizes: Vec<Vec2> = list
        .iter()
        .map(|&(w, h)| {
            vec2(
                w as f32 * layout.cw * PICKER_CELL,
                h as f32 * layout.ch * PICKER_CELL,
            )
        })
        .collect();
    let limit = sizes.iter().fold((max_w - PAD).max(0.0), |m, s| m.max(s.x));
    let rects = pack_sizes(&sizes, limit);
    let bottom = rects.iter().fold(0.0f32, |m, r| m.max(r.max.y));
    let items = list
        .iter()
        .zip(rects)
        .map(|(&(w, h), r)| (Choice { pick, w, h }, r))
        .collect();
    (items, bottom)
}

/// Lowest free gap, then the leftmost one. `limit` is the right edge a room may touch.
fn pack_sizes(sizes: &[Vec2], limit: f32) -> Vec<Rect> {
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by(|&a, &b| {
        let (sa, sb) = (sizes[a], sizes[b]);
        sb.y.total_cmp(&sa.y).then(sb.x.total_cmp(&sa.x))
    });
    let mut steps = vec![Step { x: 0.0, y: 0.0 }];
    let mut rects = vec![Rect::ZERO; sizes.len()];
    for i in order {
        let size = sizes[i];
        let (x, y) = lowest_gap(&steps, size.x, limit);
        rects[i] = Rect::from_min_size(pos2(x, y), size);
        raise(&mut steps, x, size.x + PAD, y + size.y + PAD);
    }
    rects
}

const EPS: f32 = 0.01;

fn lowest_gap(steps: &[Step], w: f32, limit: f32) -> (f32, f32) {
    let mut best: Option<(f32, f32)> = None;
    for step in steps {
        if step.x + w > limit + EPS {
            continue;
        }
        let y = sky_y(steps, step.x, w + PAD);
        let better = best
            .is_none_or(|(bx, by)| y < by - EPS || ((y - by).abs() <= EPS && step.x < bx - EPS));
        if better {
            best = Some((step.x, y));
        }
    }
    best.unwrap_or((0.0, 0.0))
}

/// Highest skyline on `[x, x + w)`.
fn sky_y(steps: &[Step], x: f32, w: f32) -> f32 {
    let right = x + w;
    let mut y = 0.0f32;
    for i in 0..steps.len() {
        let next = steps.get(i + 1).map(|s| s.x).unwrap_or(f32::MAX);
        if next <= x + EPS || steps[i].x >= right - EPS {
            continue;
        }
        y = y.max(steps[i].y);
    }
    y
}

fn y_at(steps: &[Step], x: f32) -> f32 {
    let mut y = 0.0;
    for step in steps {
        if step.x > x + EPS {
            break;
        }
        y = step.y;
    }
    y
}

/// Lifts `[x, x + span)` up to `top`. A taller step already there stays.
fn raise(steps: &mut Vec<Step>, x: f32, span: f32, top: f32) {
    let right = x + span;
    let mut xs = vec![0.0, x, right];
    xs.extend(steps.iter().map(|s| s.x));
    xs.sort_by(|a, b| a.total_cmp(b));
    xs.dedup_by(|a, b| (*a - *b).abs() <= EPS);

    let old = steps.clone();
    steps.clear();
    let mut prev = None;
    for px in xs {
        let y = if px >= x - EPS && px < right - EPS {
            top.max(y_at(&old, px))
        } else {
            y_at(&old, px)
        };
        if prev.is_none_or(|p: f32| (y - p).abs() > EPS) {
            steps.push(Step { x: px, y });
            prev = Some(y);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn apart(rects: &[Rect]) -> bool {
        rects.iter().enumerate().all(|(i, a)| {
            rects[i + 1..].iter().all(|b| {
                a.max.x + PAD <= b.min.x + EPS
                    || b.max.x + PAD <= a.min.x + EPS
                    || a.max.y + PAD <= b.min.y + EPS
                    || b.max.y + PAD <= a.min.y + EPS
            })
        })
    }

    #[test]
    fn short_rooms_sit_beside_a_tall_one() {
        let rects = pack_sizes(
            &[vec2(30.0, 90.0), vec2(30.0, 20.0), vec2(30.0, 20.0)],
            70.0,
        );
        assert!(apart(&rects));
        assert!(rects.iter().all(|r| r.max.x <= 70.0 + EPS));
        let bottom = rects.iter().fold(0.0f32, |m, r| m.max(r.max.y));
        assert!(bottom <= 90.0 + EPS);
    }

    #[test]
    fn wrapped_rooms_keep_a_gap() {
        let rects = pack_sizes(
            &[
                vec2(40.0, 50.0),
                vec2(40.0, 10.0),
                vec2(40.0, 10.0),
                vec2(30.0, 30.0),
                vec2(20.0, 40.0),
            ],
            100.0,
        );
        assert!(apart(&rects));
        assert!(
            rects
                .iter()
                .all(|r| r.min.x >= -EPS && r.max.x <= 100.0 + EPS)
        );
    }
}
