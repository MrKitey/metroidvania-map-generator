//! Metroidvania map generator.
//!
//! Generates a random map of rooms grouped by biome, shows it with pan and zoom,
//! and lets the user mark boss and save rooms.

mod biome;
mod editor;
mod render;
mod settings;
mod storage;
mod view;
mod world;

use editor::Editor;
use macroquad::prelude::*;
use render::{Frame, Layout};
use std::time::{SystemTime, UNIX_EPOCH};
use view::View;
use world::{Marker, World};

/// Map size limits, as a number of room cells.
const MIN_CELLS: u32 = 96;
const MAX_CELLS: u32 = 5120;
/// Cells added or removed per arrow key press.
const STEP: u32 = 64;

fn window_conf() -> Conf {
    Conf {
        window_title: "Metroidvania Map Generator".to_owned(),
        window_width: 1280,
        window_height: 800,
        ..Default::default()
    }
}

/// New seed from the clock, so each regeneration gives a different map.
fn time_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1)
}

/// Builds a world from the editor settings.
fn generate(cells: u32, seed: u64, biomes: &[biome::Biome], editor: &Editor) -> World {
    World::generate(
        cells,
        seed,
        biomes.len(),
        editor.min_rooms(),
        editor.max_rooms(),
        &editor.room_sizes(),
    )
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut cells = 768;
    let mut seed = time_seed();
    let mut biomes = biome::load();
    let mut editor = Editor::new(seed, &settings::load());
    let mut world = generate(cells, seed, &biomes, &editor);
    let mut view = View::new();
    // Set when the view must be refit to the map, done once the layout is known.
    let mut fit = true;
    // Time of the last seed copy, used to show the "copied" feedback.
    let mut copied_at = f64::NEG_INFINITY;

    // One iteration per frame: input, optional regeneration, camera, hover, drawing.
    loop {
        if is_key_pressed(KeyCode::Tab) {
            editor.open = !editor.open;
        }

        let mut regen = false;
        // Shortcuts are disabled while the editor is open so typing in text fields is safe.
        if !editor.open {
            if is_key_pressed(KeyCode::Escape) {
                break;
            }
            if is_key_pressed(KeyCode::R) {
                seed = time_seed();
                regen = true;
            }
            if is_key_pressed(KeyCode::C) {
                fit = true;
            }
            if (is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::Up)) && cells < MAX_CELLS {
                cells += STEP;
                regen = true;
            }
            if (is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::Down)) && cells > MIN_CELLS {
                cells -= STEP;
                regen = true;
            }
        } else if let Some(s) = editor.draw(&mut biomes) {
            seed = s;
            regen = true;
        }

        if regen {
            editor.set_seed(seed);
            world = generate(cells, seed, &biomes, &editor);
            fit = true;
        }

        // Screen width taken by the editor panel on the right.
        let reserved = if editor.open { editor::PANEL_W } else { 0.0 };
        let layout = Layout::new(editor.viewport());
        if fit {
            view.fit(&world, &layout, reserved);
            fit = false;
        }

        let over_ui = editor.is_mouse_over();
        view.update(reserved, !over_ui);
        let camera = view.camera(reserved);
        // Room under the cursor: screen point to world point to cell to room.
        let mouse = Vec2::from(mouse_position());
        let copied = get_time() - copied_at < render::COPIED_TIME;
        let over_seed = render::seed_rect(seed, copied, reserved).contains(mouse);
        if over_seed && is_mouse_button_pressed(MouseButton::Left) {
            miniquad::window::clipboard_set(&seed.to_string());
            copied_at = get_time();
        }
        let hovered = if over_ui || over_seed || mouse.y < render::TOP {
            None
        } else {
            let (x, y) = layout.cell_at(camera.screen_to_world(mouse));
            world.room_at(x, y)
        };

        if let Some(i) = hovered {
            if is_mouse_button_pressed(MouseButton::Left) {
                world.toggle_marker(i, Marker::Boss);
            }
            if is_mouse_button_pressed(MouseButton::Right) {
                world.toggle_marker(i, Marker::Save);
            }
        }

        let frame = Frame {
            layout: &layout,
            view: &view,
            camera: &camera,
            seed,
            copied,
            reserved,
            hovered,
        };
        render::draw_world(&world, &biomes, &frame);
        next_frame().await;
    }
}
