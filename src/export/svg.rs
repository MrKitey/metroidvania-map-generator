//! Vector image of the map.

use super::{MapData, PX_PER_UNIT};
use crate::color::to_hex;
use crate::icons::IconId;
use crate::render::{
    door_color, door_rect, icon_side_px, line_thickness, room_border, room_fill, room_icons,
    room_rect,
};
use std::fmt::Write;

/// Vector version of the map, in world units, on a transparent background.
/// Mirrors `render::draw_map`: rooms, then doors, then icons.
pub fn render(map: &MapData) -> String {
    let (world, biomes, layout) = (map.world, map.biomes, map.layout);
    let (min, max) = map.area();
    let size = max - min;
    // Border and door thickness. A thick border is one image pixel thicker.
    let t = line_thickness(layout);
    let thick = t + 1.0 / PX_PER_UNIT;

    let mut svg = String::new();
    // Writing to a String cannot fail.
    let _ = writeln!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" \
         viewBox=\"{} {} {} {}\" width=\"{}\" height=\"{}\">",
        min.x,
        min.y,
        size.x,
        size.y,
        size.x * PX_PER_UNIT,
        size.y * PX_PER_UNIT,
    );

    // Each icon is defined once, then placed with <use>.
    svg.push_str("<defs>\n");
    for id in IconId::ALL {
        let tag = format!("<svg id=\"{}\"", id.name());
        svg.push_str(&id.svg().trim().replacen("<svg", &tag, 1));
        svg.push('\n');
    }
    svg.push_str("</defs>\n");

    for r in &world.rooms {
        let (border, is_thick) = room_border(r, biomes);
        let thickness = if is_thick { thick } else { t };
        // SVG strokes are centered on the edge, so the rectangle is inset by half of it
        // to keep the border inside the room, as in the app.
        let half = thickness / 2.0;
        let rect = room_rect(r, layout);
        let _ = writeln!(
            svg,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>",
            rect.x + half,
            rect.y + half,
            rect.w - thickness,
            rect.h - thickness,
            to_hex(room_fill(r, biomes)),
            to_hex(border),
            thickness,
        );
    }

    // Doors open a gap in the borders.
    for d in &world.doors {
        let rect = door_rect(d, layout);
        let _ = writeln!(
            svg,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            to_hex(door_color(world, biomes, d)),
        );
    }

    let side = icon_side_px(layout, PX_PER_UNIT) / PX_PER_UNIT;
    for r in &world.rooms {
        for (id, corner, s) in room_icons(r, layout, side) {
            let _ = writeln!(
                svg,
                "<use xlink:href=\"#{}\" x=\"{}\" y=\"{}\" width=\"{s}\" height=\"{s}\"/>",
                id.name(),
                corner.x,
                corner.y,
            );
        }
    }

    svg.push_str("</svg>\n");
    svg
}
