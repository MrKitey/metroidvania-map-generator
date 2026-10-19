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
use render::{Choice, Frame, Ghost, Layout};
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
    // Top-left corner of the open room picker, and the room chosen in it, being placed.
    let mut picker: Option<Vec2> = None;
    let mut placing: Option<Choice> = None;
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
        // Then the room picker and the room being placed.
        if escape && (picker.is_some() || placing.is_some()) {
            picker = None;
            placing = None;
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
        // The picker is drawn over the header buttons and the seed, so it gets the clicks first.
        let mut picker_ui = picker.map(|a| render::picker_layout(&editor.sizes(), &layout, a));
        let over_picker = picker_ui.as_ref().is_some_and(|p| p.rect.contains(mouse));
        let over_seed = !over_picker && render::seed_rect(seed, copied, reserved).contains(mouse);
        if over_seed && is_mouse_button_pressed(MouseButton::Left) {
            miniquad::window::clipboard_set(&seed.to_string());
            copied_at = get_time();
        }
        let over_button = !over_picker && render::help_button_rect(reserved).contains(mouse);
        let over_help = over_button
            || (!over_picker && help_open && render::help_popup_rect(reserved).contains(mouse));
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

        if !over_picker
            && render::export_button_rect(reserved).contains(mouse)
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

        // True when the cursor is over the map itself, not over a panel, button or popup.
        let on_map = !(over_ui || over_seed || over_help || over_picker || mouse.y < render::TOP);
        let left = is_mouse_button_pressed(MouseButton::Left);
        let right = is_mouse_button_pressed(MouseButton::Right);

        // Room being placed. Its position snaps to the cell grid, centered on the cursor
        // cell. This runs before the picker, so the click that picks a size does not place it.
        let mut ghost = None;
        if let Some(choice) = placing {
            if on_map && !consumed {
                let (cx, cy) = layout.cell_at(camera.screen_to_world(mouse));
                let (x, y) = (cx - (choice.w / 2) as i32, cy - (choice.h / 2) as i32);
                let free = world.can_place(x, y, choice.w, choice.h);
                if left && free {
                    let (kind, marker) = choice.pick.spec();
                    world.place_manual((x, y), (choice.w, choice.h), kind, marker);
                    placing = None;
                } else {
                    ghost = Some(Ghost { x, y, choice, free });
                }
            }
            if right {
                placing = None;
                ghost = None;
                consumed = true;
            }
        }

        // Picker clicks: a size starts the placement, a click elsewhere closes the popup.
        if let Some(p) = &picker_ui
            && (left || right)
        {
            let hit = p.items.iter().find(|(_, r)| r.contains(mouse));
            if let (true, Some((choice, _))) = (left, hit) {
                placing = Some(*choice);
                picker = None;
            } else if !over_picker {
                picker = None;
                consumed = true;
            }
        }
        // Right click on the map opens the picker at the cursor.
        if picker.is_none() {
            picker_ui = None;
            if right && on_map && !consumed && placing.is_none() {
                picker = Some(mouse);
            }
        }
        if let (Some(a), None) = (picker, &picker_ui) {
            picker_ui = Some(render::picker_layout(&editor.sizes(), &layout, a));
        }

        let hovered = if on_map && !consumed && picker.is_none() && placing.is_none() {
            let (x, y) = layout.cell_at(camera.screen_to_world(mouse));
            world.room_at(x, y)
        } else {
            None
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
            picker: picker_ui.as_ref(),
            ghost,
            notice: notice_text,
        };
        render::draw_world(&world, &biomes, &frame, &mut icons, &mut legend_icons);
        next_frame().await;
    }
}
