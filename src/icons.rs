//! Marker icons, rasterized from embedded SVG files at the size they are drawn.

use crate::world::{Marker, RoomKind};
use macroquad::prelude::*;
use resvg::{tiny_skia, usvg};

/// Raster size limits, in pixels per side.
const MIN_PX: u32 = 8;
const MAX_PX: u32 = 512;

struct Icon {
    tree: usvg::Tree,
    /// Last rasterized size and its texture.
    texture: Option<(u32, Texture2D)>,
}

impl Icon {
    /// Panics if the SVG is invalid. The files are embedded, so this fails on every run.
    fn new(svg: &[u8]) -> Icon {
        let tree =
            usvg::Tree::from_data(svg, &usvg::Options::default()).expect("invalid embedded SVG");
        Icon {
            tree,
            texture: None,
        }
    }

    /// Square texture of `px` pixels per side, rasterized again only when the size changes.
    fn texture(&mut self, px: u32) -> &Texture2D {
        let px = px.clamp(MIN_PX, MAX_PX);
        if self.texture.as_ref().is_none_or(|(size, _)| *size != px) {
            self.texture = Some((px, rasterize(&self.tree, px)));
        }
        &self.texture.as_ref().unwrap().1
    }
}

/// Renders the SVG scaled to fit a `px` square.
fn rasterize(tree: &usvg::Tree, px: u32) -> Texture2D {
    let mut pixmap = tiny_skia::Pixmap::new(px, px).expect("px is at least MIN_PX");
    let size = tree.size();
    let scale = px as f32 / size.width().max(size.height());
    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    // tiny-skia stores premultiplied alpha, macroquad blends straight alpha.
    let bytes: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    Texture2D::from_rgba8(px as u16, px as u16, &bytes)
}

pub struct Icons {
    boss: Icon,
    save: Icon,
    secret: Icon,
    portal: Icon,
}

impl Icons {
    pub fn new() -> Icons {
        Icons {
            boss: Icon::new(include_bytes!("../assets/icons/boss.svg")),
            save: Icon::new(include_bytes!("../assets/icons/save.svg")),
            secret: Icon::new(include_bytes!("../assets/icons/secret.svg")),
            portal: Icon::new(include_bytes!("../assets/icons/portal.svg")),
        }
    }

    /// Icon of a room kind, for the kinds that have one.
    pub fn kind(&mut self, kind: RoomKind, px: u32) -> Option<&Texture2D> {
        match kind {
            RoomKind::Secret => Some(self.secret.texture(px)),
            RoomKind::Portal => Some(self.portal.texture(px)),
            RoomKind::Normal | RoomKind::Corridor => None,
        }
    }

    pub fn get(&mut self, marker: Marker, px: u32) -> &Texture2D {
        match marker {
            Marker::Boss => self.boss.texture(px),
            Marker::Save => self.save.texture(px),
        }
    }
}
