//! Application state and the work done each frame: input, regeneration, camera,
//! hover, room placement and drawing.

use crate::biome::{self, Biome};
use crate::editor::{self, Editor, EditorAction};
use crate::export::{self, MapData};
use crate::icons::{IconId, Icons};
use crate::layout::Layout;
use crate::project::{self, Loaded, ProjectFile};
use crate::render::{self, COPIED_TIME, Frame, Ghost, Hud, Picker, RoomLegend, picker_layout};
use crate::settings;
use crate::storage;
use crate::text;
use crate::view::View;
use crate::wayland_drop::WaylandDrop;
use crate::world::{Choice, Marker, Params, Pick, RoomKind, World, time_seed};
use egui::{Color32, CursorIcon, Event, Key, PointerButton, Pos2, Rect, Sense, ViewportCommand};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Map size limits, as a number of room cells.
const MIN_CELLS: u32 = 96;
const MAX_CELLS: u32 = 5120;
const START_CELLS: u32 = 768;
/// Pointer travel below this, in points, is a click and clears the selection.
const SELECT_CLICK: f32 = 4.0;
/// Seconds the export result stays in the header.
const NOTICE_TIME: f64 = 4.0;
const HEADER_H: f32 = 40.0;

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

/// Mouse state of one frame: where it is, what it is over, and the clicks.
struct Pointer {
    pos: Pos2,
    left: bool,
    left_down: bool,
    right: bool,
    over_seed: bool,
    over_picker: bool,
    over_legend: bool,
    on_map_rect: bool,
    /// Set when a popup used the click, so it must not reach the map.
    consumed: bool,
}

impl Pointer {
    /// True when the cursor is over the map itself, not over a panel, button or popup.
    fn on_map(&self) -> bool {
        self.on_map_rect && !(self.over_seed || self.over_picker || self.over_legend)
    }
}

/// Rooms being dragged with the left button. They all share one cell offset.
struct Moving {
    rooms: Vec<usize>,
    /// Top-left cell of each room when the drag started.
    origins: Vec<(i32, i32)>,
    /// Cell under the cursor when the drag started.
    anchor: (i32, i32),
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
    grid: bool,
    panning: bool,
    /// Top-left corner of the open room picker.
    picker: Option<Pos2>,
    /// Room chosen in the picker, being placed.
    placing: Option<Choice>,
    /// Placed rooms being dragged, if any.
    moving: Option<Moving>,
    /// Rooms chosen by the selection rectangle.
    selected: Vec<usize>,
    /// Screen position where the selection rectangle started.
    selecting: Option<Pos2>,
    /// Icon of the room type legend being dragged onto a room, if any.
    icon_drag: Option<IconId>,
    /// Last export result and when it happened, shown in the header for a while.
    notice: Option<(String, f64)>,
    /// Shown on the first frame, once the clock is known.
    pending_notice: Option<String>,
    project_name: String,
    path: Option<PathBuf>,
    dirty: bool,
    /// Time of the last change. The autosave delay is counted from here.
    dirty_at: f64,
    renaming: bool,
    rename_focus: bool,
    name_before: String,
    /// File last written or loaded. None until the first save or load.
    saved: Option<ProjectFile>,
    /// Snapshot from the previous check, so a quiet frame does not reset the delay.
    last: Option<ProjectFile>,
    /// File drops on Wayland, where winit does not report them.
    wayland_drop: Option<WaylandDrop>,
}

/// Most recent project that is still on disk. Missing entries are dropped.
fn latest_project() -> Option<PathBuf> {
    let dir = storage::app_dir();
    for path in project::read_recent(&dir) {
        if path.is_file() {
            return Some(path);
        }
        let _ = project::forget(&dir, &path);
    }
    None
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
            grid: true,
            panning: false,
            picker: None,
            placing: None,
            moving: None,
            selected: Vec::new(),
            selecting: None,
            icon_drag: None,
            notice: None,
            pending_notice: None,
            project_name: seed.to_string(),
            path: None,
            dirty: true,
            dirty_at: 0.0,
            renaming: false,
            rename_focus: false,
            name_before: String::new(),
            saved: None,
            last: None,
            wayland_drop: WaylandDrop::start(&cc.egui_ctx, cc),
        };
        project::register_mime();
        app.open_startup();
        app
    }

    /// Keeps a project name that is still the previous seed in step with the new one.
    fn change_seed(&mut self, seed: u64) {
        if self.project_name == self.seed.to_string() {
            self.project_name = seed.to_string();
        }
        self.seed = seed;
    }

    fn regen(&mut self) {
        self.editor.set_seed(self.seed);
        self.world = build_world(self.cells, self.seed, &self.biomes, &self.editor);
        self.clear_edit_state();
        self.fit = true;
    }

    fn apply_action(&mut self, action: EditorAction) {
        match action {
            EditorAction::UseSeed(seed) => {
                self.change_seed(seed);
                self.regen();
            }
            EditorAction::Regenerate => {
                self.change_seed(time_seed());
                self.regen();
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        if let Some(text) = self.pending_notice.take() {
            self.notice = Some((text, now));
        }
        self.handle_keys(&ctx, now);

        if self.editor.open {
            let mut action = None;
            egui::Panel::right("editor")
                .default_size(editor::PANEL_W)
                .min_size(280.0)
                .show(ui, |ui| {
                    action = self.editor.draw(ui, &mut self.biomes);
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
                self.draw_map_panel(ui, now);
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

        if now - self.copied_at < COPIED_TIME
            || self
                .notice
                .as_ref()
                .is_some_and(|(_, at)| now - at < NOTICE_TIME)
        {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Color32::from_rgb(15, 15, 22).to_normalized_gamma_f32()
    }
}

impl App {
    fn draw_header(&mut self, ui: &mut egui::Ui, now: f64) {
        let header = ui.max_rect();
        let notice = self
            .notice
            .as_ref()
            .filter(|(_, at)| now - at < NOTICE_TIME)
            .map(|(text, _)| text.clone());
        ui.horizontal_centered(|ui| {
            self.recent_menu(ui, now);
            if ui.button(text::hud::NEW).clicked() {
                self.new_project(now);
            }
            if ui.button(text::hud::LOAD).clicked() {
                self.load_project(now);
            }
            if ui.button(text::hud::SAVE).clicked() {
                self.save_project(now);
            }
            ui.label(text::hud::rooms(self.world.rooms.len()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(text::hud::HELP_BUTTON).clicked() {
                    self.help_open = !self.help_open;
                }
                if ui.button(text::hud::EXPORT).clicked() {
                    self.export(now);
                }
                if let Some(notice) = &notice {
                    ui.label(notice);
                }
            });
        });
        self.draw_project_name(ui, header);
    }

    fn draw_map_panel(&mut self, ui: &mut egui::Ui, now: f64) {
        let ctx = ui.ctx().clone();
        let (response, _) = ui.allocate_painter(ui.available_size(), Sense::hover());
        let map_rect = response.rect;
        let layout = Layout::new(self.editor.viewport());
        if self.fit {
            self.view.fit(&self.world, &layout, map_rect);
            self.fit = false;
        }

        let pointer_pos = ctx.input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::ZERO);
        if response.contains_pointer() {
            let wheel = ctx.input(|i| i.smooth_scroll_delta.y);
            if wheel != 0.0 {
                self.view.zoom_at(wheel, pointer_pos, map_rect);
            }
        }
        if ctx.input(|i| i.pointer.button_pressed(PointerButton::Middle))
            && response.contains_pointer()
        {
            self.panning = true;
        }
        if ctx.input(|i| !i.pointer.button_down(PointerButton::Middle)) {
            self.panning = false;
        }
        if self.panning {
            self.view.pan(ctx.input(|i| i.pointer.delta()));
        }

        let copied = now - self.copied_at < COPIED_TIME;
        let hud = Hud::new(&ctx, self.seed, copied, map_rect);
        let legend = RoomLegend::new(&ctx, &self.world, &self.biomes, map_rect);
        let mut picker_ui = self
            .picker
            .map(|a| picker_layout(&ctx, &self.editor.sizes(), &layout, a, map_rect));
        let over_picker = picker_ui
            .as_ref()
            .is_some_and(|p| p.rect.contains(pointer_pos));
        let mut pointer = Pointer {
            pos: pointer_pos,
            left: ctx.input(|i| i.pointer.button_pressed(PointerButton::Primary)),
            left_down: ctx.input(|i| i.pointer.button_down(PointerButton::Primary)),
            right: ctx.input(|i| i.pointer.button_pressed(PointerButton::Secondary)),
            over_seed: !over_picker && hud.seed.contains(pointer_pos),
            over_picker,
            over_legend: !over_picker && legend.contains(pointer_pos),
            on_map_rect: response.contains_pointer(),
            consumed: false,
        };

        if pointer.over_seed && pointer.left {
            ctx.copy_text(self.seed.to_string());
            self.copied_at = now;
        }
        if pointer.left
            && !pointer.consumed
            && let Some(biome) = legend.biome_at(pointer.pos)
        {
            self.selected = self
                .world
                .rooms
                .iter()
                .enumerate()
                .filter(|(_, room)| room.biome == biome)
                .map(|(i, _)| i)
                .collect();
            self.moving = None;
            self.selecting = None;
        }

        let ghost = self.update_placing(&mut pointer, &layout, map_rect);
        self.update_picker(&mut pointer, &mut picker_ui);

        let mut hovered = if pointer.on_map()
            && !pointer.consumed
            && self.picker.is_none()
            && self.placing.is_none()
        {
            let (x, y) = layout.cell_at(self.view.screen_to_world(pointer.pos, map_rect));
            self.world.room_at(x, y)
        } else {
            None
        };
        if let Some(i) = hovered {
            self.room_shortcuts(&ctx, i);
        }
        if let Some(removed) = self.delete_shortcut(&ctx, hovered) {
            hovered = match hovered {
                Some(i) if i == removed => None,
                Some(i) if i > removed => Some(i - 1),
                other => other,
            };
        }
        self.update_selection(&pointer, &layout, map_rect);
        let move_ghosts = self.update_moving(&pointer, hovered, &layout, map_rect);
        let select_rect = self
            .selecting
            .map(|start| Rect::from_two_pos(start, pointer.pos));
        self.update_icon_drag(&pointer, &legend, hovered);
        self.update_cursor(
            &ctx,
            &pointer,
            picker_ui.as_ref(),
            ghost.as_ref(),
            &move_ghosts,
            hovered,
        );

        let frame = Frame {
            layout: &layout,
            view: &self.view,
            map_rect,
            hud: &hud,
            legend: &legend,
            hovered,
            grid: self.grid,
            picker: picker_ui.as_ref(),
            ghost,
            move_ghosts,
            selected: &self.selected,
            select_rect,
            dragged_icon: self.icon_drag,
        };
        render::draw_world(
            ui,
            &self.world,
            &self.biomes,
            &frame,
            &mut self.icons,
            &mut self.legend_icons,
        );
    }

    /// Keyboard shortcuts. Returns after closing overlays, or quits the window.
    /// While the project name is being typed, only Escape is handled. It cancels the edit.
    fn handle_keys(&mut self, ctx: &egui::Context, now: f64) {
        if self.renaming {
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.project_name = self.name_before.clone();
                self.renaming = false;
            }
            return;
        }

        if ctx.input(|i| i.key_pressed(Key::Tab)) {
            self.editor.open = !self.editor.open;
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
        if escape && self.help_open {
            self.help_open = false;
            escape = false;
        }
        if escape
            && (self.picker.is_some()
                || self.placing.is_some()
                || self.moving.is_some()
                || self.selecting.is_some()
                || !self.selected.is_empty()
                || self.icon_drag.is_some())
        {
            self.picker = None;
            self.placing = None;
            self.moving = None;
            self.selecting = None;
            self.selected.clear();
            self.icon_drag = None;
            escape = false;
        }

        if self.editor.open {
            return;
        }
        if escape {
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }

        let mut regen = false;
        if ctx.input(|i| i.key_pressed(Key::R)) {
            self.change_seed(time_seed());
            regen = true;
        }
        if ctx.input(|i| i.key_pressed(Key::C)) {
            self.fit = true;
        }
        if ctx.input(|i| i.key_pressed(Key::G)) {
            self.grid = !self.grid;
        }
        if regen {
            self.regen();
        }
    }

    fn draw_project_name(&mut self, ui: &mut egui::Ui, header: Rect) {
        let extra = if self.dirty { 28.0 } else { 12.0 };
        let font = egui::TextStyle::Body.resolve(ui.style());
        let text_width = ui.ctx().fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(self.project_name.clone(), font, Color32::WHITE)
                .size()
                .x
        });
        // The closed name stays as wide as its text. The field stays wide while typing.
        let edit_width = (header.width() * 0.45).clamp(280.0, 480.0);
        let width = if self.renaming {
            edit_width
        } else {
            (text_width + extra).clamp(80.0, edit_width)
        };
        let name_rect = Rect::from_center_size(header.center(), egui::vec2(width, header.height()));
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("project-name")
                .max_rect(name_rect),
        );
        child.horizontal_centered(|ui| {
            if self.renaming {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.project_name)
                        .desired_width(width - 16.0)
                        .align(egui::Align2::CENTER_CENTER),
                );
                if self.rename_focus {
                    response.request_focus();
                    self.rename_focus = false;
                } else if response.lost_focus() {
                    self.finish_rename();
                }
            } else {
                let response = ui.add(
                    egui::Label::new(egui::RichText::new(&self.project_name).strong())
                        .sense(Sense::click()),
                );
                if response.clicked() {
                    self.name_before = self.project_name.clone();
                    self.renaming = true;
                    self.rename_focus = true;
                }
                if self.dirty {
                    ui.label(egui::RichText::new("*").strong());
                }
            }
        });
    }

    fn finish_rename(&mut self) {
        self.renaming = false;
        let trimmed = self.project_name.trim().to_owned();
        self.project_name = if trimmed.is_empty() {
            self.seed.to_string()
        } else {
            trimmed
        };
    }

    fn recent_menu(&mut self, ui: &mut egui::Ui, now: f64) {
        ui.menu_button(text::hud::RECENT, |ui| {
            let paths = project::read_recent(&storage::app_dir());
            if paths.is_empty() {
                ui.label(text::hud::NO_RECENT);
                return;
            }
            for path in paths {
                let label = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string());
                let full = path.display().to_string();
                if ui.button(label).on_hover_text(full).clicked() {
                    ui.close();
                    self.load_path(path, now, true);
                }
            }
        });
    }

    fn open_startup(&mut self) {
        let path = project::startup_path().or_else(|| latest_project());
        let Some(path) = path else {
            return;
        };
        match project::read(&path) {
            Ok(loaded) => {
                self.pending_notice = Some(text::status::loaded_from(&path));
                self.apply_loaded(loaded, Some(path));
            }
            Err(err) => self.pending_notice = Some(text::status::error(err)),
        }
    }

    fn new_project(&mut self, now: f64) {
        if !self.confirm_discard() {
            return;
        }
        self.seed = time_seed();
        self.project_name = self.seed.to_string();
        self.cells = START_CELLS;
        self.path = None;
        self.editor.set_seed(self.seed);
        self.world = build_world(self.cells, self.seed, &self.biomes, &self.editor);
        self.clear_edit_state();
        self.fit = true;
        self.renaming = false;
        self.saved = None;
        self.last = None;
        self.dirty = true;
        self.dirty_at = now;
    }

    fn load_project(&mut self, now: f64) {
        if !self.confirm_discard() {
            return;
        }
        let Some(path) = project::load_dialog() else {
            return;
        };
        self.load_path(path, now, false);
    }

    fn save_project(&mut self, now: f64) {
        if let Some(path) = self.path.clone() {
            self.write_path(&path, now, true);
            return;
        }
        let Some(path) = project::save_dialog(&self.project_name, self.seed) else {
            return;
        };
        self.write_path(&path, now, true);
    }

    /// `confirm` asks before replacing a dirty project. The load dialog confirms first.
    fn load_path(&mut self, path: PathBuf, now: f64, confirm: bool) {
        if confirm && !self.confirm_discard() {
            return;
        }
        if !path.is_file() {
            let _ = project::forget(&storage::app_dir(), &path);
            self.notice = Some((text::status::MISSING.to_owned(), now));
            return;
        }
        match project::read(&path) {
            Ok(loaded) => {
                self.notice = Some((text::status::loaded_from(&path), now));
                self.apply_loaded(loaded, Some(path));
            }
            Err(err) => self.notice = Some((text::status::error(err), now)),
        }
    }

    fn apply_loaded(&mut self, loaded: Loaded, path: Option<PathBuf>) {
        let current = self.editor.settings();
        let mut settings = loaded.settings;
        settings.autosave = current.autosave;
        settings.autosave_secs = current.autosave_secs;
        self.editor.apply_loaded(loaded.seed, &settings);
        self.seed = loaded.seed;
        self.cells = loaded.cells.clamp(MIN_CELLS, MAX_CELLS);
        self.grid = loaded.grid;
        self.biomes = loaded.biomes;
        self.world = loaded.world;
        self.project_name = loaded.name;
        self.path = path;
        self.clear_edit_state();
        self.fit = true;
        self.renaming = false;
        self.saved = Some(self.project_file());
        self.last = self.saved.clone();
        self.dirty = false;
        if let Some(path) = &self.path {
            let _ = project::remember(&storage::app_dir(), path);
        }
    }

    /// `announce` shows the path in the header. Autosave stays quiet unless it fails.
    fn write_path(&mut self, path: &Path, now: f64, announce: bool) -> bool {
        let file = self.project_file();
        match project::write(path, &file) {
            Ok(()) => {
                self.path = Some(path.to_owned());
                self.saved = Some(file);
                self.last = self.saved.clone();
                self.dirty = false;
                let _ = project::remember(&storage::app_dir(), path);
                if announce {
                    self.notice = Some((text::status::saved_to(path), now));
                }
                true
            }
            Err(err) => {
                self.notice = Some((text::status::error(err), now));
                false
            }
        }
    }

    fn confirm_discard(&self) -> bool {
        if !self.dirty {
            return true;
        }
        rfd::MessageDialog::new()
            .set_title(text::status::DISCARD_TITLE)
            .set_description(text::status::DISCARD)
            .set_buttons(rfd::MessageButtons::YesNo)
            .show()
            == rfd::MessageDialogResult::Yes
    }

    fn project_file(&self) -> ProjectFile {
        ProjectFile::capture(
            &self.project_name,
            self.seed,
            self.cells,
            self.grid,
            &self.editor.settings(),
            &self.biomes,
            &self.world,
        )
    }

    fn clear_edit_state(&mut self) {
        self.moving = None;
        self.selected.clear();
        self.selecting = None;
        self.picker = None;
        self.placing = None;
        self.icon_drag = None;
        self.panning = false;
    }

    fn handle_dropped(&mut self, ctx: &egui::Context, now: f64) {
        let dropped: Vec<PathBuf> = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .map(|file| file.path().to_owned())
                .collect()
        });
        self.open_dropped(dropped, now);
    }

    /// Opens the first `.mmg` in the list. Another extension shows an error.
    fn open_dropped(&mut self, dropped: Vec<PathBuf>, now: f64) {
        if dropped.is_empty() {
            return;
        }
        let Some(path) = dropped.into_iter().find(|path| project::is_mmg(path)) else {
            self.notice = Some((text::status::NOT_MMG.to_owned(), now));
            return;
        };
        self.load_path(path, now, true);
    }

    fn sync_dirty(&mut self, now: f64) {
        let current = self.project_file();
        if self.saved.as_ref() != Some(&current) {
            if self.last.as_ref() != Some(&current) {
                self.dirty_at = now;
            }
            self.dirty = true;
        } else {
            self.dirty = false;
        }
        self.last = Some(current);
    }

    fn autosave(&mut self, ctx: &egui::Context, now: f64) {
        let settings = self.editor.settings();
        if !settings.autosave || !self.dirty || self.path.is_none() {
            return;
        }
        let delay = settings.autosave_secs as f64;
        let elapsed = now - self.dirty_at;
        if elapsed >= delay {
            let path = self.path.clone().unwrap();
            if !self.write_path(&path, now, false) {
                self.dirty_at = now;
                ctx.request_repaint_after(Duration::from_secs_f64(delay));
            }
        } else {
            let wait = (delay - elapsed).max(0.05);
            ctx.request_repaint_after(Duration::from_secs_f64(wait));
        }
    }

    fn export(&mut self, now: f64) {
        let settings = self.editor.settings();
        let layout = Layout::new(self.editor.viewport());
        let map = MapData {
            seed: self.seed,
            cells: self.cells,
            settings: &settings,
            world: &self.world,
            biomes: &self.biomes,
            layout: &layout,
        };
        if let Some(text) = export::save_with_dialog(&map) {
            self.notice = Some((text, now));
        }
    }

    /// Room being placed. Its position snaps to the cell grid, centered on the cursor
    /// cell. This runs before the picker, so the click that picks a size does not place it.
    fn update_placing(&mut self, p: &mut Pointer, layout: &Layout, map: Rect) -> Option<Ghost> {
        let choice = self.placing?;
        let mut ghost = None;
        if p.on_map() && !p.consumed {
            let (cx, cy) = layout.cell_at(self.view.screen_to_world(p.pos, map));
            let (x, y) = (cx - (choice.w / 2) as i32, cy - (choice.h / 2) as i32);
            let free = self.world.can_place(x, y, choice.w, choice.h);
            if p.left && free {
                let (kind, marker) = choice.pick.spec();
                self.world
                    .place_manual((x, y), (choice.w, choice.h), kind, marker);
                self.placing = None;
            } else {
                ghost = Some(Ghost { x, y, choice, free });
            }
        }
        if p.right {
            self.placing = None;
            ghost = None;
            p.consumed = true;
        }
        ghost
    }

    /// Drags the selection rectangle on an empty cell. Releasing selects every room
    /// the rectangle cuts. A click without travel clears the selection.
    fn update_selection(&mut self, p: &Pointer, layout: &Layout, map: Rect) {
        if p.right {
            self.selecting = None;
            return;
        }
        let Some(start) = self.selecting else {
            if p.left
                && p.on_map()
                && !p.consumed
                && !self.panning
                && self.picker.is_none()
                && self.placing.is_none()
                && self.moving.is_none()
                && self.icon_drag.is_none()
            {
                let (x, y) = layout.cell_at(self.view.screen_to_world(p.pos, map));
                if self.world.room_at(x, y).is_none() {
                    self.selecting = Some(p.pos);
                }
            }
            return;
        };
        if p.left_down {
            return;
        }
        if start.distance(p.pos) < SELECT_CLICK {
            self.selected.clear();
        } else {
            let zone = Rect::from_two_pos(start, p.pos);
            self.selected = self
                .world
                .rooms
                .iter()
                .enumerate()
                .filter_map(|(i, room)| {
                    let world = render::room_rect(room, layout);
                    let screen = Rect::from_min_max(
                        self.view.world_to_screen(world.min.to_vec2(), map),
                        self.view.world_to_screen(world.max.to_vec2(), map),
                    );
                    screen.intersects(zone).then_some(i)
                })
                .collect();
        }
        self.selecting = None;
    }

    /// Drags placed rooms with the left button. A selected room drags the whole
    /// selection. Another room is dragged alone, and the selection is cleared.
    /// Releasing on the map drops them. A taken spot, Escape or a right click
    /// leaves them in place. A drag of no cells changes nothing.
    fn update_moving(
        &mut self,
        p: &Pointer,
        hovered: Option<usize>,
        layout: &Layout,
        map: Rect,
    ) -> Vec<Ghost> {
        let cell = layout.cell_at(self.view.screen_to_world(p.pos, map));
        let Some(m) = &self.moving else {
            if let (true, Some(room)) = (p.left, hovered)
                && self.selecting.is_none()
            {
                let rooms = if self.selected.contains(&room) {
                    self.selected.clone()
                } else {
                    self.selected.clear();
                    vec![room]
                };
                let origins = rooms
                    .iter()
                    .map(|&i| {
                        let r = &self.world.rooms[i];
                        (r.x, r.y)
                    })
                    .collect();
                self.moving = Some(Moving {
                    rooms,
                    origins,
                    anchor: cell,
                });
            }
            return Vec::new();
        };
        let delta = (cell.0 - m.anchor.0, cell.1 - m.anchor.1);
        let moves: Vec<_> = m
            .rooms
            .iter()
            .zip(&m.origins)
            .map(|(&idx, &(x, y))| (idx, x + delta.0, y + delta.1))
            .collect();
        if p.right {
            self.moving = None;
            return Vec::new();
        }
        if p.left_down {
            let free = match moves.as_slice() {
                [(idx, x, y)] => self.world.can_move(*idx, *x, *y),
                _ => self.world.can_move_rooms(&moves),
            };
            return moves
                .into_iter()
                .map(|(idx, x, y)| {
                    let r = &self.world.rooms[idx];
                    Ghost {
                        x,
                        y,
                        choice: Choice {
                            pick: Pick::Room,
                            w: r.w,
                            h: r.h,
                        },
                        free,
                    }
                })
                .collect();
        }
        if p.on_map() && delta != (0, 0) {
            match moves.as_slice() {
                [(idx, x, y)] => {
                    self.world.move_room(*idx, *x, *y);
                }
                _ => {
                    self.world.move_rooms(&moves);
                }
            }
        }
        self.moving = None;
        Vec::new()
    }

    /// Drags an icon of the room type legend onto a room.
    fn update_icon_drag(&mut self, p: &Pointer, legend: &RoomLegend, hovered: Option<usize>) {
        let Some(icon) = self.icon_drag else {
            if p.left
                && !p.consumed
                && self.picker.is_none()
                && self.placing.is_none()
                && self.moving.is_none()
            {
                self.icon_drag = legend.icon_at(p.pos);
            }
            return;
        };
        if p.right {
            self.icon_drag = None;
        } else if !p.left_down {
            if let Some(room) = hovered {
                self.tag_room(room, icon);
            }
            self.icon_drag = None;
        }
    }

    fn update_cursor(
        &self,
        ctx: &egui::Context,
        p: &Pointer,
        picker: Option<&Picker>,
        ghost: Option<&Ghost>,
        move_ghosts: &[Ghost],
        hovered: Option<usize>,
    ) {
        let moving = self.moving.is_some();
        let blocked = ghost.is_some_and(|g| !g.free) || move_ghosts.iter().any(|g| !g.free);
        let icon = if self.selecting.is_some() {
            CursorIcon::Crosshair
        } else if self.icon_drag.is_some() {
            if hovered.is_some() {
                CursorIcon::Move
            } else {
                CursorIcon::NotAllowed
            }
        } else if moving || self.placing.is_some() {
            if blocked {
                CursorIcon::NotAllowed
            } else if moving && !move_ghosts.is_empty() {
                CursorIcon::Move
            } else if ghost.is_some() {
                CursorIcon::Crosshair
            } else if moving {
                CursorIcon::NotAllowed
            } else {
                CursorIcon::Default
            }
        } else if self.panning {
            CursorIcon::Move
        } else if picker.is_some_and(|ui| ui.items.iter().any(|(_, r)| r.contains(p.pos))) {
            CursorIcon::PointingHand
        } else if p.over_picker {
            CursorIcon::Default
        } else if p.over_seed || p.over_legend {
            CursorIcon::PointingHand
        } else if hovered.is_some() {
            CursorIcon::Grab
        } else {
            CursorIcon::Default
        };
        ctx.set_cursor_icon(icon);
    }

    fn tag_room(&mut self, room: usize, icon: IconId) {
        match icon {
            IconId::Boss => self.world.toggle_marker(room, Marker::Boss),
            IconId::Save => self.world.toggle_marker(room, Marker::Save),
            IconId::Secret => self.world.toggle_kind(room, RoomKind::Secret),
            IconId::Portal => self.world.toggle_kind(room, RoomKind::Portal),
        }
    }

    fn update_picker(&mut self, p: &mut Pointer, picker_ui: &mut Option<Picker>) {
        if let Some(popup) = picker_ui.as_ref()
            && (p.left || p.right)
        {
            let hit = popup.items.iter().find(|(_, r)| r.contains(p.pos));
            if let (true, Some((choice, _))) = (p.left, hit) {
                self.placing = Some(*choice);
                self.picker = None;
            } else if !p.over_picker {
                self.picker = None;
                p.consumed = true;
            }
        }
        if self.picker.is_none() {
            *picker_ui = None;
            if p.right
                && p.on_map()
                && !p.consumed
                && self.placing.is_none()
                && self.moving.is_none()
                && self.selecting.is_none()
                && self.icon_drag.is_none()
            {
                self.picker = Some(p.pos);
            }
        }
    }

    /// Deletes the dragged room, or the hovered one if nothing is dragged.
    /// One press removes one room. The last room stays.
    /// Returns the removed index.
    fn delete_shortcut(&mut self, ctx: &egui::Context, hovered: Option<usize>) -> Option<usize> {
        if self.editor.open || self.renaming {
            return None;
        }
        let pressed = ctx.input(|input| {
            input.events.iter().any(|event| {
                matches!(
                    event,
                    Event::Key {
                        key: Key::Delete | Key::Backspace,
                        pressed: true,
                        repeat: false,
                        ..
                    }
                )
            })
        });
        if !pressed {
            return None;
        }
        let idx = self
            .moving
            .as_ref()
            .filter(|m| m.rooms.len() == 1)
            .map(|m| m.rooms[0])
            .or(hovered)?;
        if !self.world.remove_room(idx) {
            return None;
        }
        self.moving = None;
        self.selected.retain(|&i| i != idx);
        for i in &mut self.selected {
            if *i > idx {
                *i -= 1;
            }
        }
        Some(idx)
    }

    fn room_shortcuts(&mut self, ctx: &egui::Context, room: usize) {
        if self.editor.open || self.renaming {
            return;
        }
        if ctx.input(|i| i.key_pressed(Key::B)) {
            self.tag_room(room, IconId::Boss);
        }
        if ctx.input(|i| i.key_pressed(Key::S) && !i.modifiers.command) {
            self.tag_room(room, IconId::Save);
        }
        if ctx.input(|i| i.key_pressed(Key::H)) {
            self.tag_room(room, IconId::Secret);
        }
        if ctx.input(|i| i.key_pressed(Key::P)) {
            self.tag_room(room, IconId::Portal);
        }
    }
}
