//! Map panel: camera, pointer for this frame, and the draw call.

use super::interact::Pointer;
use super::{App, History};
use crate::layout::Layout;
use crate::render::{self, Frame, Hud, Picker, RoomLegend};
use egui::{PointerButton, Pos2, Sense};

impl App {
    pub(super) fn draw_map_panel(&mut self, ui: &mut egui::Ui, now: f64, rooms: Option<&Picker>) {
        let ctx = ui.ctx().clone();
        let (response, _) = ui.allocate_painter(ui.available_size(), Sense::hover());
        let map_rect = response.rect;
        let layout = Layout::new(self.settings.viewport());
        if self.fit {
            let bounds = self.layers.visible_bounds().unwrap_or((0, 0, 1, 1));
            self.view.fit_bounds(bounds, &layout, map_rect);
            self.fit = false;
        }

        let pointer_pos = ctx.input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::ZERO);
        if response.contains_pointer() {
            let wheel = ctx.input(|i| i.smooth_scroll_delta.y);
            if wheel != 0.0 {
                self.view.zoom_at(wheel, pointer_pos, map_rect);
            }
        }
        if ctx.input(|i| i.pointer.button_pressed(PointerButton::Middle))
            && response.contains_pointer()
        {
            self.interact.panning = true;
        }
        if ctx.input(|i| !i.pointer.button_down(PointerButton::Middle)) {
            self.interact.panning = false;
        }
        if self.interact.panning {
            self.view.pan(ctx.input(|i| i.pointer.delta()));
        }

        let copied = now - self.copied_at < render::COPIED_TIME;
        let hud = Hud::new(&ctx, self.seed, copied, self.layers.room_count(), map_rect);
        let legend = RoomLegend::new(&ctx, &self.biomes, &self.special, map_rect);
        let mut history = History {
            undo: &mut self.undo,
            seed: self.seed,
            cells: self.cells,
            biomes: &self.biomes,
        };
        let mut pointer = Pointer {
            pos: pointer_pos,
            left: ctx.input(|i| i.pointer.button_pressed(PointerButton::Primary)),
            left_down: ctx.input(|i| i.pointer.button_down(PointerButton::Primary)),
            right: ctx.input(|i| i.pointer.button_pressed(PointerButton::Secondary)),
            over_seed: hud.seed.contains(pointer_pos),
            over_legend: legend.contains(pointer_pos),
            on_map_rect: response.contains_pointer(),
            consumed: false,
        };

        if pointer.over_seed && pointer.left {
            ctx.copy_text(self.seed.to_string());
            self.copied_at = now;
        }
        if pointer.left
            && !pointer.consumed
            && let Some(biome) = legend.biome_at(pointer.pos)
        {
            self.interact.set_paint_biome(biome);
            pointer.consumed = true;
        }

        let layer_labels = if self.layer_frames {
            render::layer_label_rects(&ctx, &self.layers, &layout, &self.view, map_rect)
        } else {
            Vec::new()
        };
        self.interact.begin_layer_drag(
            &mut pointer,
            &mut self.layers,
            &layer_labels,
            &self.view,
            &layout,
            map_rect,
        );

        let ghost = self.interact.update_placing(
            &mut self.layers,
            &self.view,
            &mut pointer,
            &layout,
            map_rect,
            &mut history,
        );
        if ghost.is_none()
            && let Some(choice) = self.interact.placing
        {
            render::draw_drag_outline(&ctx, choice, self.interact.placing_size);
        }

        // Cell under the cursor: hover target, and where dragged rooms are anchored.
        let cell = layout.cell_at(self.view.screen_to_world(pointer.pos, map_rect));
        let mut hovered =
            if pointer.on_map() && !pointer.consumed && self.interact.placing.is_none() {
                self.layers.room_at(cell.0, cell.1)
            } else {
                None
            };
        // A text field typing a letter or Backspace must not edit the room under the cursor.
        let blocked = self.settings.open || self.session.renaming || ctx.text_edit_focused();
        if let Some(hit) = hovered {
            self.interact
                .room_shortcuts(&ctx, &mut self.layers, blocked, hit, &mut history);
        }
        if self
            .interact
            .delete_shortcut(&ctx, &mut self.layers, blocked, hovered, &mut history)
            > 0
            && pointer.on_map()
            && !pointer.consumed
            && self.interact.placing.is_none()
        {
            hovered = self.layers.room_at(cell.0, cell.1);
        }
        self.interact
            .update_biome_paint(&pointer, hovered, &mut self.layers, &mut history);
        self.interact
            .update_selection(&pointer, &self.view, &self.layers, &layout, map_rect);
        let mut move_ghosts =
            self.interact
                .update_layer_moving(&pointer, &mut self.layers, cell, &mut history);
        if move_ghosts.is_empty() {
            move_ghosts = self.interact.update_moving(
                &pointer,
                hovered,
                &mut self.layers,
                cell,
                &mut history,
            );
        }
        let select_rect = self.interact.select_rect(pointer.pos);
        self.interact
            .update_icon_drag(&pointer, &legend, &mut self.layers, hovered, &mut history);
        self.interact
            .update_cursor(&ctx, &pointer, rooms, ghost.as_ref(), &move_ghosts, hovered);

        let frame = Frame {
            layout: &layout,
            view: &self.view,
            map_rect,
            hud: &hud,
            legend: &legend,
            hovered: hovered.map(|(_, ri)| ri),
            hovered_layer: hovered.map(|(li, _)| li),
            grid: self.grid,
            layer_frames: self.layer_frames,
            ghost,
            move_ghosts,
            selected: self.interact.selected(),
            select_rect,
            dragged_icon: self.interact.icon_drag.clone(),
            paint_biome: self.interact.paint_biome,
            special: &self.special,
            dark: self.dark,
        };
        render::draw_layers(
            ui,
            &self.layers,
            &self.biomes,
            &frame,
            &mut self.icons,
            &mut self.legend_icons,
        );
    }
}
