//! Room icons, rasterized from embedded SVG files at the size they are drawn.

use crate::text::icons as text;
use crate::world::{Marker, RoomKind};
use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use resvg::{tiny_skia, usvg};

/// Raster size limits, in pixels per side.
const MIN_PX: u32 = 8;
const MAX_PX: u32 = 512;

/// One of the icons shown on rooms and in the legend.
#[derive(Clone, Copy, PartialEq)]
pub enum IconId {
    Boss,
    Save,
    Secret,
    Portal,
}

impl IconId {
    pub const ALL: [IconId; 4] = [IconId::Boss, IconId::Save, IconId::Secret, IconId::Portal];

    /// Icon of a room kind, for the kinds that have one.
    pub fn of_kind(kind: RoomKind) -> Option<IconId> {
        match kind {
            RoomKind::Secret => Some(IconId::Secret),
            RoomKind::Portal => Some(IconId::Portal),
            RoomKind::Normal | RoomKind::Corridor => None,
        }
    }

    pub fn of_marker(marker: Marker) -> IconId {
        match marker {
            Marker::Boss => IconId::Boss,
            Marker::Save => IconId::Save,
        }
    }

    /// Short name, used as the icon id in the SVG export.
    pub fn name(self) -> &'static str {
        match self {
            IconId::Boss => text::BOSS,
            IconId::Save => text::SAVE,
            IconId::Secret => text::SECRET,
            IconId::Portal => text::PORTAL,
        }
    }

    /// Embedded SVG source. The SVG export reuses it as it is.
    pub fn svg(self) -> &'static str {
        match self {
            IconId::Boss => include_str!("../assets/icons/boss.svg"),
            IconId::Save => include_str!("../assets/icons/save.svg"),
            IconId::Secret => include_str!("../assets/icons/secret.svg"),
            IconId::Portal => include_str!("../assets/icons/portal.svg"),
        }
    }
}

struct Icon {
    tree: usvg::Tree,
    /// Last rasterized size and its texture.
    texture: Option<(u32, TextureHandle)>,
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
    fn texture(&mut self, ctx: &Context, name: &str, px: u32) -> &TextureHandle {
        let px = px.clamp(MIN_PX, MAX_PX);
        if self.texture.as_ref().is_none_or(|(size, _)| *size != px) {
            let image = rasterize(&self.tree, px);
            let handle = ctx.load_texture(format!("{name}-{px}"), image, TextureOptions::LINEAR);
            self.texture = Some((px, handle));
        }
        &self.texture.as_ref().unwrap().1
    }
}

/// Renders the SVG scaled to fit a `px` square.
fn rasterize(tree: &usvg::Tree, px: u32) -> ColorImage {
    let mut pixmap = tiny_skia::Pixmap::new(px, px).expect("px is at least MIN_PX");
    let size = tree.size();
    let scale = px as f32 / size.width().max(size.height());
    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let bytes: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    ColorImage::from_rgba_unmultiplied([px as usize, px as usize], &bytes)
}

/// Texture cache of all icons, one raster size per icon.
pub struct Icons {
    icons: [Icon; 4],
}

impl Icons {
    pub fn new() -> Icons {
        Icons {
            icons: IconId::ALL.map(|id| Icon::new(id.svg().as_bytes())),
        }
    }

    pub fn get(&mut self, ctx: &Context, id: IconId, px: u32) -> &TextureHandle {
        self.icons[id as usize].texture(ctx, id.name(), px)
    }
}
