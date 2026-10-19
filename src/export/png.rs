//! PNG image of the map.

use super::{MapData, PX_PER_UNIT};
use crate::icons::Icons;
use crate::render::{self, MapView};
use macroquad::prelude::*;
use std::error::Error;

/// Largest image side, kept below common GPU texture limits. Big maps are scaled down.
const MAX_SIDE: f32 = 8192.0;

/// Renders the whole map into an offscreen target, on a transparent background.
pub fn render(map: &MapData, icons: &mut Icons) -> Result<Vec<u8>, Box<dyn Error>> {
    let (min, max) = map.area();
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
    let view = MapView {
        scale,
        visible: None,
        hovered: None,
    };
    render::draw_map(map.world, map.biomes, map.layout, &view, icons);
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
