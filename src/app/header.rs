//! Header bar: project actions, the name, and the recent-files menu.

use super::App;
use crate::common::EditorAction;
use crate::export::{self, MapData};
use crate::layout::Layout;
use crate::project;
use crate::storage;
use crate::text;
use egui::{Button, Color32, Rect, Sense, Vec2, load::SizedTexture};

pub(super) const HEADER_H: f32 = 40.0;

impl App {
    pub(super) fn draw_header(&mut self, ui: &mut egui::Ui, now: f64) {
        let header = ui.max_rect();
        let notice = self.session.notice_text(now);
        ui.horizontal_centered(|ui| {
            self.recent_menu(ui, now);
            if ui.button(text::hud::NEW).clicked() {
                self.new_project(now);
            }
            if ui.button(text::hud::LOAD).clicked() {
                self.load_project(now);
            }
            if ui.button(text::hud::SAVE).clicked() {
                self.save_project(now);
            }
            ui.label(text::hud::rooms(self.world.rooms.len()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.theme_button(ui);
                if ui.button(text::hud::HELP_BUTTON).clicked() {
                    self.help_open = !self.help_open;
                }
                if ui.button(text::hud::EXPORT).clicked() {
                    self.export(now);
                }
                if ui.button(text::hud::REGENERATE).clicked() {
                    self.apply_action(EditorAction::Regenerate);
                }
                if ui.button(text::hud::EDITOR).clicked() {
                    self.editor.open = !self.editor.open;
                }
                if ui.button(text::hud::BIOMES).clicked() {
                    self.biomes_open = !self.biomes_open;
                }
                if ui.button(text::rooms::ROOMS).clicked() {
                    self.rooms_open = !self.rooms_open;
                }
                if let Some(notice) = &notice {
                    ui.label(notice);
                }
            });
        });
        self.draw_project_name(ui, header);
    }

    fn draw_project_name(&mut self, ui: &mut egui::Ui, header: Rect) {
        let extra = if self.session.dirty { 28.0 } else { 12.0 };
        let font = egui::TextStyle::Body.resolve(ui.style());
        let text_width = ui.ctx().fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(self.session.name.clone(), font, Color32::WHITE)
                .size()
                .x
        });
        // The closed name stays as wide as its text. The field stays wide while typing.
        let edit_width = (header.width() * 0.45).clamp(280.0, 480.0);
        let width = if self.session.renaming {
            edit_width
        } else {
            (text_width + extra).clamp(80.0, edit_width)
        };
        let name_rect = Rect::from_center_size(header.center(), egui::vec2(width, header.height()));
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("project-name")
                .max_rect(name_rect),
        );
        child.horizontal_centered(|ui| {
            if self.session.renaming {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.session.name)
                        .desired_width(width - 16.0)
                        .align(egui::Align2::CENTER_CENTER),
                );
                if self.session.rename_focus {
                    response.request_focus();
                    self.session.rename_focus = false;
                } else if response.lost_focus() {
                    self.session.finish_rename(self.seed);
                }
            } else {
                let response = ui.add(
                    egui::Label::new(egui::RichText::new(&self.session.name).strong())
                        .sense(Sense::click()),
                );
                if response.clicked() {
                    self.session.start_rename();
                }
                if self.session.dirty {
                    ui.label(egui::RichText::new("*").strong());
                }
            }
        });
    }

    fn theme_button(&mut self, ui: &mut egui::Ui) {
        let (tex, tip) = if self.dark {
            (&self.theme_sun, text::hud::THEME_TO_LIGHT)
        } else {
            (&self.theme_night, text::hud::THEME_TO_DARK)
        };
        let image = SizedTexture::new(tex.id(), Vec2::splat(16.0));
        let button = Button::image(image).image_tint_follows_text_color(true);
        if ui.add(button).on_hover_text(tip).clicked() {
            self.toggle_theme(ui.ctx());
        }
    }

    fn recent_menu(&mut self, ui: &mut egui::Ui, now: f64) {
        ui.menu_button(text::hud::RECENT, |ui| {
            let entries = project::read_recent(&storage::app_dir());
            if entries.is_empty() {
                ui.label(text::hud::NO_RECENT);
                return;
            }
            for entry in entries {
                let label = entry.label();
                let full = entry.path.display().to_string();
                if ui.button(label).on_hover_text(full).clicked() {
                    ui.close();
                    self.load_path(entry.path, now, true);
                }
            }
        });
    }

    pub(super) fn export(&mut self, now: f64) {
        let settings = self.editor.settings();
        let layout = Layout::new(self.editor.viewport());
        let map = MapData {
            seed: self.seed,
            cells: self.cells,
            settings: &settings,
            world: &self.world,
            biomes: &self.biomes,
            special: &self.special,
            layout: &layout,
        };
        if let Some(text) = export::save_with_dialog(&map) {
            self.session.set_notice(text, now);
        }
    }
}
