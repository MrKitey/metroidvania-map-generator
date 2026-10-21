//! Small widgets shared by the editor tabs.

use super::fields::{SizeFields, size_fields};
use crate::text::editor as text;

/// Default panel width, in screen points.
pub const PANEL_W: f32 = 320.0;

/// Editable list of room sizes: width and height fields with a delete button per row,
/// then "Add size" and "Reset". Returns true if anything changed.
pub fn size_list(ui: &mut egui::Ui, fields: &mut SizeFields, defaults: &[(u32, u32)]) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(text::WIDTH);
        ui.label(text::HEIGHT);
    });
    let mut remove = None;
    for (i, (w, h)) in fields.iter_mut().enumerate() {
        ui.push_id(i, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add(egui::TextEdit::singleline(w).desired_width(80.0))
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .add(egui::TextEdit::singleline(h).desired_width(80.0))
                    .changed()
                {
                    changed = true;
                }
                if ui.button(text::DELETE).clicked() {
                    remove = Some(i);
                    changed = true;
                }
            });
        });
    }
    if let Some(i) = remove {
        fields.remove(i);
    }
    ui.horizontal(|ui| {
        if ui.button(text::ADD_SIZE).clicked() {
            fields.push(("1".to_owned(), "1".to_owned()));
            changed = true;
        }
        if ui.button(text::RESET).clicked() {
            *fields = size_fields(defaults);
            changed = true;
        }
    });
    changed
}
