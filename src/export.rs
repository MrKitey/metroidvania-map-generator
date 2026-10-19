//! Export of the current map as a zip holding map.png, map.svg and map.json.

use crate::biome::{self, Biome};
use crate::icons::{BOSS_SVG, Icons, PORTAL_SVG, SAVE_SVG, SECRET_SVG};
use crate::render::{self, Layout};
use crate::settings::Settings;
use crate::world::{Marker, RoomKind, World};
use macroquad::prelude::*;
use serde::Serialize;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;

/// Image pixels per world unit. One cell is one unit tall.
const PX_PER_UNIT: f32 = 32.0;
/// Largest image side, kept below common GPU texture limits. Big maps are scaled down.
const MAX_SIDE: f32 = 8192.0;
/// Free space around the map in the image, in world units.
const PADDING: f32 = 0.5;

/// Everything written to the export.
pub struct MapData<'a> {
    pub seed: u64,
    pub cells: u32,
    pub settings: &'a Settings,
    pub world: &'a World,
    pub biomes: &'a [Biome],
    pub layout: &'a Layout,
}

#[derive(Serialize)]
struct MapFile<'a> {
    seed: u64,
    cells: u32,
    settings: &'a Settings,
    biomes: Vec<BiomeEntry>,
    /// Indices into `biomes`, in placement order.
    used_biomes: &'a [usize],
    rooms: Vec<RoomEntry>,
    doors: Vec<DoorEntry>,
}

#[derive(Serialize)]
struct BiomeEntry {
    name: String,
    color: String,
    corridors: u32,
    portals: u32,
    secrets: u32,
    bosses: u32,
    saves: u32,
}

/// Position and size in cells. `biome` is an index into `biomes`.
#[derive(Serialize)]
struct RoomEntry {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    biome: usize,
    kind: &'static str,
    marker: Option<&'static str>,
}

/// Door between rooms `a` and `b`, indices into `rooms`. On the left edge of cell
/// (x, y) when `vertical`, otherwise on its top edge.
#[derive(Serialize)]
struct DoorEntry {
    a: usize,
    b: usize,
    x: i32,
    y: i32,
    vertical: bool,
}

/// Writes the zip to `path`. Must be called from the main loop, since the image is
/// rendered on the GPU.
pub fn write_zip(path: &Path, map: &MapData, icons: &mut Icons) -> Result<(), Box<dyn Error>> {
    let png = render_png(map, icons)?;
    let svg = render_svg(map);
    let json = serde_json::to_string_pretty(&map_file(map))?;

    let mut zip = zip::ZipWriter::new(File::create(path)?);
    // PNG data is already compressed.
    let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file("map.png", stored)?;
    zip.write_all(&png)?;
    zip.start_file("map.svg", SimpleFileOptions::default())?;
    zip.write_all(svg.as_bytes())?;
    zip.start_file("map.json", SimpleFileOptions::default())?;
    zip.write_all(json.as_bytes())?;
    zip.finish()?;
    Ok(())
}

/// Renders the whole map into an offscreen target, on a transparent background.
fn render_png(map: &MapData, icons: &mut Icons) -> Result<Vec<u8>, Box<dyn Error>> {
    let (x0, y0, x1, y1) = map.world.bounds();
    let (cw, ch) = (map.layout.cw, map.layout.ch);
    let min = vec2(x0 as f32 * cw, y0 as f32 * ch) - PADDING;
    let max = vec2(x1 as f32 * cw, y1 as f32 * ch) + PADDING;
    let size = max - min;
    let scale = PX_PER_UNIT.min(MAX_SIDE / size.x).min(MAX_SIDE / size.y);
    let (w, h) = (
        (size.x * scale).ceil() as u32,
        (size.y * scale).ceil() as u32,
    );

    let target = render_target(w, h);
    // macroquad does not flip y for render targets, and OpenGL reads rows bottom up,
    // so the pixels come back with the map top on the first row.
    let camera = Camera2D {
        target: (min + max) / 2.0,
        zoom: vec2(2.0 * scale / w as f32, 2.0 * scale / h as f32),
        render_target: Some(target.clone()),
        ..Default::default()
    };
    set_camera(&camera);
    clear_background(BLANK);
    render::draw_map(map.world, map.biomes, map.layout, scale, None, icons);
    // Switching camera flushes the pending draw calls into the target.
    set_default_camera();
    let image = target.texture.get_texture_data();

    let mut png = Vec::new();
    let mut encoder = png::Encoder::new(&mut png, image.width as u32, image.height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&image.bytes)?;
    writer.finish()?;
    Ok(png)
}

/// Vector version of the map, in world units, on a transparent background.
/// Mirrors `render::draw_map`: rooms, then doors, then icons.
fn render_svg(map: &MapData) -> String {
    let (world, biomes) = (map.world, map.biomes);
    let (x0, y0, x1, y1) = world.bounds();
    let Layout { cw, ch, .. } = *map.layout;
    let min = vec2(x0 as f32 * cw, y0 as f32 * ch) - PADDING;
    let size = vec2(x1 as f32 * cw, y1 as f32 * ch) + PADDING - min;
    // Border and door thickness. A thick border is one image pixel thicker.
    let t = cw.min(ch) * 0.12;
    let thick = t + 1.0 / PX_PER_UNIT;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" \
         viewBox=\"{} {} {} {}\" width=\"{}\" height=\"{}\">\n",
        min.x,
        min.y,
        size.x,
        size.y,
        size.x * PX_PER_UNIT,
        size.y * PX_PER_UNIT,
    );

    // Each icon is defined once, then placed with <use>.
    svg.push_str("<defs>\n");
    for (id, source) in [
        ("boss", BOSS_SVG),
        ("save", SAVE_SVG),
        ("secret", SECRET_SVG),
        ("portal", PORTAL_SVG),
    ] {
        let tag = format!("<svg id=\"{id}\"");
        svg.push_str(&source.trim().replacen("<svg", &tag, 1));
        svg.push('\n');
    }
    svg.push_str("</defs>\n");

    for (i, r) in world.rooms.iter().enumerate() {
        let fill = biome::to_hex(render::room_fill(world, biomes, i));
        let (border, is_thick) = render::room_border(world, biomes, i);
        let thickness = if is_thick { thick } else { t };
        // SVG strokes are centered on the edge, so the rectangle is inset by half of it
        // to keep the border inside the room, as in the app.
        let half = thickness / 2.0;
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>\n",
            r.x as f32 * cw + half,
            r.y as f32 * ch + half,
            r.w as f32 * cw - thickness,
            r.h as f32 * ch - thickness,
            fill,
            biome::to_hex(border),
            thickness,
        ));
    }

    // Doors open a gap in the borders.
    for d in &world.doors {
        let color = biome::to_hex(render::door_color(world, biomes, d));
        let (x, y) = (d.x as f32 * cw, d.y as f32 * ch);
        let (x, y, w, h) = if d.vertical {
            (x - t, y + ch * 0.3, 2.0 * t, ch * 0.4)
        } else {
            (x + cw * 0.3, y - t, cw * 0.4, 2.0 * t)
        };
        svg.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"{color}\"/>\n"
        ));
    }

    let side = (cw.min(ch) * 0.7).max(12.0 / PX_PER_UNIT);
    for r in &world.rooms {
        let kind = match r.kind {
            RoomKind::Secret => Some("secret"),
            RoomKind::Portal => Some("portal"),
            RoomKind::Normal | RoomKind::Corridor => None,
        };
        let marker = r.marker.map(|m| match m {
            Marker::Boss => "boss",
            Marker::Save => "save",
        });
        // A room with a kind icon and a marker shows both, side by side, smaller.
        let both = kind.is_some() && marker.is_some();
        let s = if both { side * 0.6 } else { side };
        let (cx, cy) = (
            (r.x as f32 + r.w as f32 / 2.0) * cw,
            (r.y as f32 + r.h as f32 / 2.0) * ch,
        );
        let mut x = if both { cx - s } else { cx - s / 2.0 };
        for id in kind.into_iter().chain(marker) {
            svg.push_str(&format!(
                "<use xlink:href=\"#{id}\" x=\"{x}\" y=\"{}\" width=\"{s}\" height=\"{s}\"/>\n",
                cy - s / 2.0,
            ));
            x += s;
        }
    }

    svg.push_str("</svg>\n");
    svg
}

fn map_file<'a>(map: &MapData<'a>) -> MapFile<'a> {
    let world = map.world;
    MapFile {
        seed: map.seed,
        cells: map.cells,
        settings: map.settings,
        biomes: map
            .biomes
            .iter()
            .map(|b| BiomeEntry {
                name: b.name.clone(),
                color: biome::to_hex(b.fill),
                corridors: b.corridors,
                portals: b.portals,
                secrets: b.secrets,
                bosses: b.bosses,
                saves: b.saves,
            })
            .collect(),
        used_biomes: &world.biomes,
        rooms: world
            .rooms
            .iter()
            .map(|r| RoomEntry {
                x: r.x,
                y: r.y,
                w: r.w,
                h: r.h,
                biome: r.biome,
                kind: match r.kind {
                    RoomKind::Normal => "normal",
                    RoomKind::Corridor => "corridor",
                    RoomKind::Secret => "secret",
                    RoomKind::Portal => "portal",
                },
                marker: r.marker.map(|m| match m {
                    Marker::Boss => "boss",
                    Marker::Save => "save",
                }),
            })
            .collect(),
        doors: world
            .doors
            .iter()
            .map(|d| DoorEntry {
                a: d.a,
                b: d.b,
                x: d.x,
                y: d.y,
                vertical: d.vertical,
            })
            .collect(),
    }
}
