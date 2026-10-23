//! Drawing of the whole frame: the map, then the screen overlays.

mod hud;
mod legend;
mod map;
mod pack;
mod picker;
mod theme;

pub use hud::{COPIED_TIME, Hud, draw_help, draw_notice};
pub use legend::RoomLegend;
pub use map::{
    MapView, door_rect, draw_grid, draw_layer_group, draw_map_opacity, icon_side_px,
    layer_label_rects, line_thickness, room_icons, room_rect,
};
pub use picker::{Ghost, Picker, draw_drag_outline, draw_room_list};
pub use theme::{
    BACKGROUND, BUTTON_BG, BUTTON_HOVER, background, door_color, room_border, room_fill,
};

use crate::biome::Biome;
use crate::icon_lib::IconDef;
use crate::icons::Icons;
use crate::layer::LayerStack;
use crate::layout::Layout;
use crate::view::View;
use egui::{Color32, CursorIcon, FontId, Rect, Stroke, StrokeKind, Ui, vec2};

pub(super) fn text_width(ctx: &egui::Context, text: &str, size: f32) -> f32 {
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(text.to_owned(), FontId::proportional(size), Color32::WHITE)
            .size()
            .x
    })
}

/// Everything about the current frame that the renderer needs besides the world.
pub struct Frame<'a> {
    pub layout: &'a Layout,
    pub view: &'a View,
    pub map_rect: Rect,
    pub hud: &'a Hud,
    pub legend: &'a RoomLegend,
    pub hovered: Option<usize>,
    /// Layer index of the hovered room, for tooltip lookup.
    pub hovered_layer: Option<usize>,
    pub grid: bool,
    /// Layer group fill, border and name labels.
    pub layer_frames: bool,
    /// Room being placed, if any.
    pub ghost: Option<Ghost>,
    /// Rooms being dragged, drawn together in green or red.
    pub move_ghosts: Vec<Ghost>,
    /// Rooms highlighted by the selection rectangle.
    pub selected: &'a [usize],
    /// Selection rectangle, in screen points, while the button is held.
    pub select_rect: Option<Rect>,
    /// Icon id being dragged from the room type legend, if any.
    pub dragged_icon: Option<String>,
    /// Biome currently used for paint mode, if any.
    pub paint_biome: Option<usize>,
    /// Special-room library used for colors and the drag preview.
    pub special: &'a [IconDef],
    /// Dark map canvas when true.
    pub dark: bool,
}

/// World area shown by the camera, with one unit of margin so nothing pops at the edges.
fn visible_area(view: &View, map: Rect) -> Rect {
    let a = view.screen_to_world(map.min, map);
    let b = view.screen_to_world(map.max, map);
    Rect::from_min_max(a.to_pos2(), b.to_pos2()).expand(1.0)
}

/// Draws every visible layer, then overlays.
pub fn draw_layers(
    ui: &mut Ui,
    layers: &LayerStack,
    biomes: &[Biome],
    f: &Frame,
    icons: &mut Icons,
    legend_icons: &mut Icons,
) {
    let painter = ui.painter().with_clip_rect(f.map_rect);
    painter.rect_filled(f.map_rect, 0.0, theme::background(f.dark));
    let area = visible_area(f.view, f.map_rect);
    if f.grid {
        draw_grid(&painter, f.layout, f.view, f.map_rect, area, f.dark);
    }
    let active = layers.active();
    for (i, layer) in layers.layers().iter().enumerate() {
        if !layer.visible {
            continue;
        }
        if f.layer_frames {
            draw_layer_group(
                &painter,
                ui.ctx(),
                layer,
                i == active,
                f.layout,
                f.view,
                f.map_rect,
            );
        }
        let selected = if i == active { f.selected } else { &[][..] };
        let hovered = if f.hovered_layer == Some(i) {
            f.hovered
        } else {
            None
        };
        let view = MapView {
            view: f.view,
            map: f.map_rect,
            scale: f.view.scale,
            visible: Some(area),
            hovered,
            selected,
        };
        draw_map_opacity(
            &painter,
            &layer.world,
            biomes,
            f.special,
            f.layout,
            &view,
            icons,
            layer.opacity,
        );
    }
    if let Some(g) = &f.ghost {
        picker::draw_ghost(&painter, g, f.layout, f.view, f.map_rect);
    }
    for g in &f.move_ghosts {
        picker::draw_ghost(&painter, g, f.layout, f.view, f.map_rect);
    }
    if let Some(rect) = f.select_rect {
        let (fill, stroke) = if f.dark {
            (
                Color32::from_rgba_unmultiplied(255, 255, 255, 30),
                Color32::WHITE,
            )
        } else {
            (
                Color32::from_rgba_unmultiplied(0, 0, 0, 30),
                Color32::from_rgb(40, 40, 55),
            )
        };
        painter.rect_filled(rect, 0.0, fill);
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, stroke), StrokeKind::Inside);
    }

    legend::draw_legends(ui, biomes, f.legend, legend_icons, f.paint_biome);
    hud::draw_rooms(ui, f.hud, f.dark);
    hud::draw_seed(ui, f.hud, f.dark);
    if let (Some(li), Some(ri)) = (f.hovered_layer, f.hovered) {
        let room = &layers.layers()[li].world.rooms[ri];
        hud::draw_tooltip(ui, room, biomes, f.special, f.layout);
    }
    if let Some(g) = &f.ghost {
        hud::draw_ghost_label(ui, g, f.layout);
    } else if let [g] = f.move_ghosts.as_slice() {
        hud::draw_ghost_label(ui, g, f.layout);
    }
    if let Some(icon) = &f.dragged_icon {
        legend::draw_dragged_icon(ui, icon, f.special, legend_icons);
    }
}

/// UI theme, including the map canvas fill used by the central panel.
pub fn apply_theme(ctx: &egui::Context, dark: bool) {
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    if dark {
        visuals.panel_fill = BACKGROUND;
        visuals.window_fill = egui::Color32::from_rgb(20, 20, 28);
        visuals.extreme_bg_color = BACKGROUND;
        visuals.widgets.inactive.bg_fill = BUTTON_BG;
        visuals.widgets.hovered.bg_fill = BUTTON_HOVER;
        visuals.widgets.active.bg_fill = BUTTON_HOVER;
    } else {
        visuals.panel_fill = egui::Color32::from_rgb(236, 236, 242);
        visuals.window_fill = egui::Color32::from_rgb(248, 248, 252);
        visuals.extreme_bg_color = egui::Color32::from_rgb(220, 220, 228);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(220, 220, 230);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(200, 200, 214);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(200, 200, 214);
    }
    visuals.interact_cursor = Some(CursorIcon::PointingHand);
    ctx.set_visuals(visuals);
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = vec2(8.0, 6.0);
    });
}
