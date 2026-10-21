//! Map panel: camera, pointer for this frame, and the draw call.

use super::App;
use super::interact::Pointer;
use crate::layout::Layout;
use crate::render::{self, Frame, Hud, RoomLegend, picker_layout};
use egui::{PointerButton, Pos2, Sense};

impl App {
    pub(super) fn draw_map_panel(&mut self, ui: &mut egui::Ui, now: f64) {
        let ctx = ui.ctx().clone();
        let (response, _) = ui.allocate_painter(ui.available_size(), Sense::hover());
        let map_rect = response.rect;
        let layout = Layout::new(self.editor.viewport());
        if self.fit {
            self.view.fit(&self.world, &layout, map_rect);
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
        let hud = Hud::new(&ctx, self.seed, copied, map_rect);
        let legend = RoomLegend::new(&ctx, &self.world, &self.biomes, map_rect);
        let mut picker_ui = self
            .interact
            .picker
            .map(|a| picker_layout(&ctx, &self.editor.sizes(), &layout, a, map_rect));
        let over_picker = picker_ui
            .as_ref()
            .is_some_and(|p| p.rect.contains(pointer_pos));
        let mut pointer = Pointer {
            pos: pointer_pos,
            left: ctx.input(|i| i.pointer.button_pressed(PointerButton::Primary)),
            left_down: ctx.input(|i| i.pointer.button_down(PointerButton::Primary)),
            right: ctx.input(|i| i.pointer.button_pressed(PointerButton::Secondary)),
            over_seed: !over_picker && hud.seed.contains(pointer_pos),
            over_picker,
            over_legend: !over_picker && legend.contains(pointer_pos),
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
            self.interact.select_biome(&self.world, biome);
        }

        let ghost = self.interact.update_placing(
            &mut self.world,
            &self.view,
            &mut pointer,
            &layout,
            map_rect,
        );
        self.interact.update_picker(&mut pointer, &mut picker_ui);

        let mut hovered = if pointer.on_map()
            && !pointer.consumed
            && self.interact.picker.is_none()
            && self.interact.placing.is_none()
        {
            let (x, y) = layout.cell_at(self.view.screen_to_world(pointer.pos, map_rect));
            self.world.room_at(x, y)
        } else {
            None
        };
        let blocked = self.editor.open || self.session.renaming;
        if let Some(i) = hovered {
            self.interact
                .room_shortcuts(&ctx, &mut self.world, blocked, i);
        }
        if let Some(removed) =
            self.interact
                .delete_shortcut(&ctx, &mut self.world, blocked, hovered)
        {
            hovered = match hovered {
                Some(i) if i == removed => None,
                Some(i) if i > removed => Some(i - 1),
                other => other,
            };
        }
        self.interact
            .update_selection(&pointer, &self.view, &self.world, &layout, map_rect);
        let move_ghosts = self.interact.update_moving(
            &pointer,
            hovered,
            &mut self.world,
            &self.view,
            &layout,
            map_rect,
        );
        let select_rect = self.interact.select_rect(pointer.pos);
        self.interact
            .update_icon_drag(&pointer, &legend, &mut self.world, hovered);
        self.interact.update_cursor(
            &ctx,
            &pointer,
            picker_ui.as_ref(),
            ghost.as_ref(),
            &move_ghosts,
            hovered,
        );

        let frame = Frame {
            layout: &layout,
            view: &self.view,
            map_rect,
            hud: &hud,
            legend: &legend,
            hovered,
            grid: self.grid,
            picker: picker_ui.as_ref(),
            ghost,
            move_ghosts,
            selected: self.interact.selected(),
            select_rect,
            dragged_icon: self.interact.icon_drag,
        };
        render::draw_world(
            ui,
            &self.world,
            &self.biomes,
            &frame,
            &mut self.icons,
            &mut self.legend_icons,
        );
    }
}
