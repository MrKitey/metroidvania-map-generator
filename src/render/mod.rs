//! Drawing of the whole frame: the map, then the screen overlays.

mod hud;
mod map;
mod picker;
mod theme;

pub use hud::{COPIED_TIME, Hud, RoomLegend};
pub use map::{
    MapView, door_rect, draw_grid, draw_map, icon_side_px, line_thickness, room_icons, room_rect,
};
pub use picker::{Ghost, Picker, picker_layout};
pub use theme::{door_color, room_border, room_fill};

use crate::biome::Biome;
use crate::icons::{IconId, Icons};
use crate::layout::Layout;
use crate::view::View;
use crate::world::World;
use macroquad::prelude::*;

/// Everything about the current frame that the renderer needs besides the world.
pub struct Frame<'a> {
    pub layout: &'a Layout,
    pub view: &'a View,
    pub camera: &'a Camera2D,
    pub hud: &'a Hud,
    pub legend: &'a RoomLegend,
    pub hovered: Option<usize>,
    pub help_open: bool,
    pub grid: bool,
    /// Open room picker, if any.
    pub picker: Option<&'a Picker>,
    /// Room being placed, if any.
    pub ghost: Option<Ghost>,
    /// Icon being dragged from the room type legend, if any.
    pub dragged_icon: Option<IconId>,
    /// Short message shown in the header, such as the export result.
    pub notice: Option<&'a str>,
}

/// World area shown by the camera, with one unit of margin so nothing pops at the edges.
fn visible_area(camera: &Camera2D) -> Rect {
    let a = camera.screen_to_world(Vec2::ZERO);
    let b = camera.screen_to_world(vec2(screen_width(), screen_height()));
    let (min, max) = (a.min(b), a.max(b));
    Rect::new(
        min.x - 1.0,
        min.y - 1.0,
        max.x - min.x + 2.0,
        max.y - min.y + 2.0,
    )
}

/// Draws one frame. The map uses the map camera, the rest uses screen pixels.
/// `legend_icons` is a second set used at a fixed size, so the map zoom does not
/// make the legend icons rasterize again every frame.
pub fn draw_world(
    world: &World,
    biomes: &[Biome],
    f: &Frame,
    icons: &mut Icons,
    legend_icons: &mut Icons,
) {
    clear_background(theme::BACKGROUND);
    set_camera(f.camera);
    let area = visible_area(f.camera);
    if f.grid {
        draw_grid(f.layout, f.view.scale, area);
    }
    let view = MapView {
        scale: f.view.scale,
        visible: Some(area),
        hovered: f.hovered,
    };
    draw_map(world, biomes, f.layout, &view, icons);
    if let Some(g) = &f.ghost {
        picker::draw_ghost(g, f.layout);
    }
    set_default_camera();

    hud::draw_header(world, biomes, f.notice, f.legend, legend_icons);
    hud::draw_seed(f.hud);
    if let Some(i) = f.hovered {
        hud::draw_tooltip(&world.rooms[i], biomes, f.layout);
    }
    if let Some(g) = &f.ghost {
        hud::draw_ghost_label(g, f.layout);
    }
    if let Some(icon) = f.dragged_icon {
        hud::draw_dragged_icon(icon, legend_icons);
    }
    hud::draw_export_button(f.hud);
    hud::draw_help(f.hud, f.help_open);
    if let Some(p) = f.picker {
        picker::draw_picker(p);
    }
}
