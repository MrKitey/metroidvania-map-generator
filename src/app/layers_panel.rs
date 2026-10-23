//! Layers tab: CRUD, reorder, visibility, lock, opacity.

use super::{UndoEntry, remember};
use crate::biome::Biome;
use crate::icons::{self, UI_ICON_PX};
use crate::layer::LayerStack;
use crate::text::hud as t;
use egui::{
    Button, Color32, Context, Image, Response, RichText, Sense, TextureHandle, Ui, Vec2,
    load::SizedTexture, vec2,
};

const ICON_SIZE: f32 = 14.0;

/// UI textures for layer row actions.
pub struct LayerIcons {
    eye: TextureHandle,
    eye_off: TextureHandle,
    lock: TextureHandle,
    lock_open: TextureHandle,
    duplicate: TextureHandle,
    delete: TextureHandle,
}

impl LayerIcons {
    pub fn load(ctx: &Context) -> LayerIcons {
        LayerIcons {
            eye: load_ui_icon(
                ctx,
                "layer-eye",
                include_bytes!("../../assets/icons/eye.svg"),
            ),
            eye_off: load_ui_icon(
                ctx,
                "layer-eye-off",
                include_bytes!("../../assets/icons/eye-off.svg"),
            ),
            lock: load_ui_icon(
                ctx,
                "layer-lock",
                include_bytes!("../../assets/icons/lock.svg"),
            ),
            lock_open: load_ui_icon(
                ctx,
                "layer-lock-open",
                include_bytes!("../../assets/icons/lock-open.svg"),
            ),
            duplicate: load_ui_icon(
                ctx,
                "layer-duplicate",
                include_bytes!("../../assets/icons/duplicate.svg"),
            ),
            delete: load_ui_icon(
                ctx,
                "layer-delete",
                include_bytes!("../../assets/icons/delete.svg"),
            ),
        }
    }
}

/// Rasterizes a Lucide SVG as white so egui tint follows the theme text color.
fn load_ui_icon(ctx: &Context, name: &str, bytes: &[u8]) -> TextureHandle {
    let svg = std::str::from_utf8(bytes)
        .expect("utf8 svg")
        .replace("currentColor", "white");
    icons::texture_from_svg(ctx, name, svg.as_bytes(), UI_ICON_PX).expect(name)
}

fn layer_icon_button(ui: &mut Ui, tex: &TextureHandle, tip: &str) -> Response {
    let image = SizedTexture::new(tex.id(), Vec2::splat(ICON_SIZE));
    let button = Button::image(image).image_tint_follows_text_color(true);
    ui.add(button).on_hover_text(tip)
}

fn layer_delete_button(ui: &mut Ui, tex: &TextureHandle) -> Response {
    let image = Image::new(SizedTexture::new(tex.id(), Vec2::splat(ICON_SIZE)));
    let button = Button::image(image).fill(Color32::from_rgb(180, 50, 50));
    ui.add(button).on_hover_text(t::LAYER_DELETE)
}

/// Result of one Layers tab frame.
pub struct LayersAction {
    pub changed: bool,
}

/// Layers list inside the Tools panel.
pub fn draw_layers(
    ui: &mut Ui,
    layers: &mut LayerStack,
    undo: &mut Vec<UndoEntry>,
    seed: u64,
    cells: u32,
    biomes: &[Biome],
    icons: &LayerIcons,
) -> LayersAction {
    let mut action = LayersAction { changed: false };
    let mut delete_confirm: Option<usize> = None;

    {
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

                    let grip =
                        ui.add(egui::Label::new(RichText::new("=").strong()).sense(Sense::drag()));
                    if grip.dragged()
                        && let Some(pos) = ui.ctx().pointer_interact_pos()
                    {
                        let row_h = grip.rect.height() + ui.spacing().item_spacing.y;
                        let rel = (pos.y - grip.rect.center().y) / row_h;
                        let target = (i as f32 + rel).round().clamp(0.0, (len - 1) as f32) as usize;
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

                        let vis = if layer.visible {
                            &icons.eye
                        } else {
                            &icons.eye_off
                        };
                        if layer_icon_button(ui, vis, t::LAYER_VISIBILITY).clicked() {
                            layer.visible = !layer.visible;
                            action.changed = true;
                        }

                        let lock = if layer.locked {
                            &icons.lock
                        } else {
                            &icons.lock_open
                        };
                        if layer_icon_button(ui, lock, t::LAYER_LOCK).clicked() {
                            layer.locked = !layer.locked;
                            action.changed = true;
                        }

                        let name_id = ui.make_persistent_id(("layer-name", i));
                        let editing = ui
                            .ctx()
                            .data(|d| d.get_temp::<bool>(name_id))
                            .unwrap_or(false);
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

                    if layer_icon_button(ui, &icons.duplicate, t::LAYER_DUPLICATE).clicked() {
                        to_duplicate = Some(i);
                    }
                    if i != 0 && layer_delete_button(ui, &icons.delete).clicked() {
                        to_delete = Some(i);
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
    }

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
