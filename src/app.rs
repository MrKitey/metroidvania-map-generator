//! Application state and the work done each frame: input, regeneration, camera,
//! hover, room placement and drawing.

use crate::biome::{self, Biome};
use crate::editor::{self, Editor, EditorAction};
use crate::export::{self, MapData};
use crate::icons::{IconId, Icons};
use crate::layout::Layout;
use crate::render::{self, COPIED_TIME, Frame, Ghost, Hud, Picker, RoomLegend, picker_layout};
use crate::settings;
use crate::text;
use crate::view::View;
use crate::world::{Choice, Marker, Params, Pick, RoomKind, World, time_seed};
use egui::{Color32, CursorIcon, Key, PointerButton, Pos2, Rect, Sense, ViewportCommand};
use std::time::Duration;

/// Map size limits, as a number of room cells.
const MIN_CELLS: u32 = 96;
const MAX_CELLS: u32 = 5120;
/// Cells added or removed per arrow key press.
const STEP: u32 = 64;
const START_CELLS: u32 = 768;
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

/// Room being dragged with the left button.
struct Moving {
    room: usize,
    /// Cell grabbed, relative to the room top-left cell, so the room does not jump.
    grab: (i32, i32),
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
    /// Placed room being dragged, if any.
    moving: Option<Moving>,
    /// Icon of the room type legend being dragged onto a room, if any.
    icon_drag: Option<IconId>,
    /// Last export result and when it happened, shown in the header for a while.
    notice: Option<(String, f64)>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> App {
        render::apply_theme(&cc.egui_ctx);
        let seed = time_seed();
        let biomes = biome::load();
        let editor = Editor::new(seed, &settings::load());
        let world = build_world(START_CELLS, seed, &biomes, &editor);
        App {
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
            icon_drag: None,
            notice: None,
        }
    }

    fn regen(&mut self) {
        self.editor.set_seed(self.seed);
        self.world = build_world(self.cells, self.seed, &self.biomes, &self.editor);
        self.fit = true;
    }

    fn apply_action(&mut self, action: EditorAction) {
        match action {
            EditorAction::UseSeed(seed) => {
                self.seed = seed;
                self.regen();
            }
            EditorAction::Regenerate => {
                self.seed = time_seed();
                self.regen();
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        self.handle_keys(&ctx);

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
        ui.horizontal_centered(|ui| {
            let notice = self
                .notice
                .as_ref()
                .filter(|(_, at)| now - at < NOTICE_TIME)
                .map(|(text, _)| text.as_str());
            ui.label(text::hud::header(self.world.rooms.len(), notice));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(text::hud::HELP_BUTTON).clicked() {
                    self.help_open = !self.help_open;
                }
                if ui.button(text::hud::EXPORT).clicked() {
                    self.export(now);
                }
            });
        });
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
            over_legend: !over_picker && legend.rect.contains(pointer_pos),
            on_map_rect: response.contains_pointer(),
            consumed: false,
        };

        if pointer.over_seed && pointer.left {
            ctx.copy_text(self.seed.to_string());
            self.copied_at = now;
        }

        let ghost = self.update_placing(&mut pointer, &layout, map_rect);
        self.update_picker(&mut pointer, &mut picker_ui);

        let hovered = if pointer.on_map()
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
        let ghost = ghost.or(self.update_moving(&pointer, hovered, &layout, map_rect));
        self.update_icon_drag(&pointer, &legend, hovered);
        self.update_cursor(&ctx, &pointer, picker_ui.as_ref(), ghost.as_ref(), hovered);

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
    fn handle_keys(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(Key::Tab)) {
            self.editor.open = !self.editor.open;
        }

        let mut escape = ctx.input(|i| i.key_pressed(Key::Escape));
        if escape && self.help_open {
            self.help_open = false;
            escape = false;
        }
        if escape
            && (self.picker.is_some()
                || self.placing.is_some()
                || self.moving.is_some()
                || self.icon_drag.is_some())
        {
            self.picker = None;
            self.placing = None;
            self.moving = None;
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
            self.seed = time_seed();
            regen = true;
        }
        if ctx.input(|i| i.key_pressed(Key::C)) {
            self.fit = true;
        }
        if ctx.input(|i| i.key_pressed(Key::G)) {
            self.grid = !self.grid;
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::ArrowUp))
            && self.cells < MAX_CELLS
        {
            self.cells += STEP;
            regen = true;
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::ArrowDown))
            && self.cells > MIN_CELLS
        {
            self.cells -= STEP;
            regen = true;
        }
        if regen {
            self.regen();
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

    /// Drags a placed room with the left button. Pressing on a room grabs it, releasing
    /// on the map drops it. A taken spot, Escape or a right click leave it in place.
    fn update_moving(
        &mut self,
        p: &Pointer,
        hovered: Option<usize>,
        layout: &Layout,
        map: Rect,
    ) -> Option<Ghost> {
        let cell = layout.cell_at(self.view.screen_to_world(p.pos, map));
        let Some(m) = &self.moving else {
            if let (true, Some(room)) = (p.left, hovered) {
                let r = &self.world.rooms[room];
                self.moving = Some(Moving {
                    room,
                    grab: (cell.0 - r.x, cell.1 - r.y),
                });
            }
            return None;
        };
        let r = &self.world.rooms[m.room];
        let (x, y) = (cell.0 - m.grab.0, cell.1 - m.grab.1);
        if p.right {
            self.moving = None;
            return None;
        }
        if p.left_down {
            return Some(Ghost {
                x,
                y,
                choice: Choice {
                    pick: Pick::Room,
                    w: r.w,
                    h: r.h,
                },
                free: self.world.can_move(m.room, x, y),
            });
        }
        let room = m.room;
        if p.on_map() && (x, y) != (r.x, r.y) {
            self.world.move_room(room, x, y);
        }
        self.moving = None;
        None
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
        hovered: Option<usize>,
    ) {
        let moving = self.moving.is_some();
        let icon = if self.icon_drag.is_some() {
            if hovered.is_some() {
                CursorIcon::Move
            } else {
                CursorIcon::NotAllowed
            }
        } else if moving || self.placing.is_some() {
            match ghost {
                Some(g) if !g.free => CursorIcon::NotAllowed,
                Some(_) if moving => CursorIcon::Move,
                Some(_) => CursorIcon::Crosshair,
                None if moving => CursorIcon::NotAllowed,
                None => CursorIcon::Default,
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
                && self.icon_drag.is_none()
            {
                self.picker = Some(p.pos);
            }
        }
    }

    fn room_shortcuts(&mut self, ctx: &egui::Context, room: usize) {
        if self.editor.open {
            return;
        }
        if ctx.input(|i| i.key_pressed(Key::B)) {
            self.tag_room(room, IconId::Boss);
        }
        if ctx.input(|i| i.key_pressed(Key::S)) {
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
