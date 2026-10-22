//! Application state and the work done each frame: input, regeneration, camera,
//! hover, room placement and drawing.

mod header;
mod interact;
mod map;
mod session;

use crate::biome::{self, Biome};
use crate::biomes::BiomesTab;
use crate::common::EditorAction;
use crate::editor::{self, Editor};
use crate::icons::Icons;
use crate::layout::Layout;
use crate::platform::WaylandDrop;
use crate::render::{self, COPIED_TIME};
use crate::settings;
use crate::text;
use crate::view::View;
use crate::world::{Params, World, time_seed};
use egui::{Color32, Key, ViewportCommand};
use header::HEADER_H;
use interact::Interaction;
use session::Session;
use std::time::Duration;

/// Map size limits, as a number of room cells.
const MIN_CELLS: u32 = 96;
const MAX_CELLS: u32 = 5120;
const START_CELLS: u32 = 768;
const UNDO_LIMIT: usize = 50;

/// Snapshot of the map and the values regen may change.
pub(super) struct UndoEntry {
    world: World,
    seed: u64,
    cells: u32,
    biomes: Vec<Biome>,
}

/// Pushes a snapshot. Drops the oldest entry past `UNDO_LIMIT`.
pub(super) fn remember(
    undo: &mut Vec<UndoEntry>,
    world: &World,
    seed: u64,
    cells: u32,
    biomes: &[Biome],
) {
    undo.push(UndoEntry {
        world: world.clone(),
        seed,
        cells,
        biomes: biomes.to_vec(),
    });
    if undo.len() > UNDO_LIMIT {
        undo.remove(0);
    }
}

/// Builds a world from the biomes and the editor settings.
fn build_world(cells: u32, seed: u64, biomes: &[Biome], editor: &Editor) -> World {
    let extras: Vec<_> = biomes.iter().map(Biome::extras).collect();
    World::generate(&Params {
        cells,
        seed,
        extras: &extras,
        min_rooms: editor.min_rooms(),
        max_rooms: editor.max_rooms(),
        sizes: &editor.sizes(),
    })
}

pub struct App {
    cells: u32,
    seed: u64,
    biomes: Vec<Biome>,
    editor: Editor,
    world: World,
    view: View,
    icons: Icons,
    legend_icons: Icons,
    /// Set when the view must be refit to the map, done once the layout is known.
    fit: bool,
    /// Time of the last seed copy, used to show the "copied" feedback.
    copied_at: f64,
    help_open: bool,
    /// Room list panel on the left.
    rooms_open: bool,
    /// Biomes panel.
    biomes_open: bool,
    biomes_tab: BiomesTab,
    grid: bool,
    session: Session,
    interact: Interaction,
    /// File drops on Wayland, where winit does not report them.
    wayland_drop: Option<WaylandDrop>,
    undo: Vec<UndoEntry>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> App {
        render::apply_theme(&cc.egui_ctx);
        let seed = time_seed();
        let biomes = biome::load();
        let editor = Editor::new(seed, &settings::load());
        let world = build_world(START_CELLS, seed, &biomes, &editor);
        let mut app = App {
            cells: START_CELLS,
            seed,
            biomes,
            editor,
            world,
            view: View::new(),
            icons: Icons::new(),
            legend_icons: Icons::new(),
            fit: true,
            copied_at: f64::NEG_INFINITY,
            help_open: false,
            rooms_open: false,
            biomes_open: false,
            biomes_tab: BiomesTab::new(),
            grid: true,
            session: Session::new(seed.to_string()),
            interact: Interaction::new(),
            wayland_drop: WaylandDrop::start(&cc.egui_ctx, cc),
            undo: Vec::new(),
        };
        crate::project::register_mime();
        app.open_startup();
        app
    }

    /// Keeps a project name that is still the previous seed in step with the new one.
    fn change_seed(&mut self, seed: u64) {
        if self.session.name == self.seed.to_string() {
            self.session.name = seed.to_string();
        }
        self.seed = seed;
    }

    fn push_undo(&mut self) {
        remember(
            &mut self.undo,
            &self.world,
            self.seed,
            self.cells,
            &self.biomes,
        );
    }

    fn undo_edit(&mut self) {
        let Some(prev) = self.undo.pop() else {
            return;
        };
        self.world = prev.world;
        self.cells = prev.cells;
        self.biomes = prev.biomes;
        self.change_seed(prev.seed);
        self.editor.set_seed(prev.seed);
        self.interact.clear();
    }

    /// Rebuilds the map from the current seed, cells and biomes.
    fn regen(&mut self) {
        self.editor.set_seed(self.seed);
        self.world = build_world(self.cells, self.seed, &self.biomes, &self.editor);
        self.interact.clear();
        self.fit = true;
    }

    fn apply_action(&mut self, action: EditorAction) {
        // Snapshot before the seed changes, so undo restores the previous one.
        self.push_undo();
        match action {
            EditorAction::UseSeed(seed) => self.change_seed(seed),
            EditorAction::Regenerate => self.change_seed(time_seed()),
        }
        self.regen();
    }

    /// Keyboard shortcuts. Returns after closing overlays, or quits the window.
    /// While the project name is being typed, only Escape is handled. It cancels the edit.
    fn handle_keys(&mut self, ctx: &egui::Context, now: f64) {
        if self.session.renaming {
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.session.cancel_rename();
            }
            return;
        }

        if ctx.input(|i| i.key_pressed(Key::Tab)) {
            self.editor.open = !self.editor.open;
        }
        if !ctx.text_edit_focused() && ctx.input(|i| i.key_pressed(Key::L)) {
            self.rooms_open = !self.rooms_open;
        }
        if !ctx.text_edit_focused() && ctx.input(|i| i.key_pressed(Key::M)) {
            self.biomes_open = !self.biomes_open;
        }

        let mut escape = ctx.input(|i| i.key_pressed(Key::Escape));
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::N)) {
            self.new_project(now);
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::O)) {
            self.load_project(now);
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::S)) {
            self.save_project(now);
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::Z)) {
            self.undo_edit();
        }
        if escape && self.help_open {
            self.help_open = false;
            escape = false;
        }
        if escape && self.interact.cancel() {
            escape = false;
        }

        if self.editor.open {
            return;
        }
        if escape {
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }

        if ctx.input(|i| i.key_pressed(Key::C)) {
            self.fit = true;
        }
        if ctx.input(|i| i.key_pressed(Key::G)) {
            self.grid = !self.grid;
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        self.session.take_pending(now);
        self.handle_keys(&ctx, now);

        if self.editor.open {
            let mut action = None;
            egui::Panel::right("editor")
                .default_size(editor::PANEL_W)
                .min_size(280.0)
                .show(ui, |ui| {
                    action = self.editor.draw(ui);
                });
            if let Some(action) = action {
                self.apply_action(action);
            }
        }

        let mut room_list = None;
        if self.rooms_open {
            let sizes = self.editor.sizes();
            let layout = Layout::new(self.editor.viewport());
            let mut started = None;
            let mut sizes_changed = false;
            egui::Panel::left("rooms")
                .default_size(editor::PANEL_W)
                .min_size(280.0)
                .show(ui, |ui| {
                    ui.heading(text::rooms::ROOMS);
                    ui.separator();
                    let (rooms, bosses, corridors) = self.editor.size_lists_mut();
                    let (picker, drag, changed) =
                        render::draw_room_list(ui, &sizes, &layout, rooms, bosses, corridors);
                    sizes_changed = changed;
                    started = drag;
                    room_list = Some(picker);
                });
            if sizes_changed {
                self.editor.save_settings();
            }
            if let Some((choice, size)) = started {
                self.interact.placing = Some(choice);
                self.interact.placing_size = size;
            }
        }

        if self.biomes_open {
            let mut action = None;
            egui::Panel::right("biomes")
                .default_size(editor::PANEL_W)
                .min_size(280.0)
                .show(ui, |ui| {
                    ui.heading(text::hud::BIOMES);
                    ui.separator();
                    action = self.biomes_tab.draw(ui, &mut self.biomes, &mut self.world);
                });
            if let Some(action) = action {
                self.apply_action(action);
            }
        }

        egui::Panel::top("header")
            .exact_size(HEADER_H)
            .resizable(false)
            .show_separator_line(false)
            .show(ui, |ui| {
                self.draw_header(ui, now);
            });

        if self.help_open {
            render::draw_help(&ctx, &mut self.help_open);
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(render::BACKGROUND))
            .show(ui, |ui| {
                self.draw_map_panel(ui, now, room_list.as_ref());
            });

        self.handle_dropped(&ctx, now);
        let wayland_paths = self
            .wayland_drop
            .as_mut()
            .map(|drop| drop.poll())
            .unwrap_or_default();
        self.open_dropped(wayland_paths, now);
        self.sync_dirty(now);
        self.autosave(&ctx, now);

        if now - self.copied_at < COPIED_TIME || self.session.notice_visible(now) {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Color32::from_rgb(15, 15, 22).to_normalized_gamma_f32()
    }
}
