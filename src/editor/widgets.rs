//! Sizes and small widgets shared by the editor tabs.
//!
//! macroquad widget ids must be unique in the panel. Each tab keeps its ids close to
//! its code, and the ranges below keep them apart:
//! 1 to 10 for the window, tabs and settings fields, 100 and up for the biome fields,
//! 500_000 and up for the size fields, 1_000_000 and up for the biome tree nodes.

use super::fields::{SizeFields, size_fields};
use crate::biome::MAX_COUNT;
use crate::text::editor as text;
use macroquad::prelude::*;
use macroquad::ui::{Ui, widgets};

/// Panel width, in screen pixels.
pub const PANEL_W: f32 = 320.0;
/// Skin margin of the panel. The window adds a 1 px border, so content sits 5 px
/// from the edges. Also used as the spacing between widgets.
pub const PANEL_MARGIN: f32 = 4.0;
/// Full inner width: the panel minus its 1 px borders and the margin on each side.
pub const INNER_W: f32 = PANEL_W - 2.0 - 2.0 * PANEL_MARGIN;
pub const HALF_BUTTON: Vec2 = row_button(2);
pub const THIRD_BUTTON: Vec2 = row_button(3);
/// Room size column: one width or height field, and its header above.
const SIDE_FIELD: Vec2 = vec2(100.0, 19.0);

/// Size of each of `n` equal buttons filling a row. Inner width is the panel minus its
/// 1 px borders. macroquad wraps a widget unless it ends before `inner - 2 * margin`,
/// hence the extra pixel.
const fn row_button(n: u32) -> Vec2 {
    let n = n as f32;
    vec2((PANEL_W - 2.0 - (n + 2.0) * PANEL_MARGIN) / n - 1.0, 22.0)
}

/// Shows `text` as labels, broken into lines that fit the panel. Breaks between
/// characters, since paths have few spaces.
pub fn wrapped_label(ui: &mut Ui, text: &str) {
    let mut line = String::new();
    for c in text.chars() {
        line.push(c);
        if line.chars().count() > 1 && ui.calc_size(&line).x > INNER_W {
            line.pop();
            ui.label(None, &line);
            line = c.to_string();
        }
    }
    if !line.is_empty() {
        ui.label(None, &line);
    }
}

/// Slider for a count between `min` and `MAX_COUNT`.
/// The slider works in floats, rounding makes it step between integers.
pub fn count_slider(ui: &mut Ui, id: u64, label: &str, min: u32, value: &mut u32) {
    let mut v = *value as f32;
    ui.slider(id, label, min as f32..MAX_COUNT as f32, &mut v);
    *value = (v.round() as u32).clamp(min, MAX_COUNT);
}

/// Editable list of room sizes: width and height fields with a delete button per row,
/// then "Add size" and "Reset". `id` is the base id of the fields.
pub fn size_list(ui: &mut Ui, fields: &mut SizeFields, id: u64, defaults: &[(u32, u32)]) {
    widgets::Label::new(text::WIDTH).size(SIDE_FIELD).ui(ui);
    ui.same_line(0.0);
    widgets::Label::new(text::HEIGHT).size(SIDE_FIELD).ui(ui);
    let mut remove = None;
    for (i, (w, h)) in fields.iter_mut().enumerate() {
        let id = id + 2 * i as u64;
        widgets::InputText::new(id).size(SIDE_FIELD).ui(ui, w);
        ui.same_line(0.0);
        widgets::InputText::new(id + 1).size(SIDE_FIELD).ui(ui, h);
        ui.same_line(0.0);
        if ui.button(None, text::DELETE) {
            remove = Some(i);
        }
    }
    if let Some(i) = remove {
        fields.remove(i);
    }
    if ui.button(None, text::ADD_SIZE) {
        fields.push(("1".to_owned(), "1".to_owned()));
    }
    ui.same_line(0.0);
    if ui.button(None, text::RESET) {
        *fields = size_fields(defaults);
    }
}
