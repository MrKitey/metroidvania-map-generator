//! Application state and the work done each frame: input, regeneration, camera,
//! hover, room placement and drawing.

use crate::biome::{self, Biome};
use crate::editor::{self, Editor, EditorAction};
use crate::export::{self, MapData};
use crate::icons::{IconId, Icons};
use crate::layout::{Layout, TOP};
use crate::render::{self, COPIED_TIME, Frame, Ghost, Hud, Picker, RoomLegend, picker_layout};
use crate::settings;
use crate::view::View;
use crate::world::{Choice, Marker, Params, Pick, RoomKind, World, time_seed};
use macroquad::prelude::*;

/// Map size limits, as a number of room cells.
const MIN_CELLS: u32 = 96;
const MAX_CELLS: u32 = 5120;
/// Cells added or removed per arrow key press.
const STEP: u32 = 64;
const START_CELLS: u32 = 768;
/// Seconds the export result stays in the header.
const NOTICE_TIME: f64 = 4.0;

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
    pos: Vec2,
    left: bool,
    right: bool,
    over_editor: bool,
    over_seed: bool,
    over_help_button: bool,
    over_help: bool,
    over_picker: bool,
    over_legend: bool,
    /// Set when a popup used the click, so it must not reach the map.
    consumed: bool,
}

impl Pointer {
    /// True when the cursor is over the map itself, not over a panel, button or popup.
    fn on_map(&self) -> bool {
        !(self.over_editor
            || self.over_seed
            || self.over_help
            || self.over_picker
            || self.over_legend
            || self.pos.y < TOP)
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
    /// Top-left corner of the open room picker.
    picker: Option<Vec2>,
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
    pub fn new() -> App {
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
            picker: None,
            placing: None,
            moving: None,
            icon_drag: None,
            notice: None,
        }
    }

    /// Runs one frame: input, optional regeneration, camera, hover and drawing.
    /// Returns false when the user quits.
    pub fn frame(&mut self) -> bool {
        if !self.handle_keys() {
            return false;
        }

        // Screen width taken by the editor panel on the right.
        let reserved = if self.editor.open {
            editor::PANEL_W
        } else {
            0.0
        };
        let layout = Layout::new(self.editor.viewport());
        if self.fit {
            self.view.fit(&self.world, &layout, reserved);
            self.fit = false;
        }

        let over_editor = self.editor.is_mouse_over();
        self.view.update(reserved, !over_editor);
        let camera = self.view.camera(reserved);

        let copied = get_time() - self.copied_at < COPIED_TIME;
        let hud = Hud::new(self.seed, copied, reserved);
        let legend = RoomLegend::new(&self.world, &self.biomes);
        // The picker is drawn over the header buttons and the seed, so it gets the clicks first.
        let mouse = Vec2::from(mouse_position());
        let mut picker_ui = self
            .picker
            .map(|a| picker_layout(&self.editor.sizes(), &layout, a));
        let over_picker = picker_ui.as_ref().is_some_and(|p| p.rect.contains(mouse));
        let over_help_button = !over_picker && hud.help.contains(mouse);
        let mut pointer = Pointer {
            pos: mouse,
            left: is_mouse_button_pressed(MouseButton::Left),
            right: is_mouse_button_pressed(MouseButton::Right),
            over_editor,
            over_seed: !over_picker && hud.seed.contains(mouse),
            over_help_button,
            over_help: over_help_button
                || (!over_picker && self.help_open && hud.popup.contains(mouse)),
            over_picker,
            over_legend: !over_picker && legend.rect.contains(mouse),
            consumed: false,
        };

        self.click_header(&mut pointer);
        if !pointer.over_picker && pointer.left && hud.export.contains(pointer.pos) {
            self.export(&layout);
        }
        let ghost = self.update_placing(&mut pointer, &layout, &camera);
        self.update_picker(&mut pointer, &mut picker_ui, &layout);

        let hovered = if pointer.on_map()
            && !pointer.consumed
            && self.picker.is_none()
            && self.placing.is_none()
        {
            let (x, y) = layout.cell_at(camera.screen_to_world(pointer.pos));
            self.world.room_at(x, y)
        } else {
            None
        };
        if let Some(i) = hovered {
            self.room_shortcuts(i);
        }
        let ghost = ghost.or(self.update_moving(&pointer, hovered, &layout, &camera));
        self.update_icon_drag(&pointer, &legend, hovered);

        let notice = self
            .notice
            .as_ref()
            .filter(|(_, at)| get_time() - at < NOTICE_TIME)
            .map(|(text, _)| text.as_str());
        let frame = Frame {
            layout: &layout,
            view: &self.view,
            camera: &camera,
            hud: &hud,
            legend: &legend,
            hovered,
            help_open: self.help_open,
            grid: self.grid,
            picker: picker_ui.as_ref(),
            ghost,
            dragged_icon: self.icon_drag,
            notice,
        };
        render::draw_world(
            &self.world,
            &self.biomes,
            &frame,
            &mut self.icons,
            &mut self.legend_icons,
        );
        true
    }

    /// Keyboard shortcuts and the editor panel, then regeneration if asked.
    /// Returns false when the user quits.
    fn handle_keys(&mut self) -> bool {
        if is_key_pressed(KeyCode::Tab) {
            self.editor.open = !self.editor.open;
        }

        // Escape closes the shortcuts popup first, then the room picker and the room
        // being placed. It only quits when none is open.
        let mut escape = is_key_pressed(KeyCode::Escape);
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

        let mut regen = false;
        // Shortcuts are disabled while the editor is open so typing in text fields is safe.
        if self.editor.open {
            match self.editor.draw(&mut self.biomes) {
                Some(EditorAction::UseSeed(seed)) => {
                    self.seed = seed;
                    regen = true;
                }
                Some(EditorAction::Regenerate) => {
                    self.seed = time_seed();
                    regen = true;
                }
                None => {}
            }
        } else {
            if escape {
                return false;
            }
            if is_key_pressed(KeyCode::R) {
                self.seed = time_seed();
                regen = true;
            }
            if is_key_pressed(KeyCode::C) {
                self.fit = true;
            }
            if is_key_pressed(KeyCode::G) {
                self.grid = !self.grid;
            }
            if (is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::Up))
                && self.cells < MAX_CELLS
            {
                self.cells += STEP;
                regen = true;
            }
            if (is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::Down))
                && self.cells > MIN_CELLS
            {
                self.cells -= STEP;
                regen = true;
            }
        }

        if regen {
            self.editor.set_seed(self.seed);
            self.world = build_world(self.cells, self.seed, &self.biomes, &self.editor);
            self.fit = true;
        }
        true
    }

    /// Clicks on the seed label and the shortcuts button and popup.
    fn click_header(&mut self, p: &mut Pointer) {
        if p.over_seed && p.left {
            miniquad::window::clipboard_set(&self.seed.to_string());
            self.copied_at = get_time();
        }
        // A click outside the open popup only closes it, it does not reach the map.
        if p.over_help_button && p.left {
            self.help_open = !self.help_open;
        } else if self.help_open && (p.left || p.right) && !p.over_help {
            self.help_open = false;
            p.consumed = true;
        }
    }

    /// Asks where to save, then writes the zip and shows the result in the header.
    fn export(&mut self, layout: &Layout) {
        let settings = self.editor.settings();
        let map = MapData {
            seed: self.seed,
            cells: self.cells,
            settings: &settings,
            world: &self.world,
            biomes: &self.biomes,
            layout,
        };
        if let Some(text) = export::save_with_dialog(&map, &mut self.icons) {
            self.notice = Some((text, get_time()));
        }
    }

    /// Room being placed. Its position snaps to the cell grid, centered on the cursor
    /// cell. This runs before the picker, so the click that picks a size does not place it.
    /// Returns the preview to draw.
    fn update_placing(
        &mut self,
        p: &mut Pointer,
        layout: &Layout,
        camera: &Camera2D,
    ) -> Option<Ghost> {
        let choice = self.placing?;
        let mut ghost = None;
        if p.on_map() && !p.consumed {
            let (cx, cy) = layout.cell_at(camera.screen_to_world(p.pos));
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
    /// Returns the preview to draw while dragging.
    fn update_moving(
        &mut self,
        p: &Pointer,
        hovered: Option<usize>,
        layout: &Layout,
        camera: &Camera2D,
    ) -> Option<Ghost> {
        let cell = layout.cell_at(camera.screen_to_world(p.pos));
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
        if is_mouse_button_down(MouseButton::Left) {
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
        // Button released. A drop on the spot it came from changes nothing.
        let room = m.room;
        if p.on_map() && (x, y) != (r.x, r.y) {
            self.world.move_room(room, x, y);
        }
        self.moving = None;
        None
    }

    /// Drags an icon of the room type legend onto a room. Pressing on a legend row grabs
    /// the icon, releasing over a room applies it. Escape or a right click cancels.
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
        } else if !is_mouse_button_down(MouseButton::Left) {
            if let Some(room) = hovered {
                self.tag_room(room, icon);
            }
            self.icon_drag = None;
        }
    }

    /// Sets the marker or kind shown by `icon` on a room, or clears it if the room
    /// already has it.
    fn tag_room(&mut self, room: usize, icon: IconId) {
        match icon {
            IconId::Boss => self.world.toggle_marker(room, Marker::Boss),
            IconId::Save => self.world.toggle_marker(room, Marker::Save),
            IconId::Secret => self.world.toggle_kind(room, RoomKind::Secret),
            IconId::Portal => self.world.toggle_kind(room, RoomKind::Portal),
        }
    }

    /// Picker clicks: a size starts the placement, a click elsewhere closes the popup.
    /// A right click on the map opens the picker at the cursor.
    fn update_picker(&mut self, p: &mut Pointer, picker_ui: &mut Option<Picker>, layout: &Layout) {
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
        if let (Some(anchor), None) = (self.picker, &*picker_ui) {
            *picker_ui = Some(picker_layout(&self.editor.sizes(), layout, anchor));
        }
    }

    /// Room shortcuts act on the room under the cursor. They are off while the editor
    /// is open.
    fn room_shortcuts(&mut self, room: usize) {
        if self.editor.open {
            return;
        }
        if is_key_pressed(KeyCode::B) {
            self.tag_room(room, IconId::Boss);
        }
        if is_key_pressed(KeyCode::S) {
            self.tag_room(room, IconId::Save);
        }
        if is_key_pressed(KeyCode::H) {
            self.tag_room(room, IconId::Secret);
        }
        if is_key_pressed(KeyCode::P) {
            self.tag_room(room, IconId::Portal);
        }
    }
}
