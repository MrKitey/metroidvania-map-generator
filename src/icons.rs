//! Room icons, loaded from the special-room library and rasterized on demand.

use crate::icon_lib::IconDef;
use crate::text::icons as ids;
use crate::world::{Marker, Room, RoomKind};
use egui::{Color32, ColorImage, Context, TextureHandle, TextureOptions};
use resvg::{tiny_skia, usvg};
use std::collections::HashMap;
use std::path::Path;

/// Raster size limits, in pixels per side.
const MIN_PX: u32 = 8;
const MAX_PX: u32 = 512;

/// Pixel size of the theme switch icons in the header.
pub const UI_ICON_PX: u32 = 48;

/// Effective icon id(s) of a room for display, at most two. A custom icon hides the
/// kind and marker icons.
pub fn room_icon_ids(r: &Room) -> impl Iterator<Item = &str> {
    let kind = match r.kind {
        RoomKind::Secret => Some(ids::SECRET),
        RoomKind::Portal => Some(ids::PORTAL),
        RoomKind::Normal | RoomKind::Corridor => None,
    };
    let marker = match r.marker {
        Some(Marker::Boss) => Some(ids::BOSS),
        Some(Marker::Save) => Some(ids::SAVE),
        None => None,
    };
    let (kind, marker) = if r.icon.is_some() {
        (None, None)
    } else {
        (kind, marker)
    };
    r.icon.as_deref().into_iter().chain(kind).chain(marker)
}

enum Source {
    Svg(Box<usvg::Tree>),
    Raster(image::RgbaImage),
}

struct Icon {
    source: Source,
    /// Last rasterized size and its texture.
    texture: Option<(u32, TextureHandle)>,
}

impl Icon {
    fn from_bytes(path: &Path, bytes: &[u8]) -> Option<Icon> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let source = if ext == "svg" {
            let tree = usvg::Tree::from_data(bytes, &usvg::Options::default()).ok()?;
            Source::Svg(Box::new(tree))
        } else {
            Source::Raster(image::load_from_memory(bytes).ok()?.to_rgba8())
        };
        Some(Icon {
            source,
            texture: None,
        })
    }

    fn from_def(def: &IconDef) -> Option<Icon> {
        let path = def.abs_path();
        let bytes = std::fs::read(&path).ok()?;
        Self::from_bytes(&path, &bytes)
    }

    fn texture(&mut self, ctx: &Context, name: &str, px: u32) -> &TextureHandle {
        let px = px.clamp(MIN_PX, MAX_PX);
        if self.texture.as_ref().is_none_or(|(size, _)| *size != px) {
            let image = match &self.source {
                Source::Svg(tree) => rasterize_svg(tree, px),
                Source::Raster(img) => scale_raster(img, px),
            };
            let handle = ctx.load_texture(format!("{name}-{px}"), image, TextureOptions::LINEAR);
            self.texture = Some((px, handle));
        }
        &self.texture.as_ref().unwrap().1
    }
}

fn rasterize_svg(tree: &usvg::Tree, px: u32) -> ColorImage {
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

fn scale_raster(img: &image::RgbaImage, px: u32) -> ColorImage {
    let scaled = image::imageops::resize(img, px, px, image::imageops::FilterType::Triangle);
    ColorImage::from_rgba_unmultiplied([px as usize, px as usize], &scaled)
}

/// Loads an embedded SVG once for a UI control.
pub fn texture_from_svg(ctx: &Context, name: &str, bytes: &[u8], px: u32) -> Option<TextureHandle> {
    let tree = usvg::Tree::from_data(bytes, &usvg::Options::default()).ok()?;
    let px = px.clamp(MIN_PX, MAX_PX);
    let image = rasterize_svg(&tree, px);
    Some(ctx.load_texture(name.to_owned(), image, TextureOptions::LINEAR))
}

/// Texture cache keyed by icon id.
pub struct Icons {
    icons: HashMap<String, Icon>,
}

impl Icons {
    pub fn new() -> Icons {
        Icons {
            icons: HashMap::new(),
        }
    }

    /// Rebuilds the cache from the current library definitions.
    pub fn sync(&mut self, defs: &[IconDef]) {
        let mut next = HashMap::new();
        for def in defs {
            if let Some(icon) = Icon::from_def(def) {
                next.insert(def.id.clone(), icon);
            }
        }
        self.icons = next;
    }

    /// Reloads one icon after its file changed.
    pub fn reload(&mut self, def: &IconDef) {
        if let Some(icon) = Icon::from_def(def) {
            self.icons.insert(def.id.clone(), icon);
        } else {
            self.icons.remove(&def.id);
        }
    }

    pub fn get(&mut self, ctx: &Context, id: &str, px: u32) -> Option<&TextureHandle> {
        let icon = self.icons.get_mut(id)?;
        Some(icon.texture(ctx, id, px))
    }
}

/// SVG markup (or an `<image>` wrapper) for export defs.
pub fn export_svg_def(def: &IconDef) -> Option<String> {
    let path = def.abs_path();
    let bytes = std::fs::read(&path).ok()?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext == "svg" {
        let raw = String::from_utf8_lossy(&bytes);
        let tag = format!("<svg id=\"{}\"", def.id);
        return Some(raw.trim().replacen("<svg", &tag, 1));
    }
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, img.width(), img.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(&img).ok()?;
        writer.finish().ok()?;
    }
    let b64 = base64_encode(&png);
    Some(format!(
        "<svg id=\"{}\" xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" \
         width=\"24\" height=\"24\" viewBox=\"0 0 24 24\">\
         <image width=\"24\" height=\"24\" href=\"data:image/png;base64,{b64}\"/></svg>",
        def.id
    ))
}

fn base64_encode(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let a = chunk[0] as u32;
        let b = chunk.get(1).copied().unwrap_or(0) as u32;
        let c = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (a << 16) | (b << 8) | c;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            T[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Fallback colors when an id is missing from the library.
pub fn fallback_colors(id: &str) -> Option<(Color32, Color32)> {
    match id {
        ids::BOSS => Some((
            Color32::from_rgb(0x80, 0x00, 0x00),
            Color32::from_rgb(0xFF, 0x00, 0x00),
        )),
        ids::SAVE => Some((
            Color32::from_rgb(0x00, 0x80, 0x00),
            Color32::from_rgb(0x00, 0xFF, 0x00),
        )),
        ids::SECRET => Some((
            Color32::from_rgb(0x80, 0x80, 0x00),
            Color32::from_rgb(0xFF, 0xFF, 0x00),
        )),
        ids::PORTAL => Some((
            Color32::from_rgb(0x00, 0x80, 0x80),
            Color32::from_rgb(0x00, 0xFF, 0xFF),
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(kind: RoomKind, marker: Option<Marker>, icon: Option<&str>) -> Room {
        Room {
            x: 0,
            y: 0,
            w: 1,
            h: 1,
            biome: 0,
            kind,
            marker,
            icon: icon.map(str::to_owned),
        }
    }

    fn ids_of(r: &Room) -> Vec<&str> {
        room_icon_ids(r).collect()
    }

    #[test]
    fn icon_ids_follow_kind_marker_and_custom_icon() {
        assert!(ids_of(&room(RoomKind::Normal, None, None)).is_empty());
        assert!(ids_of(&room(RoomKind::Corridor, None, None)).is_empty());
        assert_eq!(ids_of(&room(RoomKind::Secret, None, None)), [ids::SECRET]);
        assert_eq!(ids_of(&room(RoomKind::Portal, None, None)), [ids::PORTAL]);
        assert_eq!(
            ids_of(&room(RoomKind::Normal, Some(Marker::Boss), None)),
            [ids::BOSS]
        );
        assert_eq!(
            ids_of(&room(RoomKind::Secret, Some(Marker::Save), None)),
            [ids::SECRET, ids::SAVE]
        );
        assert_eq!(
            ids_of(&room(RoomKind::Portal, Some(Marker::Boss), Some("lava"))),
            ["lava"]
        );
    }
}
