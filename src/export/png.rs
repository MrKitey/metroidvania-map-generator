//! PNG image of the map, rasterized from the SVG on the CPU.

use super::MapData;
use png::{BitDepth, ColorType, Encoder};
use resvg::{tiny_skia, usvg};
use std::error::Error;

/// Largest image side, kept below common size limits. Big maps are scaled down.
const MAX_SIDE: f32 = 8192.0;

/// Renders the whole map into a PNG, on a transparent background.
pub fn render(map: &MapData) -> Result<Vec<u8>, Box<dyn Error>> {
    let svg = super::svg::render(map);
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default())?;
    let size = tree.size();
    let scale = (MAX_SIDE / size.width())
        .min(MAX_SIDE / size.height())
        .min(1.0);
    let (w, h) = (
        (size.width() * scale).round().max(1.0) as u32,
        (size.height() * scale).round().max(1.0) as u32,
    );

    let mut pixmap = tiny_skia::Pixmap::new(w, h).expect("png size is at least 1");
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    let rgba: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();

    let mut png = Vec::new();
    let mut encoder = Encoder::new(&mut png, w, h);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&rgba)?;
    writer.finish()?;
    Ok(png)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biome;
    use crate::layout::Layout;
    use crate::settings::{
        DEFAULT_BOSS_SIZES, DEFAULT_CORRIDOR_SIZES, DEFAULT_ROOM_SIZES, Settings,
    };
    use crate::world::{Params, Sizes, World};

    #[test]
    fn svg_rasterizes_to_png() {
        let biomes = biome::default_biomes();
        let extras: Vec<_> = biomes.iter().map(biome::Biome::extras).collect();
        let sizes = Sizes {
            rooms: DEFAULT_ROOM_SIZES.to_vec(),
            boss: DEFAULT_BOSS_SIZES.to_vec(),
            corridor: DEFAULT_CORRIDOR_SIZES.to_vec(),
        };
        let world = World::generate(&Params {
            cells: 96,
            seed: 7,
            extras: &extras,
            min_rooms: 5,
            max_rooms: 50,
            sizes: &sizes,
        });
        let layout = Layout::new((640, 360));
        let settings = Settings::default();
        let map = MapData {
            seed: 7,
            cells: 96,
            settings: &settings,
            world: &world,
            biomes: &biomes,
            layout: &layout,
        };
        let png = render(&map).expect("svg rasterizes");
        assert_eq!(&png[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        assert!(png.len() > 100);
    }
}
