//! Biomes tab: one folding entry per biome, plus add, import and save.

use super::EditorAction;
use super::widgets::{THIRD_BUTTON, count_slider, wrapped_label};
use crate::biome::{self, Biome, MIN_CORRIDORS};
use crate::color::{parse_hex, to_hex};
use macroquad::color::hsl_to_rgb;
use macroquad::prelude::*;
use macroquad::ui::{Ui, widgets};

/// Color picker size: hue along x, lightness from white to black along y.
const PICKER_SIZE: Vec2 = vec2(280.0, 60.0);
const SWATCH_H: f32 = 12.0;
/// Base id of the biome fields. Each biome uses a block of `FIELD_STRIDE` ids.
const FIELD_ID: u64 = 100;
const FIELD_STRIDE: u64 = 10;
/// Base id of the biome tree nodes. Each generation uses its own block of 1000 ids.
const NODE_ID: u64 = 1_000_000;
const NODE_BLOCK: u64 = 1000;

/// Picker color at `t`, a position normalized to 0..1 on both axes.
fn picker_color(t: Vec2) -> Color {
    hsl_to_rgb(t.x, 1.0, 1.0 - t.y)
}

fn picker_texture() -> Texture2D {
    let (w, h) = (PICKER_SIZE.x as u16, PICKER_SIZE.y as u16);
    let mut image = Image::gen_image_color(w, h, BLACK);
    for x in 0..w as u32 {
        for y in 0..h as u32 {
            image.set_pixel(x, y, picker_color(vec2(x as f32, y as f32) / PICKER_SIZE));
        }
    }
    Texture2D::from_image(&image)
}

pub struct BiomesTab {
    /// One `#RRGGBB` field per biome, rebuilt when the biome count changes.
    hex: Vec<String>,
    picker: Texture2D,
    /// Index of the unfolded biome. Only one is open at a time.
    open_biome: Option<usize>,
    /// Tree node state lives in the UI storage, keyed by id. Changing the
    /// generation gives every node a new id, which folds them all.
    generation: u64,
}

impl BiomesTab {
    pub fn new() -> BiomesTab {
        BiomesTab {
            hex: Vec::new(),
            picker: picker_texture(),
            open_biome: None,
            generation: 0,
        }
    }

    /// Rebuilds the color fields if the biome count changed.
    pub fn sync(&mut self, biomes: &[Biome]) {
        if self.hex.len() != biomes.len() {
            self.hex = biomes.iter().map(|b| to_hex(b.fill)).collect();
        }
    }

    fn fold_all(&mut self) {
        self.open_biome = None;
        self.generation += 1;
    }

    /// Returns `Regenerate` when the biome list changed.
    pub fn draw(
        &mut self,
        ui: &mut Ui,
        biomes: &mut Vec<Biome>,
        status: &mut String,
    ) -> Option<EditorAction> {
        let mut action = None;
        // The list cannot shrink while the loop iterates over it, so removal happens after.
        let mut remove = None;
        let mut unfolded = None;

        for (i, b) in biomes.iter_mut().enumerate() {
            let node_id = NODE_ID + self.generation * NODE_BLOCK + i as u64;
            let mut node = widgets::TreeNode::new(node_id, b.name.as_str());
            if self.open_biome == Some(i) {
                node = node.init_unfolded();
            }
            let Some(token) = node.begin(ui) else {
                if self.open_biome == Some(i) {
                    self.open_biome = None;
                }
                continue;
            };
            if self.open_biome != Some(i) {
                unfolded = Some(i);
            }

            let id = FIELD_ID + i as u64 * FIELD_STRIDE;
            ui.input_text(id, "Name", &mut b.name);
            ui.input_text(id + 1, "Color", &mut self.hex[i]);
            if let Some(color) = parse_hex(&self.hex[i]) {
                b.fill = color;
            }

            let mut canvas = ui.canvas();
            let pos = canvas.request_space(vec2(PICKER_SIZE.x, SWATCH_H));
            canvas.rect(
                Rect::new(pos.x, pos.y, PICKER_SIZE.x, SWATCH_H),
                None,
                b.fill,
            );
            let pos = canvas.request_space(PICKER_SIZE);
            let rect = Rect::new(pos.x, pos.y, PICKER_SIZE.x, PICKER_SIZE.y);
            canvas.image(rect, &self.picker);
            let mouse = Vec2::from(mouse_position());
            if is_mouse_button_down(MouseButton::Left) && rect.contains(mouse) {
                b.fill = picker_color((mouse - pos) / PICKER_SIZE);
                self.hex[i] = to_hex(b.fill);
            }
            count_slider(ui, id + 2, "Secret rooms", 0, &mut b.secrets);
            count_slider(ui, id + 3, "Boss rooms", 0, &mut b.bosses);
            count_slider(ui, id + 4, "Save rooms", 0, &mut b.saves);
            count_slider(ui, id + 5, "Portal rooms", 0, &mut b.portals);
            count_slider(ui, id + 6, "Corridors", MIN_CORRIDORS, &mut b.corridors);
            if ui.button(None, "Delete") {
                remove = Some(i);
            }
            token.end(ui);
        }
        ui.separator();

        if widgets::Button::new("Add").size(THIRD_BUTTON).ui(ui) {
            biomes.push(Biome::new("New", 128, 128, 128));
            action = Some(EditorAction::Regenerate);
        }
        ui.same_line(0.0);
        if widgets::Button::new("Import").size(THIRD_BUTTON).ui(ui) {
            // Blocks the app until the dialog is closed.
            let picked = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .pick_file();
            if let Some(path) = picked {
                match biome::read(&path) {
                    Ok(list) => {
                        *biomes = list;
                        // Forces a rebuild even if the biome count is unchanged.
                        self.hex.clear();
                        self.fold_all();
                        action = Some(EditorAction::Regenerate);
                        *status = format!("Imported {}", path.display());
                    }
                    Err(e) => *status = format!("Error: {e}"),
                }
            }
        }
        ui.same_line(0.0);
        if widgets::Button::new("Save").size(THIRD_BUTTON).ui(ui) {
            *status = match biome::save(biomes) {
                Ok(path) => format!("Saved to {}", path.display()),
                Err(e) => format!("Error: {e}"),
            };
        }
        wrapped_label(ui, status);

        if let Some(i) = unfolded {
            self.fold_all();
            self.open_biome = Some(i);
        }
        if let Some(i) = remove {
            // The generator needs at least one biome.
            if biomes.len() > 1 {
                biomes.remove(i);
                self.fold_all();
                action = Some(EditorAction::Regenerate);
            } else {
                *status = "At least one biome is required".to_owned();
            }
        }
        action
    }
}
