//! Floating Layers window: CRUD, reorder, visibility, lock, opacity.

use super::{UndoEntry, remember};
use crate::biome::Biome;
use crate::layer::LayerStack;
use crate::text::hud as t;
use egui::{Color32, Context, RichText, Sense, vec2};

/// Result of one Layers window frame.
pub struct LayersAction {
    pub changed: bool,
}

/// Floating, movable Layers panel.
pub fn draw_layers_window(
    ctx: &Context,
    open: &mut bool,
    layers: &mut LayerStack,
    undo: &mut Vec<UndoEntry>,
    seed: u64,
    cells: u32,
    biomes: &[Biome],
) -> LayersAction {
    let mut action = LayersAction { changed: false };
    let mut delete_confirm: Option<usize> = None;

    egui::Window::new(t::LAYERS)
        .open(open)
        .collapsible(true)
        .resizable(true)
        .default_pos(egui::pos2(ctx.content_rect().max.x - 440.0, 48.0))
        .default_size(vec2(420.0, 300.0))
        .show(ctx, |ui| {
            let mut reorder: Option<(usize, usize)> = None;
            let mut to_duplicate: Option<usize> = None;
            let mut to_delete: Option<usize> = None;
            let mut add = false;
            let mut set_active: Option<usize> = None;
            let len = layers.len();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for i in 0..len {
                    let is_active = layers.active() == i;
                    ui.horizontal(|ui| {
                        if is_active {
                            let rect = ui.max_rect().expand2(vec2(4.0, 2.0));
                            ui.painter()
                                .rect_filled(rect, 3.0, ui.visuals().selection.bg_fill);
                        }

                        let grip = ui.add(
                            egui::Label::new(RichText::new("=").strong()).sense(Sense::drag()),
                        );
                        if grip.dragged()
                            && let Some(pos) = ui.ctx().pointer_interact_pos()
                        {
                            let row_h = grip.rect.height() + ui.spacing().item_spacing.y;
                            let rel = (pos.y - grip.rect.center().y) / row_h;
                            let target =
                                (i as f32 + rel).round().clamp(0.0, (len - 1) as f32) as usize;
                            if target != i {
                                reorder = Some((i, target));
                            }
                        }

                        {
                            let layer = &mut layers.layers_mut()[i];
                            let mut rgb = layer.color;
                            if ui.color_edit_button_srgb(&mut rgb).changed() {
                                layer.color = rgb;
                                action.changed = true;
                            }

                            let vis = if layer.visible { "Vis" } else { "Hid" };
                            if ui
                                .small_button(vis)
                                .on_hover_text(t::LAYER_VISIBILITY)
                                .clicked()
                            {
                                layer.visible = !layer.visible;
                                action.changed = true;
                            }

                            let lock = if layer.locked { "Lock" } else { "Edit" };
                            if ui
                                .small_button(lock)
                                .on_hover_text(t::LAYER_LOCK)
                                .clicked()
                            {
                                layer.locked = !layer.locked;
                                action.changed = true;
                            }

                            let name_id = ui.make_persistent_id(("layer-name", i));
                            let editing =
                                ui.ctx().data(|d| d.get_temp::<bool>(name_id)).unwrap_or(false);
                            if editing {
                                let response = ui.add(
                                    egui::TextEdit::singleline(&mut layer.name)
                                        .desired_width(90.0)
                                        .id_salt(("layer-edit", i)),
                                );
                                if response.lost_focus() {
                                    ui.ctx().data_mut(|d| d.insert_temp(name_id, false));
                                    if layer.name.trim().is_empty() {
                                        layer.name = format!("Layer {i}");
                                    }
                                    action.changed = true;
                                }
                            } else {
                                let response = ui
                                    .add(egui::Label::new(&layer.name).sense(Sense::click()))
                                    .on_hover_text("Click to rename");
                                if response.clicked() {
                                    ui.ctx().data_mut(|d| d.insert_temp(name_id, true));
                                }
                            }

                            let mut opacity_pct = layer.opacity * 100.0;
                            ui.scope(|ui| {
                                ui.set_max_width(56.0);
                                if ui
                                    .add(
                                        egui::Slider::new(&mut opacity_pct, 5.0..=100.0)
                                            .show_value(false),
                                    )
                                    .changed()
                                {
                                    layer.opacity = opacity_pct / 100.0;
                                    layer.clamp_opacity();
                                    action.changed = true;
                                }
                            });
                        }

                        if ui
                            .small_button("Dup")
                            .on_hover_text(t::LAYER_DUPLICATE)
                            .clicked()
                        {
                            to_duplicate = Some(i);
                        }
                        if i != 0 {
                            let del = egui::Button::new(
                                RichText::new("Del").color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(180, 50, 50));
                            if ui.add(del).on_hover_text(t::LAYER_DELETE).clicked() {
                                to_delete = Some(i);
                            }
                        }

                        if !is_active
                            && ui
                                .small_button("Act")
                                .on_hover_text(t::LAYER_SET_ACTIVE)
                                .clicked()
                        {
                            set_active = Some(i);
                        }
                    });
                }
            });

            ui.separator();
            if ui.button(t::LAYER_ADD).clicked() {
                add = true;
            }

            if let Some((from, to)) = reorder {
                remember(undo, layers, seed, cells, biomes);
                layers.reorder(from, to);
                action.changed = true;
            }
            if let Some(idx) = to_duplicate {
                remember(undo, layers, seed, cells, biomes);
                layers.duplicate(idx);
                action.changed = true;
            }
            if let Some(idx) = set_active {
                layers.set_active(idx);
                action.changed = true;
            }
            if let Some(idx) = to_delete {
                let has_rooms = !layers.layers()[idx].world.rooms.is_empty();
                if has_rooms {
                    delete_confirm = Some(idx);
                } else {
                    remember(undo, layers, seed, cells, biomes);
                    if layers.remove(idx) {
                        action.changed = true;
                    } else {
                        undo.pop();
                    }
                }
            }
            if add {
                remember(undo, layers, seed, cells, biomes);
                layers.add_empty();
                action.changed = true;
            }
        });

    if let Some(idx) = delete_confirm {
        let confirmed = rfd::MessageDialog::new()
            .set_title(t::LAYER_DELETE_TITLE)
            .set_description(t::LAYER_DELETE_CONFIRM)
            .set_buttons(rfd::MessageButtons::YesNo)
            .show()
            == rfd::MessageDialogResult::Yes;
        if confirmed {
            remember(undo, layers, seed, cells, biomes);
            if layers.remove(idx) {
                action.changed = true;
            } else {
                undo.pop();
            }
        }
    }

    action
}
