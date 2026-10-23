//! Drawing of rooms, doors and icons, in world units converted to the map camera.

use super::theme::{door_color, room_border, room_fill};
use crate::biome::Biome;
use crate::icon_lib::{self, IconDef};
use crate::icons::{self, Icons};
use crate::layout::Layout;
use crate::settings;
use crate::view::View;
use crate::world::{Door, Room, World};
use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2, pos2, vec2};

/// Smallest icon, in points.
const MIN_ICON_PX: f32 = 12.0;
/// Size of two icons side by side, as a share of one.
const PAIR_SHARE: f32 = 0.6;

/// How the map is drawn.
pub struct MapView<'a> {
    pub view: &'a View,
    pub map: Rect,
    /// Points per world unit. Icons are rasterized at their drawn size, and a thick
    /// border is one pixel thicker than the others, at any zoom.
    pub scale: f32,
    /// World area on screen. Rooms and doors outside it are skipped. None draws all.
    pub visible: Option<Rect>,
    pub hovered: Option<usize>,
    /// Rooms kept highlighted after a selection rectangle.
    pub selected: &'a [usize],
}

/// Border and door thickness, in world units.
pub fn line_thickness(layout: &Layout) -> f32 {
    layout.cw.min(layout.ch) * 0.12
}

/// Room rectangle, in world units.
pub fn room_rect(r: &Room, layout: &Layout) -> Rect {
    Rect::from_min_size(
        pos2(r.x as f32 * layout.cw, r.y as f32 * layout.ch),
        vec2(r.w as f32 * layout.cw, r.h as f32 * layout.ch),
    )
}

/// Door rectangle, in world units. It is drawn over the room borders to open a gap.
pub fn door_rect(d: &Door, layout: &Layout) -> Rect {
    let Layout { cw, ch, .. } = *layout;
    let t = line_thickness(layout);
    let (x, y) = (d.x as f32 * cw, d.y as f32 * ch);
    if d.vertical {
        Rect::from_min_size(pos2(x - t, y + ch * 0.3), vec2(2.0 * t, ch * 0.4))
    } else {
        Rect::from_min_size(pos2(x + cw * 0.3, y - t), vec2(cw * 0.4, 2.0 * t))
    }
}

/// Side of an icon in points, at `scale` points per world unit.
/// `icon_size` is 1..=10; cell share is `icon_size / 10`.
pub fn icon_side_px(layout: &Layout, scale: f32, icon_size: u32) -> f32 {
    let share = settings::icon_share(icon_size);
    (layout.cw.min(layout.ch) * share * scale).max(MIN_ICON_PX)
}

/// Icons of a room, left to right, as (id, top-left corner, side) in world units.
/// A room with a kind icon and a marker shows both, side by side, smaller.
/// Each id uses its own size from `special`.
pub fn room_icons(
    r: &Room,
    layout: &Layout,
    special: &[IconDef],
    scale: f32,
) -> Vec<(String, Vec2, f32)> {
    let ids = icons::room_icon_ids(r);
    let both = ids.len() > 1;
    let sides: Vec<f32> = ids
        .iter()
        .map(|id| {
            let size = icon_lib::size_of(special, id);
            let side = icon_side_px(layout, scale, size) / scale;
            if both { side * PAIR_SHARE } else { side }
        })
        .collect();
    let total: f32 = sides.iter().sum();
    let c = vec2(
        (r.x as f32 + r.w as f32 / 2.0) * layout.cw,
        (r.y as f32 + r.h as f32 / 2.0) * layout.ch,
    );
    let mut x = c.x - total / 2.0;
    ids.into_iter()
        .zip(sides)
        .map(|(id, s)| {
            let corner = vec2(x, c.y - s / 2.0);
            x += s;
            (id, corner, s)
        })
        .collect()
}

/// Grid line color, transparent white.
fn grid_color(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgba_unmultiplied(255, 255, 255, 20)
    } else {
        Color32::from_rgba_unmultiplied(0, 0, 0, 25)
    }
}
/// Smallest cell on screen, in points. Below it the grid is too dense and is skipped.
const MIN_GRID_CELL_PX: f32 = 6.0;

fn to_screen(view: &View, map: Rect, world: Rect) -> Rect {
    Rect::from_min_max(
        view.world_to_screen(world.min.to_vec2(), map),
        view.world_to_screen(world.max.to_vec2(), map),
    )
}

/// Draws one line per cell edge over `area`, one pixel thick. Meant to be drawn
/// before the map so the rooms cover it.
pub fn draw_grid(
    painter: &Painter,
    layout: &Layout,
    view: &View,
    map: Rect,
    area: Rect,
    dark: bool,
) {
    if layout.cw.min(layout.ch) * view.scale < MIN_GRID_CELL_PX {
        return;
    }
    let stroke = Stroke::new(1.0, grid_color(dark));
    let first_x = (area.min.x / layout.cw).floor() as i32;
    let last_x = (area.max.x / layout.cw).ceil() as i32;
    for i in first_x..=last_x {
        let x = i as f32 * layout.cw;
        let a = view.world_to_screen(vec2(x, area.min.y), map);
        let b = view.world_to_screen(vec2(x, area.max.y), map);
        painter.line_segment([a, b], stroke);
    }
    let first_y = (area.min.y / layout.ch).floor() as i32;
    let last_y = (area.max.y / layout.ch).ceil() as i32;
    for j in first_y..=last_y {
        let y = j as f32 * layout.ch;
        let a = view.world_to_screen(vec2(area.min.x, y), map);
        let b = view.world_to_screen(vec2(area.max.x, y), map);
        painter.line_segment([a, b], stroke);
    }
}

/// Multiplies RGB alpha by `opacity` (0..=1).
fn with_opacity(c: Color32, opacity: f32) -> Color32 {
    let a = (c.a() as f32 * opacity).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

/// Draws rooms, doors and icons with a layer opacity multiplier.
pub fn draw_map_opacity(
    painter: &Painter,
    world: &World,
    biomes: &[Biome],
    special: &[IconDef],
    layout: &Layout,
    map_view: &MapView,
    icons: &mut Icons,
    opacity: f32,
) {
    let (view, map, scale) = (map_view.view, map_view.map, map_view.scale);
    let t = line_thickness(layout) * scale;
    let shown = |rect: Rect| map_view.visible.is_none_or(|v| v.intersects(rect));
    let uv = Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0));

    for (i, r) in world.rooms.iter().enumerate() {
        let world_rect = room_rect(r, layout);
        if !shown(world_rect) {
            continue;
        }
        let rect = to_screen(view, map, world_rect);
        let (border, thick) = room_border(r, biomes, special);
        let thickness = if thick { t + 1.0 } else { t };
        painter.rect_filled(
            rect,
            0.0,
            with_opacity(room_fill(r, biomes, special), opacity),
        );
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(thickness, with_opacity(border, opacity)),
            StrokeKind::Middle,
        );
        if map_view.selected.contains(&i) {
            painter.rect_filled(
                rect,
                0.0,
                with_opacity(Color32::from_rgba_unmultiplied(255, 255, 255, 70), opacity),
            );
        }
        if map_view.hovered == Some(i) {
            painter.rect_filled(
                rect,
                0.0,
                with_opacity(Color32::from_rgba_unmultiplied(255, 255, 255, 38), opacity),
            );
        }
    }

    // Doors are drawn over the borders to open a gap in them. A door inside a biome
    // takes the room color so it blends in, a door between biomes stands out.
    for d in &world.doors {
        let world_rect = door_rect(d, layout);
        if shown(world_rect) {
            painter.rect_filled(
                to_screen(view, map, world_rect),
                0.0,
                with_opacity(door_color(world, biomes, special, d), opacity),
            );
        }
    }

    // Icons are rasterized at their size in pixels, so they stay sharp at any zoom.
    let ppp = painter.ctx().pixels_per_point();
    let tint = with_opacity(Color32::WHITE, opacity);
    for r in &world.rooms {
        if !shown(room_rect(r, layout)) {
            continue;
        }
        for (id, corner, s) in room_icons(r, layout, special, scale) {
            let dest = Rect::from_min_size(view.world_to_screen(corner, map), vec2(s, s) * scale);
            let px = (s * scale * ppp).ceil() as u32;
            if let Some(tex) = icons.get(painter.ctx(), &id, px) {
                painter.image(tex.id(), dest, uv, tint);
            }
        }
    }
}

const LABEL_SIZE: f32 = 13.0;
const LABEL_PAD: f32 = 4.0;
/// Padding around layer group bounds, in cells.
const GROUP_PAD: f32 = 0.35;

/// Screen rect of the layer name label for each visible non-empty layer.
pub fn layer_label_rects(
    ctx: &egui::Context,
    layers: &crate::layer::LayerStack,
    layout: &Layout,
    view: &View,
    map: Rect,
) -> Vec<(usize, Rect)> {
    let mut out = Vec::new();
    for (i, layer) in layers.layers().iter().enumerate() {
        if !layer.visible {
            continue;
        }
        let Some(bounds) = layer.world.bounds_opt() else {
            continue;
        };
        let group = group_world_rect(bounds, layout);
        let screen = to_screen(view, map, group);
        let w = super::text_width(ctx, &layer.name, LABEL_SIZE) + LABEL_PAD * 2.0;
        let h = LABEL_SIZE + LABEL_PAD;
        let label =
            Rect::from_min_size(pos2(screen.min.x + 2.0, screen.min.y - h + 2.0), vec2(w, h));
        out.push((i, label));
    }
    out
}

fn group_world_rect(bounds: (i32, i32, i32, i32), layout: &Layout) -> Rect {
    let (x0, y0, x1, y1) = bounds;
    Rect::from_min_max(
        pos2(
            x0 as f32 * layout.cw - GROUP_PAD * layout.cw,
            y0 as f32 * layout.ch - GROUP_PAD * layout.ch,
        ),
        pos2(
            x1 as f32 * layout.cw + GROUP_PAD * layout.cw,
            y1 as f32 * layout.ch + GROUP_PAD * layout.ch,
        ),
    )
}

/// Draws the group frame and name label for one layer.
pub fn draw_layer_group(
    painter: &Painter,
    ctx: &egui::Context,
    layer: &crate::layer::Layer,
    active: bool,
    layout: &Layout,
    view: &View,
    map: Rect,
) {
    let Some(bounds) = layer.world.bounds_opt() else {
        return;
    };
    let group = group_world_rect(bounds, layout);
    let screen = to_screen(view, map, group);
    painter.rect_filled(screen, 0.0, layer.group_fill());
    let stroke_w = if active { 2.5 } else { 1.5 };
    painter.rect_stroke(
        screen,
        0.0,
        Stroke::new(stroke_w, layer.group_stroke()),
        StrokeKind::Outside,
    );
    let w = super::text_width(ctx, &layer.name, LABEL_SIZE) + LABEL_PAD * 2.0;
    let h = LABEL_SIZE + LABEL_PAD;
    let label = Rect::from_min_size(pos2(screen.min.x + 2.0, screen.min.y - h + 2.0), vec2(w, h));
    painter.rect_filled(label, 3.0, layer.group_stroke());
    painter.text(
        label.min + vec2(LABEL_PAD, LABEL_PAD * 0.5),
        Align2::LEFT_TOP,
        &layer.name,
        FontId::proportional(LABEL_SIZE),
        Color32::WHITE,
    );
}
