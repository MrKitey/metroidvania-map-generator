//! Biome list and room-type list on the left of the map.

use super::text_width;
use super::theme::*;
use crate::biome::Biome;
use crate::icon_lib::IconDef;
use crate::icons::Icons;
use crate::layout::MARGIN;
use egui::{Align2, Color32, Context, FontId, Pos2, Rect, Stroke, StrokeKind, Ui, pos2, vec2};

const ICON_BOX: f32 = 20.0;
const ICON_PX: u32 = 16;
const DRAG_ICON_PX: u32 = 32;
const SWATCH: f32 = 14.0;
const LEGEND_GAP: f32 = 8.0;

fn biome_legend_rect(ctx: &Context, biomes: &[Biome], map: Rect) -> Rect {
    let names_w = biomes
        .iter()
        .map(|b| text_width(ctx, &b.name, LEGEND_TEXT))
        .fold(0.0, f32::max);
    let w = PAD + SWATCH + PAD + names_w + PAD;
    let h = if biomes.is_empty() {
        0.0
    } else {
        PAD + biomes.len() as f32 * ROW_H + PAD
    };
    Rect::from_min_size(
        pos2(map.min.x + MARGIN, map.min.y + (map.height() - h) / 2.0),
        vec2(w.max(1.0), h),
    )
}

fn draw_biome_legend(ui: &Ui, biomes: &[Biome], legend: &RoomLegend, paint: Option<usize>) {
    if legend.biome_rows.is_empty() {
        return;
    }
    let painter = ui.painter();
    let hover = ui.input(|i| i.pointer.hover_pos());
    painter.rect_filled(
        legend.biome_rect,
        4.0,
        Color32::from_rgba_unmultiplied(0, 0, 0, 153),
    );
    for &(id, row) in &legend.biome_rows {
        let biome = &biomes[id];
        let active = paint == Some(id);
        if active {
            painter.rect_filled(row, 0.0, Color32::from_rgba_unmultiplied(255, 255, 255, 40));
        } else if hover.is_some_and(|p| row.contains(p)) {
            painter.rect_filled(row, 0.0, Color32::from_rgba_unmultiplied(255, 255, 255, 28));
        }
        let swatch_y = row.min.y + (ROW_H - SWATCH) / 2.0;
        let swatch = Rect::from_min_size(pos2(row.min.x + PAD, swatch_y), vec2(SWATCH, SWATCH));
        painter.rect_filled(swatch, 2.0, biome.fill);
        let stroke_w = if active { 3.0 } else { 2.0 };
        painter.rect_stroke(
            swatch,
            2.0,
            Stroke::new(stroke_w, biome.border()),
            StrokeKind::Inside,
        );
        painter.text(
            pos2(row.min.x + PAD + SWATCH + PAD, row.min.y + ROW_H * 0.5),
            Align2::LEFT_CENTER,
            &biome.name,
            FontId::proportional(LEGEND_TEXT),
            biome.border(),
        );
    }
}

/// Room type legend, in a block under the biome legend. Built once per frame, so
/// hit tests and drawing agree.
pub struct RoomLegend {
    /// Whole room-type block, backdrop included.
    pub rect: Rect,
    biome_rect: Rect,
    /// One row per project biome: its index, and the clickable row.
    biome_rows: Vec<(usize, Rect)>,
    /// One row per special-room entry, in library order.
    rows: Vec<(String, Rect)>,
    /// Snapshot of labels and colors for drawing.
    entries: Vec<(String, Color32, Color32)>,
}

impl RoomLegend {
    pub fn new(
        ctx: &Context,
        biomes: &[Biome],
        special: &[IconDef],
        map: Rect,
    ) -> RoomLegend {
        let biome_rect = biome_legend_rect(ctx, biomes, map);
        let names_w = special
            .iter()
            .map(|d| text_width(ctx, &d.label, LEGEND_TEXT))
            .fold(0.0, f32::max);
        let w = PAD + ICON_BOX + PAD + names_w + PAD;
        let h = if special.is_empty() {
            0.0
        } else {
            PAD + special.len() as f32 * ROW_H + PAD
        };
        let rect = Rect::from_min_size(
            pos2(biome_rect.min.x, biome_rect.max.y + LEGEND_GAP),
            vec2(w.max(1.0), h),
        );
        let biome_w = biome_rect.width();
        let biome_rows = biomes
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let y = biome_rect.min.y + PAD + i as f32 * ROW_H;
                (
                    i,
                    Rect::from_min_size(pos2(biome_rect.min.x, y), vec2(biome_w, ROW_H)),
                )
            })
            .collect();
        let rows = special
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let y = rect.min.y + PAD + i as f32 * ROW_H;
                (
                    d.id.clone(),
                    Rect::from_min_size(pos2(rect.min.x, y), vec2(w, ROW_H)),
                )
            })
            .collect();
        let entries = special
            .iter()
            .map(|d| (d.label.clone(), d.fill, d.border))
            .collect();
        RoomLegend {
            rect,
            biome_rect,
            biome_rows,
            rows,
            entries,
        }
    }

    /// True when `p` is over the biome list or the room-type list.
    pub fn contains(&self, p: Pos2) -> bool {
        (!self.biome_rows.is_empty() && self.biome_rect.contains(p))
            || (!self.rows.is_empty() && self.rect.contains(p))
    }

    /// Biome index under `p`, if the cursor is on a biome row.
    pub fn biome_at(&self, p: Pos2) -> Option<usize> {
        self.biome_rows
            .iter()
            .find(|(_, row)| row.contains(p))
            .map(|(id, _)| *id)
    }

    /// Icon id of the row under `p`, if any.
    pub fn icon_at(&self, p: Pos2) -> Option<String> {
        self.rows
            .iter()
            .find(|(_, r)| r.contains(p))
            .map(|r| r.0.clone())
    }
}

/// Room types, one per row with its icon and name.
fn draw_room_legend(ui: &Ui, legend: &RoomLegend, icons: &mut Icons) {
    if legend.entries.is_empty() {
        return;
    }
    let painter = ui.painter();
    let hover = ui.input(|i| i.pointer.hover_pos());
    painter.rect_filled(
        legend.rect,
        4.0,
        Color32::from_rgba_unmultiplied(0, 0, 0, 153),
    );
    let uv = Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0));
    for (i, (name, fill, border)) in legend.entries.iter().enumerate() {
        let (id, row) = &legend.rows[i];
        if hover.is_some_and(|p| row.contains(p)) {
            painter.rect_filled(*row, 0.0, Color32::from_rgba_unmultiplied(255, 255, 255, 28));
        }
        let row_y = legend.rect.min.y + PAD + i as f32 * ROW_H;
        let box_y = row_y + (ROW_H - ICON_BOX) / 2.0;
        let boxed = Rect::from_min_size(
            pos2(legend.rect.min.x + PAD, box_y),
            vec2(ICON_BOX, ICON_BOX),
        );
        painter.rect_filled(boxed, 2.0, *fill);
        painter.rect_stroke(boxed, 2.0, Stroke::new(2.0, *border), StrokeKind::Inside);
        let side = ICON_PX as f32;
        let offset = (ICON_BOX - side) / 2.0;
        let dest = Rect::from_min_size(boxed.min + vec2(offset, offset), vec2(side, side));
        if let Some(tex) = icons.get(ui.ctx(), id, ICON_PX) {
            painter.image(tex.id(), dest, uv, Color32::WHITE);
        }
        painter.text(
            pos2(
                legend.rect.min.x + PAD + ICON_BOX + PAD,
                row_y + ROW_H * 0.5,
            ),
            Align2::LEFT_CENTER,
            name,
            FontId::proportional(LEGEND_TEXT),
            *border,
        );
    }
}

/// Biome legend and room type legend on the left of the map.
pub(super) fn draw_legends(
    ui: &Ui,
    biomes: &[Biome],
    legend: &RoomLegend,
    legend_icons: &mut Icons,
    paint_biome: Option<usize>,
) {
    draw_biome_legend(ui, biomes, legend, paint_biome);
    draw_room_legend(ui, legend, legend_icons);
}

/// Icon being dragged from the legend, centered on the cursor.
pub(super) fn draw_dragged_icon(
    ui: &Ui,
    icon_id: &str,
    special: &[IconDef],
    icons: &mut Icons,
) {
    let Some(pos) = ui.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let painter = ui.painter();
    let side = DRAG_ICON_PX as f32;
    let boxed = side * ICON_BOX / ICON_PX as f32;
    if let Some(def) = special.iter().find(|d| d.id == icon_id) {
        let rect = Rect::from_center_size(pos, vec2(boxed, boxed));
        painter.rect_filled(rect, 2.0, def.fill);
        painter.rect_stroke(rect, 2.0, Stroke::new(2.0, def.border), StrokeKind::Inside);
    }
    let dest = Rect::from_center_size(pos, vec2(side, side));
    if let Some(tex) = icons.get(ui.ctx(), icon_id, DRAG_ICON_PX) {
        painter.image(
            tex.id(),
            dest,
            Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
}
