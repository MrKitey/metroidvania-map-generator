//! Skyline packer for the room sizes shown in the picker.

use super::theme::PAD;
use crate::layout::Layout;
use crate::world::{Choice, Pick};
use egui::{Rect, Vec2, pos2, vec2};

/// Height in points of one cell in the room picker. The width follows the cell aspect.
const PICKER_CELL: f32 = 20.0;
/// Tallest a room is drawn in the picker, in cells. The real size is kept.
const PICKER_MAX_H: u32 = 10;
const EPS: f32 = 0.01;

/// One step of the skyline: from `x` rightward, the next free top is `y`.
#[derive(Clone)]
struct Step {
    x: f32,
    y: f32,
}

/// Packs one section. Returns rects in list order, and the height they reach.
pub(super) fn pack_rooms(
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
                h.min(PICKER_MAX_H) as f32 * layout.ch * PICKER_CELL,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Layout;
    use crate::world::Pick;

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
    fn tall_rooms_are_drawn_ten_cells_high() {
        let layout = Layout::new((16, 9));
        let cell = layout.ch * PICKER_CELL;
        let (items, _) = pack_rooms(&[(4, 40), (2, 3)], Pick::Room, &layout, 1000.0);
        assert_eq!((items[0].0.w, items[0].0.h), (4, 40));
        assert!((items[0].1.height() - 10.0 * cell).abs() < EPS);
        assert!((items[1].1.height() - 3.0 * cell).abs() < EPS);
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
