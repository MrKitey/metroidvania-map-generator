//! Side panel to edit generation settings and biomes.

use crate::biome::{self, Biome};
use crate::settings::{self, DEFAULT_MAX_ROOMS, DEFAULT_MIN_ROOMS, DEFAULT_ROOM_SIZES, DEFAULT_VIEWPORT, Settings};
use macroquad::color::hsl_to_rgb;
use macroquad::prelude::*;
use macroquad::ui::{Skin, root_ui, widgets};

/// Panel width, in screen pixels.
pub const PANEL_W: f32 = 320.0;
/// Color picker size: hue along x, lightness from white to black along y.
const PICKER_SIZE: Vec2 = vec2(280.0, 60.0);
const SWATCH_H: f32 = 12.0;
const TABS: &[&str] = &["Settings", "Biomes"];
const TABS_SIZE: Vec2 = vec2(280.0, 24.0);
/// Base id of the biome tree nodes. Each generation uses its own block of 1000 ids.
const NODE_ID: u64 = 1_000_000;
/// Base id of the room size fields, far from the biome field ids.
const SIZE_ID: u64 = 500_000;
/// Width of a field followed by a button on the same line.
const SIZE_FIELD: Vec2 = vec2(200.0, 19.0);
/// Vertical space above the Settings action buttons, in pixels.
const BUTTONS_GAP: f32 = 12.0;
/// Skin margin of the panel. The window adds a 1 px border, so content sits 5 px
/// from the edges. Also used as the spacing between widgets.
const PANEL_MARGIN: f32 = 4.0;
/// Two buttons share a row. Inner width is the panel minus its 1 px borders. macroquad
/// wraps a widget unless it ends before `inner - 2 * margin`, hence the extra pixel.
const HALF_BUTTON: Vec2 = vec2((PANEL_W - 2.0 - 4.0 * PANEL_MARGIN) / 2.0 - 1.0, 22.0);
/// Upper bound of the minimum, to keep generation fast.
const MAX_MIN_ROOMS: usize = 50;
/// Largest room side, in cells, to keep generation fast.
const MAX_ROOM_SIDE: u32 = 10;

/// Parses `WxH`, with both sides between 1 and `MAX_ROOM_SIDE`.
fn parse_size(text: &str) -> Option<(u32, u32)> {
    let text = text.trim().to_lowercase();
    let (w, h) = text.split_once('x')?;
    let side = |s: &str| s.trim().parse().ok().filter(|v| (1..=MAX_ROOM_SIDE).contains(v));
    Some((side(w)?, side(h)?))
}

/// Fields are kept as raw text so the user can type freely. They are parsed on read,
/// and invalid input falls back to the default value.
pub struct Editor {
    pub open: bool,
    viewport_w: String,
    viewport_h: String,
    min_rooms: String,
    max_rooms: String,
    /// One `WxH` field per allowed room size.
    sizes: Vec<String>,
    /// Shows the current seed, can be edited and applied with "Use seed".
    seed: String,
    /// One `#RRGGBB` field per biome, rebuilt when the biome count changes.
    hex: Vec<String>,
    picker: Texture2D,
    /// Default skin with `PANEL_MARGIN`, applied to the panel only.
    skin: Skin,
    /// Index of the unfolded biome. Only one is open at a time.
    open_biome: Option<usize>,
    /// Tree node state lives in the UI storage, keyed by id. Changing the
    /// generation gives every node a new id, which folds them all.
    generation: u64,
    /// Feedback message shown at the bottom of the panel.
    status: String,
}

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

impl Editor {
    pub fn new(seed: u64, settings: &Settings) -> Editor {
        Editor {
            open: false,
            viewport_w: settings.viewport.0.to_string(),
            viewport_h: settings.viewport.1.to_string(),
            min_rooms: settings.min_rooms.to_string(),
            max_rooms: settings.max_rooms.to_string(),
            sizes: settings.room_sizes.iter().map(|(w, h)| format!("{w}x{h}")).collect(),
            seed: seed.to_string(),
            hex: Vec::new(),
            picker: picker_texture(),
            skin: Skin {
                margin: PANEL_MARGIN,
                ..root_ui().default_skin()
            },
            open_biome: None,
            generation: 0,
            status: String::new(),
        }
    }

    /// 0 means no limit. Never below the minimum.
    pub fn max_rooms(&self) -> usize {
        match self.max_rooms.trim().parse().unwrap_or(DEFAULT_MAX_ROOMS) {
            0 => usize::MAX,
            n => n.max(self.min_rooms()),
        }
    }

    /// Clamped between 1 and `MAX_MIN_ROOMS`.
    pub fn min_rooms(&self) -> usize {
        self.min_rooms
            .trim()
            .parse()
            .unwrap_or(DEFAULT_MIN_ROOMS)
            .clamp(1, MAX_MIN_ROOMS)
    }

    /// Size of one in-game screen in pixels. Zero or invalid values use the default.
    pub fn viewport(&self) -> (u32, u32) {
        let parse = |s: &str, default: u32| s.trim().parse().ok().filter(|&v| v > 0).unwrap_or(default);
        (
            parse(&self.viewport_w, DEFAULT_VIEWPORT.0),
            parse(&self.viewport_h, DEFAULT_VIEWPORT.1),
        )
    }

    /// Valid sizes only. Falls back to the defaults if none is valid.
    pub fn room_sizes(&self) -> Vec<(u32, u32)> {
        let sizes: Vec<(u32, u32)> = self.sizes.iter().filter_map(|s| parse_size(s)).collect();
        if sizes.is_empty() {
            DEFAULT_ROOM_SIZES.to_vec()
        } else {
            sizes
        }
    }

    /// Current field values, validated, in the format saved to disk.
    fn settings(&self) -> Settings {
        Settings {
            viewport: self.viewport(),
            min_rooms: self.min_rooms(),
            max_rooms: self.max_rooms.trim().parse().unwrap_or(DEFAULT_MAX_ROOMS),
            room_sizes: self.room_sizes(),
        }
    }

    /// Called after each regeneration so the field shows the seed in use.
    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed.to_string();
    }

    /// True when the cursor is over the open panel, so map clicks must be ignored.
    pub fn is_mouse_over(&self) -> bool {
        let (x, y) = mouse_position();
        self.open && root_ui().is_mouse_over(vec2(x, y))
    }

    fn fold_all(&mut self) {
        self.open_biome = None;
        self.generation += 1;
    }

    /// Returns the seed to regenerate the world with, or None to keep it.
    pub fn draw(&mut self, biomes: &mut Vec<Biome>) -> Option<u64> {
        let mut regen = false;
        let mut chosen = None;
        // The list cannot shrink while the UI iterates over it, so removal happens afterwards.
        let mut remove = None;
        let mut unfolded = None;
        let pos = vec2(screen_width() - PANEL_W, 0.0);
        let size = vec2(PANEL_W, screen_height());
        if self.hex.len() != biomes.len() {
            self.hex = biomes.iter().map(|b| biome::to_hex(b.fill)).collect();
        }

        root_ui().push_skin(&self.skin);
        // Widget ids must stay unique. Settings use small ids, biomes use 100 + 10 per biome.
        widgets::Window::new(1, pos, size)
            .label("Editor")
            .movable(false)
            .ui(&mut root_ui(), |ui| {
                if ui.tabbar(7, TABS_SIZE, TABS) == 0 {
                    widgets::InputText::new(6).size(SIZE_FIELD).ui(ui, &mut self.seed);
                    ui.same_line(0.0);
                    if ui.button(None, "Use seed") {
                        match self.seed.trim().parse() {
                            Ok(seed) => chosen = Some(seed),
                            Err(_) => self.status = "Invalid seed".to_owned(),
                        }
                    }
                    ui.separator();

                    ui.label(None, "Screen size (px)");
                    ui.input_text(2, "Width", &mut self.viewport_w);
                    ui.input_text(3, "Height", &mut self.viewport_h);
                    ui.separator();

                    ui.label(None, &format!("Min rooms per biome (1 to {MAX_MIN_ROOMS})"));
                    ui.input_text(4, "Minimum", &mut self.min_rooms);
                    ui.label(None, "Max rooms per biome (0 = no limit)");
                    ui.input_text(5, "Maximum", &mut self.max_rooms);
                    ui.separator();

                    let label = format!("Room sizes (WxH, 1 to {MAX_ROOM_SIDE})");
                    widgets::TreeNode::new(8, label.as_str()).ui(ui, |ui| {
                        let mut remove_size = None;
                        for (i, size) in self.sizes.iter_mut().enumerate() {
                            widgets::InputText::new(SIZE_ID + i as u64)
                                .size(SIZE_FIELD)
                                .ui(ui, size);
                            ui.same_line(0.0);
                            if ui.button(None, "Delete") {
                                remove_size = Some(i);
                            }
                        }
                        if let Some(i) = remove_size {
                            self.sizes.remove(i);
                        }
                        if ui.button(None, "Add size") {
                            self.sizes.push("1x1".to_owned());
                        }
                    });
                    ui.separator();
                    ui.canvas().request_space(vec2(0.0, BUTTONS_GAP));

                    if widgets::Button::new("Generate").size(HALF_BUTTON).ui(ui) {
                        regen = true;
                    }
                    ui.same_line(0.0);
                    if widgets::Button::new("Save settings").size(HALF_BUTTON).ui(ui) {
                        self.status = match settings::save(&self.settings()) {
                            Ok(path) => format!("Saved to {}", path.display()),
                            Err(e) => format!("Error: {e}"),
                        };
                    }
                    ui.label(None, &self.status);
                    return;
                }

                for (i, b) in biomes.iter_mut().enumerate() {
                    let node_id = NODE_ID + self.generation * 1000 + i as u64;
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

                    let id = 100 + i as u64 * 10;
                    ui.input_text(id, "Name", &mut b.name);
                    ui.input_text(id + 1, "Color", &mut self.hex[i]);
                    if let Some(color) = biome::parse_hex(&self.hex[i]) {
                        b.fill = color;
                    }

                    let mut canvas = ui.canvas();
                    let pos = canvas.request_space(vec2(PICKER_SIZE.x, SWATCH_H));
                    canvas.rect(Rect::new(pos.x, pos.y, PICKER_SIZE.x, SWATCH_H), None, b.fill);
                    let pos = canvas.request_space(PICKER_SIZE);
                    let rect = Rect::new(pos.x, pos.y, PICKER_SIZE.x, PICKER_SIZE.y);
                    canvas.image(rect, &self.picker);
                    let mouse = Vec2::from(mouse_position());
                    if is_mouse_button_down(MouseButton::Left) && rect.contains(mouse) {
                        b.fill = picker_color((mouse - pos) / PICKER_SIZE);
                        self.hex[i] = biome::to_hex(b.fill);
                    }
                    if ui.button(None, "Delete") {
                        remove = Some(i);
                    }
                    token.end(ui);
                }
                ui.separator();

                if ui.button(None, "Add biome") {
                    biomes.push(Biome::new("New", 128, 128, 128));
                    regen = true;
                }
                if ui.button(None, "Import biomes") {
                    // Blocks the app until the dialog is closed.
                    let picked = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file();
                    if let Some(path) = picked {
                        match biome::read(&path) {
                            Ok(list) => {
                                *biomes = list;
                                // Forces a rebuild even if the biome count is unchanged.
                                self.hex.clear();
                                self.fold_all();
                                regen = true;
                                self.status = format!("Imported {}", path.display());
                            }
                            Err(e) => self.status = format!("Error: {e}"),
                        }
                    }
                }
                if ui.button(None, "Save") {
                    self.status = match biome::save(biomes) {
                        Ok(path) => format!("Saved to {}", path.display()),
                        Err(e) => format!("Error: {e}"),
                    };
                }
                ui.label(None, &self.status);
            });
        root_ui().pop_skin();

        if let Some(i) = unfolded {
            self.fold_all();
            self.open_biome = Some(i);
        }
        // The generator needs at least one biome.
        if let Some(i) = remove {
            if biomes.len() > 1 {
                biomes.remove(i);
                self.fold_all();
                regen = true;
            } else {
                self.status = "At least one biome is required".to_owned();
            }
        }
        if regen {
            chosen = Some(crate::time_seed());
        }
        chosen
    }
}
