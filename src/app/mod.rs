//! Application state and the work done each frame: input, regeneration, camera,
//! hover, room placement and drawing.

mod header;
mod interact;
mod layers_panel;
mod map;
mod session;
mod tools_panel;

use crate::biome::{self, Biome};
use crate::biomes::BiomesTab;
use crate::common::SettingsAction;
use crate::icon_lib::{self, IconDef};
use crate::icons::{self, Icons};
use crate::layer::LayerStack;
use crate::platform::WaylandDrop;
use crate::render::{self, COPIED_TIME};
use crate::settings;
use crate::settings_panel::{self, Settings};
use crate::view::View;
use crate::world::{Params, World, time_seed};
use egui::{Key, TextureHandle, ViewportCommand};
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
    layers: LayerStack,
    seed: u64,
    cells: u32,
    biomes: Vec<Biome>,
}

/// Pushes a snapshot. Drops the oldest entry past `UNDO_LIMIT`.
pub(super) fn remember(
    undo: &mut Vec<UndoEntry>,
    layers: &LayerStack,
    seed: u64,
    cells: u32,
    biomes: &[Biome],
) {
    undo.push(UndoEntry {
        layers: layers.clone(),
        seed,
        cells,
        biomes: biomes.to_vec(),
    });
    if undo.len() > UNDO_LIMIT {
        undo.remove(0);
    }
}

/// Builds a world from the biomes and the settings.
fn build_world(cells: u32, seed: u64, biomes: &[Biome], settings: &Settings) -> World {
    let extras: Vec<_> = biomes.iter().map(Biome::extras).collect();
    World::generate(&Params {
        cells,
        seed,
        extras: &extras,
        min_rooms: settings.min_rooms(),
        max_rooms: settings.max_rooms(),
        sizes: &settings.sizes(),
    })
}

pub struct App {
    cells: u32,
    seed: u64,
    biomes: Vec<Biome>,
    special: Vec<IconDef>,
    settings: Settings,
    layers: LayerStack,
    view: View,
    icons: Icons,
    legend_icons: Icons,
    /// Set when the view must be refit to the map, done once the layout is known.
    fit: bool,
    /// Time of the last seed copy, used to show the "copied" feedback.
    copied_at: f64,
    help_open: bool,
    /// Rooms, biomes, layers and special rooms.
    tools: tools_panel::Tools,
    biomes_tab: BiomesTab,
    grid: bool,
    /// Layer group fill, border and name labels on the map.
    layer_frames: bool,
    session: Session,
    interact: Interaction,
    /// File drops on Wayland, where winit does not report them.
    wayland_drop: Option<WaylandDrop>,
    undo: Vec<UndoEntry>,
    /// Dark UI theme when true.
    dark: bool,
    theme_sun: TextureHandle,
    theme_night: TextureHandle,
    settings_icon: TextureHandle,
    layer_icons: layers_panel::LayerIcons,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> App {
        let loaded = settings::load();
        render::apply_theme(&cc.egui_ctx, loaded.dark);
        let seed = time_seed();
        let biomes = biome::load();
        let special = icon_lib::load();
        let settings = Settings::new(seed, &loaded);
        let world = build_world(START_CELLS, seed, &biomes, &settings);
        let layers = LayerStack::from_world(world);
        let mut icons = Icons::new();
        let mut legend_icons = Icons::new();
        icons.sync(&special);
        legend_icons.sync(&special);
        let theme_sun = icons::texture_from_svg(
            &cc.egui_ctx,
            "theme-sun",
            include_bytes!("../../assets/icons/sun.svg"),
            icons::UI_ICON_PX,
        )
        .expect("sun.svg");
        let theme_night = icons::texture_from_svg(
            &cc.egui_ctx,
            "theme-night",
            include_bytes!("../../assets/icons/night.svg"),
            icons::UI_ICON_PX,
        )
        .expect("night.svg");
        let settings_svg = std::str::from_utf8(include_bytes!("../../assets/icons/settings.svg"))
            .expect("utf8 settings.svg")
            .replace("currentColor", "white");
        let settings_icon = icons::texture_from_svg(
            &cc.egui_ctx,
            "hud-settings",
            settings_svg.as_bytes(),
            icons::UI_ICON_PX,
        )
        .expect("settings.svg");
        let layer_icons = layers_panel::LayerIcons::load(&cc.egui_ctx);
        let mut app = App {
            cells: START_CELLS,
            seed,
            biomes,
            special,
            settings,
            layers,
            view: View::new(),
            icons,
            legend_icons,
            fit: true,
            copied_at: f64::NEG_INFINITY,
            help_open: false,
            tools: tools_panel::Tools::new(),
            biomes_tab: BiomesTab::new(),
            grid: true,
            layer_frames: true,
            session: Session::new(seed.to_string()),
            interact: Interaction::new(),
            wayland_drop: WaylandDrop::start(&cc.egui_ctx, cc),
            undo: Vec::new(),
            dark: loaded.dark,
            theme_sun,
            theme_night,
            settings_icon,
            layer_icons,
        };
        crate::project::register_mime();
        app.open_startup();
        app
    }

    /// Flips the UI theme and writes it to settings.json.
    pub(super) fn toggle_theme(&mut self, ctx: &egui::Context) {
        self.dark = !self.dark;
        self.settings.set_dark(self.dark);
        render::apply_theme(ctx, self.dark);
        let _ = settings::save(&self.settings.settings());
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
            &self.layers,
            self.seed,
            self.cells,
            &self.biomes,
        );
    }

    fn undo_edit(&mut self) {
        let Some(prev) = self.undo.pop() else {
            return;
        };
        self.layers = prev.layers;
        self.cells = prev.cells;
        self.biomes = prev.biomes;
        self.change_seed(prev.seed);
        self.settings.set_seed(prev.seed);
        self.interact.clear();
    }

    /// Rebuilds the active layer from the current seed, cells and biomes.
    fn regen(&mut self) {
        if self.layers.is_active_locked() {
            return;
        }
        self.settings.set_seed(self.seed);
        *self.layers.active_world_mut() =
            build_world(self.cells, self.seed, &self.biomes, &self.settings);
        self.interact.clear();
        self.fit = true;
    }

    fn apply_action(&mut self, action: SettingsAction) {
        if self.layers.is_active_locked() {
            return;
        }
        // Snapshot before the seed changes, so undo restores the previous one.
        self.push_undo();
        match action {
            SettingsAction::UseSeed(seed) => self.change_seed(seed),
            SettingsAction::Regenerate => self.change_seed(time_seed()),
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
            self.settings.open = !self.settings.open;
        }
        if !ctx.text_edit_focused() && ctx.input(|i| i.key_pressed(Key::L)) {
            self.tools.open = true;
            self.tools.tab = tools_panel::ToolsTab::Rooms;
        }
        if !ctx.text_edit_focused() && ctx.input(|i| i.key_pressed(Key::M)) {
            self.tools.open = true;
            self.tools.tab = tools_panel::ToolsTab::Biomes;
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
        // A focused text field keeps Ctrl+Z for its own undo.
        if !ctx.text_edit_focused() && ctx.input(|i| i.modifiers.command && i.key_pressed(Key::Z)) {
            self.undo_edit();
        }
        if escape && self.help_open {
            self.help_open = false;
            escape = false;
        }
        if escape && self.tools.open {
            self.tools.open = false;
            escape = false;
        }
        if escape && self.interact.cancel() {
            escape = false;
        }

        if self.settings.open {
            return;
        }
        if escape {
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }

        if ctx.text_edit_focused() {
            return;
        }
        if ctx.input(|i| i.key_pressed(Key::C)) {
            self.fit = true;
        }
        if ctx.input(|i| i.key_pressed(Key::G)) {
            self.grid = !self.grid;
        }
        if ctx.input(|i| i.key_pressed(Key::F)) {
            self.layer_frames = !self.layer_frames;
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        self.session.take_pending(now);
        self.handle_keys(&ctx, now);

        let mut picker = None;
        if self.tools.open {
            egui::Panel::left("tools")
                .default_size(settings_panel::PANEL_W)
                .min_size(280.0)
                .show(ui, |ui| {
                    picker = self.draw_tools_body(ui);
                });
        }

        if self.settings.open {
            let mut action = None;
            egui::Panel::right("settings")
                .default_size(settings_panel::PANEL_W)
                .min_size(280.0)
                .show(ui, |ui| {
                    action = self.settings.draw(ui);
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
            .frame(egui::Frame::NONE.fill(render::background(self.dark)))
            .show(ui, |ui| {
                self.draw_map_panel(ui, now, picker.as_ref());
            });
        if let Some(notice) = self.session.notice_text(now) {
            render::draw_notice(&ctx, &notice, self.dark);
        }

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
        render::background(self.dark).to_normalized_gamma_f32()
    }
}
