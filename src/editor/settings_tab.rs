//! Settings tab: seed, screen size, room counts and allowed room sizes.

use super::EditorAction;
use super::fields::{Fields, MAX_MIN_ROOMS, MAX_ROOM_SIDE};
use super::widgets::size_list;
use crate::settings::{self, DEFAULT_BOSS_SIZES, DEFAULT_CORRIDOR_SIZES, DEFAULT_ROOM_SIZES};
use crate::text::{editor as text, status};

pub fn draw(
    ui: &mut egui::Ui,
    fields: &mut Fields,
    seed: &mut String,
    status: &mut String,
) -> Option<EditorAction> {
    let mut action = None;
    let mut changed = false;

    ui.horizontal(|ui| {
        if ui
            .add(egui::TextEdit::singleline(seed).desired_width(160.0))
            .changed()
        {
            changed = true;
        }
        if ui.button(text::USE_SEED).clicked() {
            match seed.trim().parse() {
                Ok(s) => {
                    action = Some(EditorAction::UseSeed(s));
                    changed = true;
                }
                Err(_) => *status = status::INVALID_SEED.to_owned(),
            }
        }
    });
    ui.separator();

    ui.label(text::SCREEN_SIZE);
    ui.horizontal(|ui| {
        ui.label(text::WIDTH);
        if ui
            .add(egui::TextEdit::singleline(&mut fields.viewport_w).desired_width(80.0))
            .changed()
        {
            changed = true;
        }
        ui.label(text::HEIGHT);
        if ui
            .add(egui::TextEdit::singleline(&mut fields.viewport_h).desired_width(80.0))
            .changed()
        {
            changed = true;
        }
    });
    ui.separator();

    ui.label(text::min_rooms_label(MAX_MIN_ROOMS));
    ui.horizontal(|ui| {
        ui.label(text::MINIMUM);
        if ui
            .add(egui::TextEdit::singleline(&mut fields.min_rooms).desired_width(80.0))
            .changed()
        {
            changed = true;
        }
    });
    ui.label(text::MAX_ROOMS_LABEL);
    ui.horizontal(|ui| {
        ui.label(text::MAXIMUM);
        if ui
            .add(egui::TextEdit::singleline(&mut fields.max_rooms).desired_width(80.0))
            .changed()
        {
            changed = true;
        }
    });
    ui.separator();

    ui.collapsing(text::room_sizes_label(MAX_ROOM_SIDE), |ui| {
        if size_list(ui, &mut fields.sizes, DEFAULT_ROOM_SIZES) {
            changed = true;
        }
    });
    ui.collapsing(text::boss_sizes_label(MAX_ROOM_SIDE), |ui| {
        if size_list(ui, &mut fields.boss_sizes, DEFAULT_BOSS_SIZES) {
            changed = true;
        }
    });
    ui.collapsing(text::corridor_sizes_label(MAX_ROOM_SIDE), |ui| {
        if size_list(ui, &mut fields.corridor_sizes, DEFAULT_CORRIDOR_SIZES) {
            changed = true;
        }
    });
    ui.separator();

    if ui.checkbox(&mut fields.autosave, text::AUTOSAVE).changed() {
        changed = true;
    }
    ui.horizontal(|ui| {
        ui.label(text::AUTOSAVE_DELAY);
        if ui
            .add(egui::TextEdit::singleline(&mut fields.autosave_secs).desired_width(80.0))
            .changed()
        {
            changed = true;
        }
    });
    ui.separator();

    ui.horizontal(|ui| {
        // Generating wins over a seed typed in the same frame.
        if ui.button(text::GENERATE).clicked() {
            action = Some(EditorAction::Regenerate);
        }
    });
    if !status.is_empty() {
        ui.label(status.as_str());
    }
    if changed {
        *status = match settings::save(&fields.settings()) {
            Ok(path) => status::saved_to(&path),
            Err(e) => status::error(e),
        };
    }
    action
}
