//! Room list of the side panel, and the preview of the room being dragged.

use super::map::line_thickness;
use super::pack::pack_rooms;
use crate::editor::{self, SizeFields};
use crate::layout::Layout;
use crate::settings::{DEFAULT_BOSS_SIZES, DEFAULT_CORRIDOR_SIZES, DEFAULT_ROOM_SIZES};
use crate::text::{editor as editor_text, rooms};
use crate::view::View;
use crate::world::{Choice, Pick, Sizes};
use egui::{
    Align2, Color32, Context, FontId, Id, LayerId, Order, Painter, PointerButton, Pos2, Rect,
    Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};

const PICKER_TEXT: f32 = 14.0;
/// Extra space between the foldable sections.
const SECTION_GAP: f32 = 16.0;

/// Outline, label and hover fill for the room size previews.
fn picker_colors(dark: bool) -> (Color32, Color32, Color32) {
    if dark {
        (
            Color32::WHITE,
            Color32::WHITE,
            Color32::from_rgba_unmultiplied(255, 255, 255, 38),
        )
    } else {
        (
            Color32::from_rgb(40, 40, 55),
            Color32::from_rgb(40, 40, 55),
            Color32::from_rgba_unmultiplied(0, 0, 0, 30),
        )
    }
}

/// Room sizes currently shown, with screen positions already computed.
pub struct Picker {
    pub items: Vec<(Choice, Rect)>,
}

/// Room being placed, snapped to the cell grid. `free` is false if it overlaps a room.
pub struct Ghost {
    pub x: i32,
    pub y: i32,
    pub choice: Choice,
    pub free: bool,
}

/// One open section: white outlines, each with its size in the middle.
fn draw_section(
    ui: &mut Ui,
    pick: Pick,
    list: &[(u32, u32)],
    layout: &Layout,
) -> Vec<(Choice, Rect)> {
    let width = ui.available_width().max(1.0);
    let origin = ui.cursor().min;
    let (placed, bottom) = pack_rooms(list, pick, layout, width);
    let items: Vec<_> = placed
        .into_iter()
        .map(|(choice, rect)| (choice, rect.translate(origin.to_vec2())))
        .collect();
    if bottom > 0.0 {
        let right = items
            .iter()
            .fold(origin.x, |max, (_, rect)| max.max(rect.max.x));
        ui.allocate_rect(
            Rect::from_min_size(origin, vec2((right - origin.x).max(1.0), bottom)),
            egui::Sense::hover(),
        );
    }
    let mouse = ui.input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::ZERO);
    let (stroke, text, hover) = picker_colors(ui.visuals().dark_mode);
    let painter = ui.painter();
    for (choice, rect) in &items {
        if rect.contains(mouse) {
            painter.rect_filled(*rect, 0.0, hover);
        }
        painter.rect_stroke(*rect, 0.0, Stroke::new(2.0, stroke), StrokeKind::Inside);
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            format!("{}x{}", choice.w, choice.h),
            FontId::proportional(PICKER_TEXT),
            text,
        );
    }
    items
}

/// Draws the room list in the side panel.
/// Returns the sizes on screen, the entry under the pointer when the drag starts,
/// and whether any size list was edited.
pub fn draw_room_list(
    ui: &mut Ui,
    sizes: &Sizes,
    layout: &Layout,
    room_fields: &mut SizeFields,
    boss_fields: &mut SizeFields,
    corridor_fields: &mut SizeFields,
) -> (Picker, Option<(Choice, Vec2)>, bool) {
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let sections = [
                (
                    rooms::ROOMS,
                    Pick::Room,
                    sizes.rooms.as_slice(),
                    editor_text::room_sizes_label(editor::MAX_ROOM_SIDE),
                    room_fields,
                    DEFAULT_ROOM_SIZES,
                ),
                (
                    rooms::BOSS_ROOMS,
                    Pick::Boss,
                    sizes.boss.as_slice(),
                    editor_text::boss_sizes_label(editor::MAX_ROOM_SIDE),
                    boss_fields,
                    DEFAULT_BOSS_SIZES,
                ),
                (
                    rooms::CORRIDORS,
                    Pick::Corridor,
                    sizes.corridor.as_slice(),
                    editor_text::corridor_sizes_label(editor::MAX_ROOM_SIDE),
                    corridor_fields,
                    DEFAULT_CORRIDOR_SIZES,
                ),
            ];
            let pressed = ui.input(|i| i.pointer.button_pressed(PointerButton::Primary));
            let pos = ui.input(|i| i.pointer.hover_pos());
            let mut items = Vec::new();
            let mut drag = None;
            let mut changed = false;
            for (i, (title, pick, list, size_label, fields, defaults)) in
                sections.into_iter().enumerate()
            {
                if i > 0 {
                    ui.add_space(SECTION_GAP);
                }
                egui::CollapsingHeader::new(title)
                    .id_salt(("room-section", i))
                    .default_open(i == 0)
                    .show(ui, |ui| {
                        let section = draw_section(ui, pick, list, layout);
                        if drag.is_none() && pressed {
                            drag = pos.and_then(|pos| {
                                section
                                    .iter()
                                    .find(|(_, rect)| rect.contains(pos))
                                    .map(|(choice, rect)| (*choice, rect.size()))
                            });
                        }
                        items.extend(section);
                        ui.collapsing(size_label, |ui| {
                            if editor::size_list(ui, fields, defaults) {
                                changed = true;
                            }
                        });
                    });
            }
            (Picker { items }, drag, changed)
        })
        .inner
}

/// Outline of the dragged room, centered on the cursor, above the panels.
/// `size` is the size of that entry in the list.
pub fn draw_drag_outline(ctx: &Context, choice: Choice, size: Vec2) {
    let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let (stroke, text, _) = picker_colors(ctx.global_style().visuals.dark_mode);
    let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::unique("room-drag")));
    let rect = Rect::from_center_size(pos, size);
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(2.0, stroke),
        StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        format!("{}x{}", choice.w, choice.h),
        FontId::proportional(PICKER_TEXT),
        text,
    );
}

/// Room being placed, in the map camera. Green if free, red if not.
pub fn draw_ghost(painter: &Painter, g: &Ghost, layout: &Layout, view: &View, map: Rect) {
    let world = Rect::from_min_size(
        pos2(g.x as f32 * layout.cw, g.y as f32 * layout.ch),
        vec2(g.choice.w as f32 * layout.cw, g.choice.h as f32 * layout.ch),
    );
    let rect = Rect::from_min_max(
        view.world_to_screen(world.min.to_vec2(), map),
        view.world_to_screen(world.max.to_vec2(), map),
    );
    let fill = if g.free {
        Color32::from_rgba_unmultiplied(0, 255, 0, 89)
    } else {
        Color32::from_rgba_unmultiplied(255, 0, 0, 89)
    };
    painter.rect_filled(rect, 0.0, fill);
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(line_thickness(layout) * view.scale, Color32::WHITE),
        StrokeKind::Middle,
    );
}
