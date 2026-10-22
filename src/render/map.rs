//! Drawing of rooms, doors and icons, in world units converted to the map camera.

use super::theme::{door_color, room_border, room_fill};
use crate::biome::Biome;
use crate::icon_lib::IconDef;
use crate::icons::{self, Icons};
use crate::layout::Layout;
use crate::view::View;
use crate::world::{Door, Room, World};
use egui::{Color32, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2, pos2, vec2};

/// Share of a cell taken by an icon.
const ICON_SHARE: f32 = 0.7;
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
pub fn icon_side_px(layout: &Layout, scale: f32) -> f32 {
    (layout.cw.min(layout.ch) * ICON_SHARE * scale).max(MIN_ICON_PX)
}

/// Icons of a room, left to right, as (id, top-left corner, side) in world units.
/// A room with a kind icon and a marker shows both, side by side, smaller.
/// `side` is the side of a single icon.
pub fn room_icons(r: &Room, layout: &Layout, side: f32) -> Vec<(String, Vec2, f32)> {
    let ids = icons::room_icon_ids(r);
    let both = ids.len() > 1;
    let s = if both { side * PAIR_SHARE } else { side };
    let c = vec2(
        (r.x as f32 + r.w as f32 / 2.0) * layout.cw,
        (r.y as f32 + r.h as f32 / 2.0) * layout.ch,
    );
    let mut x = if both { c.x - s } else { c.x - s / 2.0 };
    ids.into_iter()
        .map(|id| {
            let corner = vec2(x, c.y - s / 2.0);
            x += s;
            (id, corner, s)
        })
        .collect()
}

/// Grid line color, transparent white.
const GRID_COLOR: Color32 = Color32::from_rgba_unmultiplied_const(255, 255, 255, 20);
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
pub fn draw_grid(painter: &Painter, layout: &Layout, view: &View, map: Rect, area: Rect) {
    if layout.cw.min(layout.ch) * view.scale < MIN_GRID_CELL_PX {
        return;
    }
    let stroke = Stroke::new(1.0, GRID_COLOR);
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

/// Draws rooms, doors and icons.
pub fn draw_map(
    painter: &Painter,
    world: &World,
    biomes: &[Biome],
    special: &[IconDef],
    layout: &Layout,
    map_view: &MapView,
    icons: &mut Icons,
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
        painter.rect_filled(rect, 0.0, room_fill(r, biomes, special));
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(thickness, border),
            StrokeKind::Middle,
        );
        if map_view.selected.contains(&i) {
            painter.rect_filled(
                rect,
                0.0,
                Color32::from_rgba_unmultiplied(255, 255, 255, 70),
            );
        }
        if map_view.hovered == Some(i) {
            painter.rect_filled(
                rect,
                0.0,
                Color32::from_rgba_unmultiplied(255, 255, 255, 38),
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
                door_color(world, biomes, special, d),
            );
        }
    }

    // Icons are rasterized at their size in pixels, so they stay sharp at any zoom.
    let side_px = icon_side_px(layout, scale);
    let px = (side_px * painter.ctx().pixels_per_point()).ceil() as u32;
    let side = side_px / scale;
    for r in &world.rooms {
        if !shown(room_rect(r, layout)) {
            continue;
        }
        for (id, corner, s) in room_icons(r, layout, side) {
            let dest = Rect::from_min_size(view.world_to_screen(corner, map), vec2(s, s) * scale);
            if let Some(tex) = icons.get(painter.ctx(), &id, px) {
                painter.image(tex.id(), dest, uv, Color32::WHITE);
            }
        }
    }
}
