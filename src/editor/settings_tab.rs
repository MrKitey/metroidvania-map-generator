//! Settings tab: seed, screen size, room counts and allowed room sizes.

use super::EditorAction;
use super::fields::{Fields, MAX_MIN_ROOMS, MAX_ROOM_SIDE};
use super::widgets::{HALF_BUTTON, size_list, wrapped_label};
use crate::settings::{self, DEFAULT_BOSS_SIZES, DEFAULT_CORRIDOR_SIZES, DEFAULT_ROOM_SIZES};
use crate::text::{editor as text, status};
use macroquad::prelude::*;
use macroquad::ui::{Ui, widgets};

/// Widget ids of the tab.
const VIEWPORT_W_ID: u64 = 2;
const VIEWPORT_H_ID: u64 = 3;
const MIN_ROOMS_ID: u64 = 4;
const MAX_ROOMS_ID: u64 = 5;
const SEED_ID: u64 = 6;
const ROOM_SIZES_NODE_ID: u64 = 8;
const BOSS_SIZES_NODE_ID: u64 = 9;
const CORRIDOR_SIZES_NODE_ID: u64 = 10;
/// Base ids of the size fields, far from the biome field ids.
const ROOM_SIZES_ID: u64 = 500_000;
const BOSS_SIZES_ID: u64 = 600_000;
const CORRIDOR_SIZES_ID: u64 = 700_000;

/// Width of a field followed by a button on the same line.
const SEED_FIELD: Vec2 = vec2(200.0, 19.0);
/// Vertical space above the action buttons, in pixels.
const BUTTONS_GAP: f32 = 12.0;

pub fn draw(
    ui: &mut Ui,
    fields: &mut Fields,
    seed: &mut String,
    status: &mut String,
) -> Option<EditorAction> {
    let mut action = None;

    widgets::InputText::new(SEED_ID)
        .size(SEED_FIELD)
        .ui(ui, seed);
    ui.same_line(0.0);
    if ui.button(None, text::USE_SEED) {
        match seed.trim().parse() {
            Ok(seed) => action = Some(EditorAction::UseSeed(seed)),
            Err(_) => *status = status::INVALID_SEED.to_owned(),
        }
    }
    ui.separator();

    ui.label(None, text::SCREEN_SIZE);
    ui.input_text(VIEWPORT_W_ID, text::WIDTH, &mut fields.viewport_w);
    ui.input_text(VIEWPORT_H_ID, text::HEIGHT, &mut fields.viewport_h);
    ui.separator();

    ui.label(None, &text::min_rooms_label(MAX_MIN_ROOMS));
    ui.input_text(MIN_ROOMS_ID, text::MINIMUM, &mut fields.min_rooms);
    ui.label(None, text::MAX_ROOMS_LABEL);
    ui.input_text(MAX_ROOMS_ID, text::MAXIMUM, &mut fields.max_rooms);
    ui.separator();

    let label = text::room_sizes_label(MAX_ROOM_SIDE);
    widgets::TreeNode::new(ROOM_SIZES_NODE_ID, label.as_str()).ui(ui, |ui| {
        size_list(ui, &mut fields.sizes, ROOM_SIZES_ID, DEFAULT_ROOM_SIZES);
    });
    let label = text::boss_sizes_label(MAX_ROOM_SIDE);
    widgets::TreeNode::new(BOSS_SIZES_NODE_ID, label.as_str()).ui(ui, |ui| {
        size_list(
            ui,
            &mut fields.boss_sizes,
            BOSS_SIZES_ID,
            DEFAULT_BOSS_SIZES,
        );
    });
    let label = text::corridor_sizes_label(MAX_ROOM_SIDE);
    widgets::TreeNode::new(CORRIDOR_SIZES_NODE_ID, label.as_str()).ui(ui, |ui| {
        size_list(
            ui,
            &mut fields.corridor_sizes,
            CORRIDOR_SIZES_ID,
            DEFAULT_CORRIDOR_SIZES,
        );
    });
    ui.separator();
    ui.canvas().request_space(vec2(0.0, BUTTONS_GAP));

    // Generating wins over a seed typed in the same frame.
    if widgets::Button::new(text::GENERATE)
        .size(HALF_BUTTON)
        .ui(ui)
    {
        action = Some(EditorAction::Regenerate);
    }
    ui.same_line(0.0);
    if widgets::Button::new(text::SAVE_SETTINGS)
        .size(HALF_BUTTON)
        .ui(ui)
    {
        *status = match settings::save(&fields.settings()) {
            Ok(path) => status::saved_to(&path),
            Err(e) => status::error(e),
        };
    }
    wrapped_label(ui, status);
    action
}
