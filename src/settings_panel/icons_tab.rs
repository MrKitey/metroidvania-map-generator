//! Special Room tab: CRUD for icon definitions.

use crate::icon_lib::{self, IconDef};
use crate::icons::Icons;
use crate::text::{files, settings as text, status};
use crate::world::World;
use egui::{Color32, Sense};
use std::path::PathBuf;

pub struct IconsTab {
    status: String,
}

impl IconsTab {
    pub fn new() -> IconsTab {
        IconsTab {
            status: String::new(),
        }
    }

    pub fn take_status(&mut self) -> Option<String> {
        if self.status.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.status))
        }
    }

    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        list: &mut Vec<IconDef>,
        world: &mut World,
        icons: &mut Icons,
        legend_icons: &mut Icons,
    ) {
        let mut remove = None;
        let mut changed = false;
        let mut reload_ids = Vec::new();
        let can_delete = list.len() > 1;

        for i in 0..list.len() {
            let id_salt = list[i].id.clone();
            let label = list[i].label.clone();
            ui.push_id(i, |ui| {
                egui::CollapsingHeader::new(label)
                    .id_salt(id_salt)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(text::LABEL);
                            if ui.text_edit_singleline(&mut list[i].label).changed() {
                                changed = true;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(text::FILL);
                            let mut rgb = [list[i].fill.r(), list[i].fill.g(), list[i].fill.b()];
                            if ui.color_edit_button_srgb(&mut rgb).changed() {
                                list[i].fill = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                                changed = true;
                            }
                            ui.label(text::BORDER);
                            let mut rgb =
                                [list[i].border.r(), list[i].border.g(), list[i].border.b()];
                            if ui.color_edit_button_srgb(&mut rgb).changed() {
                                list[i].border = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                                changed = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            if let Some(tex) = icons.get(ui.ctx(), &list[i].id, 32) {
                                ui.image((tex.id(), egui::vec2(32.0, 32.0)));
                            } else {
                                ui.allocate_exact_size(egui::vec2(32.0, 32.0), Sense::hover());
                            }
                            if ui.button(text::CHOOSE_FILE).clicked() {
                                let picked = rfd::FileDialog::new()
                                    .add_filter(files::IMAGE_FILTER, files::IMAGE_EXTS)
                                    .pick_file();
                                if let Some(path) = picked {
                                    match apply_image(&mut list[i], &path) {
                                        Ok(()) => {
                                            reload_ids.push(i);
                                            changed = true;
                                        }
                                        Err(e) => self.status = status::error(e),
                                    }
                                }
                            }
                        });

                        if ui
                            .add_enabled(can_delete, egui::Button::new(text::DELETE))
                            .clicked()
                        {
                            remove = Some(i);
                        }
                    });
            });
        }

        ui.separator();
        if ui.button(text::ADD).clicked() {
            list.push(icon_lib::new_blank(list));
            changed = true;
        }

        if let Some(i) = remove {
            if list.len() > 1 {
                let removed = list.remove(i);
                let valid: Vec<String> = list.iter().map(|d| d.id.clone()).collect();
                world.clear_missing_icons(&valid);
                let _ = std::fs::remove_file(removed.abs_path());
                changed = true;
            } else {
                self.status = status::AT_LEAST_ONE_ICON.to_owned();
            }
        }

        for i in reload_ids {
            icons.reload(&list[i]);
            legend_icons.reload(&list[i]);
        }

        if changed {
            icons.sync(list);
            legend_icons.sync(list);
            self.status = match icon_lib::save(list) {
                Ok(path) => status::saved_to(&path),
                Err(e) => status::error(e),
            };
        }

        if !self.status.is_empty() {
            ui.label(self.status.as_str());
        }
    }

    /// Applies dropped image files when this tab is active.
    /// Returns true when a drop was handled (so the project opener should skip).
    pub fn handle_drops(
        &mut self,
        dropped: &[PathBuf],
        list: &mut Vec<IconDef>,
        icons: &mut Icons,
        legend_icons: &mut Icons,
    ) -> bool {
        let Some(path) = dropped.iter().find(|p| icon_lib::is_image_path(p)) else {
            return false;
        };
        if list.is_empty() {
            list.push(icon_lib::new_blank(list));
        }
        let idx = list.len() - 1;
        match apply_image(&mut list[idx], path) {
            Ok(()) => {
                icons.reload(&list[idx]);
                legend_icons.reload(&list[idx]);
                self.status = match icon_lib::save(list) {
                    Ok(p) => status::saved_to(&p),
                    Err(e) => status::error(e),
                };
            }
            Err(e) => self.status = status::error(e),
        }
        true
    }
}

fn apply_image(def: &mut IconDef, src: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    def.file = icon_lib::import_file(&def.id, src)?;
    Ok(())
}
