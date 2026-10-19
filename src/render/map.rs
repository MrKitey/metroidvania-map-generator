//! Drawing of rooms, doors and icons, in world units with the camera already set.

use super::theme::{door_color, room_border, room_fill};
use crate::biome::Biome;
use crate::icons::{IconId, Icons};
use crate::layout::Layout;
use crate::world::{Door, Room, World};
use macroquad::prelude::*;

/// Share of a cell taken by an icon.
const ICON_SHARE: f32 = 0.7;
/// Smallest icon, in pixels.
const MIN_ICON_PX: f32 = 12.0;
/// Size of two icons side by side, as a share of one.
const PAIR_SHARE: f32 = 0.6;

/// How the map is drawn.
pub struct MapView {
    /// Pixels per world unit. Icons are rasterized at their drawn size, and a thick
    /// border is one pixel thicker than the others, at any zoom.
    pub scale: f32,
    /// World area on screen. Rooms and doors outside it are skipped. None draws all.
    pub visible: Option<Rect>,
    pub hovered: Option<usize>,
}

/// Border and door thickness, in world units.
pub fn line_thickness(layout: &Layout) -> f32 {
    layout.cw.min(layout.ch) * 0.12
}

/// Room rectangle, in world units.
pub fn room_rect(r: &Room, layout: &Layout) -> Rect {
    Rect::new(
        r.x as f32 * layout.cw,
        r.y as f32 * layout.ch,
        r.w as f32 * layout.cw,
        r.h as f32 * layout.ch,
    )
}

/// Door rectangle, in world units. It is drawn over the room borders to open a gap.
pub fn door_rect(d: &Door, layout: &Layout) -> Rect {
    let Layout { cw, ch, .. } = *layout;
    let t = line_thickness(layout);
    let (x, y) = (d.x as f32 * cw, d.y as f32 * ch);
    if d.vertical {
        Rect::new(x - t, y + ch * 0.3, 2.0 * t, ch * 0.4)
    } else {
        Rect::new(x + cw * 0.3, y - t, cw * 0.4, 2.0 * t)
    }
}

/// Side of an icon in pixels, at `scale` pixels per world unit.
pub fn icon_side_px(layout: &Layout, scale: f32) -> f32 {
    (layout.cw.min(layout.ch) * ICON_SHARE * scale).max(MIN_ICON_PX)
}

/// Icons of a room, left to right, as (icon, top-left corner, side) in world units.
/// A room with a kind icon and a marker shows both, side by side, smaller.
/// `side` is the side of a single icon.
pub fn room_icons(r: &Room, layout: &Layout, side: f32) -> Vec<(IconId, Vec2, f32)> {
    let kind = IconId::of_kind(r.kind);
    let marker = r.marker.map(IconId::of_marker);
    let both = kind.is_some() && marker.is_some();
    let s = if both { side * PAIR_SHARE } else { side };
    let c = vec2(
        (r.x as f32 + r.w as f32 / 2.0) * layout.cw,
        (r.y as f32 + r.h as f32 / 2.0) * layout.ch,
    );
    let mut x = if both { c.x - s } else { c.x - s / 2.0 };
    kind.into_iter()
        .chain(marker)
        .map(|id| {
            let corner = vec2(x, c.y - s / 2.0);
            x += s;
            (id, corner, s)
        })
        .collect()
}

/// Draws rooms, doors and icons.
pub fn draw_map(
    world: &World,
    biomes: &[Biome],
    layout: &Layout,
    view: &MapView,
    icons: &mut Icons,
) {
    let scale = view.scale;
    let t = line_thickness(layout);
    let shown = |rect: &Rect| view.visible.is_none_or(|v| v.overlaps(rect));

    for (i, r) in world.rooms.iter().enumerate() {
        let rect = room_rect(r, layout);
        if !shown(&rect) {
            continue;
        }
        let (border, thick) = room_border(r, biomes);
        let thickness = if thick { t + 1.0 / scale } else { t };
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, room_fill(r, biomes));
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, thickness, border);
        if view.hovered == Some(i) {
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                Color::new(1.0, 1.0, 1.0, 0.15),
            );
        }
    }

    // Doors are drawn over the borders to open a gap in them. A door inside a biome
    // takes the room color so it blends in, a door between biomes stands out.
    for d in &world.doors {
        let rect = door_rect(d, layout);
        if shown(&rect) {
            let color = door_color(world, biomes, d);
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, color);
        }
    }

    // Icons are rasterized at their size in pixels, so they stay sharp at any zoom.
    // One raster size for all icons, otherwise the cache would rasterize every frame.
    let side_px = icon_side_px(layout, scale);
    let px = side_px.ceil() as u32;
    let side = side_px / scale;
    for r in &world.rooms {
        if !shown(&room_rect(r, layout)) {
            continue;
        }
        for (id, corner, s) in room_icons(r, layout, side) {
            let params = DrawTextureParams {
                dest_size: Some(vec2(s, s)),
                ..Default::default()
            };
            draw_texture_ex(icons.get(id, px), corner.x, corner.y, WHITE, params);
        }
    }
}
