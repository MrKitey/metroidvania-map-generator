//! Metroidvania map generator.
//!
//! Generates a random map of rooms grouped by biome, shows it with pan and zoom,
//! and lets the user mark boss, save, secret and portal rooms with the keyboard.

// No console window behind the app in Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod biome;
mod editor;
mod export;
mod icons;
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
use world::{Marker, RoomKind, World};

/// Map size limits, as a number of room cells.
const MIN_CELLS: u32 = 96;
const MAX_CELLS: u32 = 5120;
/// Cells added or removed per arrow key press.
const STEP: u32 = 64;
/// Seconds the export result stays in the header.
const NOTICE_TIME: f64 = 4.0;

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
        &biomes.iter().map(|b| b.extras()).collect::<Vec<_>>(),
        editor.min_rooms(),
        editor.max_rooms(),
        editor.sizes(),
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
    let mut icons = icons::Icons::new();
    let mut legend_icons = icons::Icons::new();
    // Set when the view must be refit to the map, done once the layout is known.
    let mut fit = true;
    // Time of the last seed copy, used to show the "copied" feedback.
    let mut copied_at = f64::NEG_INFINITY;
    let mut help_open = false;
    // Last export result and when it happened, shown in the header for a while.
    let mut notice: Option<(String, f64)> = None;

    // One iteration per frame: input, optional regeneration, camera, hover, drawing.
    loop {
        if is_key_pressed(KeyCode::Tab) {
            editor.open = !editor.open;
        }

        // Escape closes the shortcuts popup first, and only quits when it is closed.
        let mut escape = is_key_pressed(KeyCode::Escape);
        if escape && help_open {
            help_open = false;
            escape = false;
        }

        let mut regen = false;
        // Shortcuts are disabled while the editor is open so typing in text fields is safe.
        if !editor.open {
            if escape {
                break;
            }
            if is_key_pressed(KeyCode::R) {
                seed = time_seed();
                regen = true;
            }
            if is_key_pressed(KeyCode::C) {
                fit = true;
            }
            if (is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::Up)) && cells < MAX_CELLS
            {
                cells += STEP;
                regen = true;
            }
            if (is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::Down)) && cells > MIN_CELLS
            {
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
        let over_button = render::help_button_rect(reserved).contains(mouse);
        let over_help =
            over_button || (help_open && render::help_popup_rect(reserved).contains(mouse));
        // A click outside the open popup only closes it, it does not reach the map.
        let clicked = is_mouse_button_pressed(MouseButton::Left)
            || is_mouse_button_pressed(MouseButton::Right);
        let mut consumed = false;
        if over_button && is_mouse_button_pressed(MouseButton::Left) {
            help_open = !help_open;
        } else if help_open && clicked && !over_help {
            help_open = false;
            consumed = true;
        }

        if render::export_button_rect(reserved).contains(mouse)
            && is_mouse_button_pressed(MouseButton::Left)
        {
            // Blocks the app until the dialog is closed.
            let picked = rfd::FileDialog::new()
                .add_filter("Zip", &["zip"])
                .set_file_name(format!("map-{seed}.zip"))
                .save_file();
            if let Some(mut path) = picked {
                if path.extension().is_none() {
                    path.set_extension("zip");
                }
                let settings = editor.settings();
                let map = export::MapData {
                    seed,
                    cells,
                    settings: &settings,
                    world: &world,
                    biomes: &biomes,
                    layout: &layout,
                };
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let text = match export::write_zip(&path, &map, &mut icons) {
                    Ok(()) => format!("Exported {name}"),
                    Err(e) => format!("Export failed: {e}"),
                };
                notice = Some((text, get_time()));
            }
        }
        let notice_text = notice
            .as_ref()
            .filter(|(_, at)| get_time() - at < NOTICE_TIME)
            .map(|(text, _)| text.as_str());

        let hovered = if over_ui || over_seed || over_help || consumed || mouse.y < render::TOP {
            None
        } else {
            let (x, y) = layout.cell_at(camera.screen_to_world(mouse));
            world.room_at(x, y)
        };

        // Room shortcuts act on the room under the cursor.
        if let (Some(i), false) = (hovered, editor.open) {
            if is_key_pressed(KeyCode::B) {
                world.toggle_marker(i, Marker::Boss);
            }
            if is_key_pressed(KeyCode::S) {
                world.toggle_marker(i, Marker::Save);
            }
            if is_key_pressed(KeyCode::H) {
                world.toggle_kind(i, RoomKind::Secret);
            }
            if is_key_pressed(KeyCode::P) {
                world.toggle_kind(i, RoomKind::Portal);
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
            help_open,
            notice: notice_text,
        };
        render::draw_world(&world, &biomes, &frame, &mut icons, &mut legend_icons);
        next_frame().await;
    }
}
