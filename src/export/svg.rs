//! Vector image of the map.

use super::{MapData, PX_PER_UNIT};
use crate::color::to_hex;
use crate::icons::{self, room_icon_ids};
use crate::render::{
    door_color, door_rect, line_thickness, room_border, room_fill, room_icons, room_rect,
};
use crate::world::World;
use std::collections::HashSet;
use std::fmt::Write;

/// Vector version of the map, in world units, on a transparent background.
/// Visible layers are stacked in list order.
pub fn render(map: &MapData) -> String {
    let (biomes, special, layout) = (map.biomes, map.special, map.layout);
    let worlds = map.worlds();
    let (min, max) = map.area();
    let size = max - min;
    let t = line_thickness(layout);
    let thick = t + 1.0 / PX_PER_UNIT;

    let mut svg = String::new();
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

    let mut used = HashSet::new();
    for world in &worlds {
        for r in &world.rooms {
            for id in room_icon_ids(r) {
                used.insert(id);
            }
        }
    }

    svg.push_str("<defs>\n");
    for def in special {
        if !used.contains(&def.id) {
            continue;
        }
        if let Some(def_svg) = icons::export_svg_def(def) {
            svg.push_str(&def_svg);
            svg.push('\n');
        }
    }
    svg.push_str("</defs>\n");

    for world in &worlds {
        append_world(&mut svg, world, biomes, special, layout, t, thick);
    }

    svg.push_str("</svg>\n");
    svg
}

fn append_world(
    svg: &mut String,
    world: &World,
    biomes: &[crate::biome::Biome],
    special: &[crate::icon_lib::IconDef],
    layout: &crate::layout::Layout,
    t: f32,
    thick: f32,
) {
    for r in &world.rooms {
        let (border, is_thick) = room_border(r, biomes, special);
        let thickness = if is_thick { thick } else { t };
        let half = thickness / 2.0;
        let rect = room_rect(r, layout);
        let _ = writeln!(
            svg,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>",
            rect.min.x + half,
            rect.min.y + half,
            rect.width() - thickness,
            rect.height() - thickness,
            to_hex(room_fill(r, biomes, special)),
            to_hex(border),
            thickness,
        );
    }

    for d in &world.doors {
        let rect = door_rect(d, layout);
        let _ = writeln!(
            svg,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            rect.min.x,
            rect.min.y,
            rect.width(),
            rect.height(),
            to_hex(door_color(world, biomes, special, d)),
        );
    }

    for r in &world.rooms {
        for (id, corner, s) in room_icons(r, layout, special, PX_PER_UNIT) {
            let _ = writeln!(
                svg,
                "<use xlink:href=\"#{id}\" x=\"{}\" y=\"{}\" width=\"{s}\" height=\"{s}\"/>",
                corner.x, corner.y,
            );
        }
    }
}
