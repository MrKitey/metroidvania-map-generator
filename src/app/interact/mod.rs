//! Pointer gestures on the map: place, select, move, and room shortcuts.

mod drag;

use super::{UndoEntry, remember};
use crate::biome::Biome;
use crate::layer::LayerStack;
use crate::layout::Layout;
use crate::render::{Ghost, RoomLegend};
use crate::text::icons as ids;
use crate::view::View;
use crate::world::{Choice, RoomKind};
use egui::{Context, Event, Key, Pos2, Rect, Vec2};

/// Pointer travel below this, in points, is a click and clears the selection.
const SELECT_CLICK: f32 = 4.0;

/// Mouse state of one frame: where it is, what it is over, and the clicks.
pub(super) struct Pointer {
    pub(super) pos: Pos2,
    pub(super) left: bool,
    pub(super) left_down: bool,
    pub(super) right: bool,
    pub(super) over_seed: bool,
    pub(super) over_legend: bool,
    pub(super) on_map_rect: bool,
    /// Set when a gesture used the click, so it must not reach the map.
    pub(super) consumed: bool,
}

impl Pointer {
    /// True when the cursor is over the map itself, not over the seed or a legend.
    pub(super) fn on_map(&self) -> bool {
        self.on_map_rect && !(self.over_seed || self.over_legend)
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

/// Whole layer being dragged by its name label.
struct MovingLayer {
    layer: usize,
    origins: Vec<(i32, i32)>,
    anchor: (i32, i32),
}

pub(super) struct Interaction {
    pub(super) panning: bool,
    /// Room being dragged out of the room list.
    pub(super) placing: Option<Choice>,
    /// Size of that entry in the list, used while the cursor is off the map.
    pub(super) placing_size: Vec2,
    /// Placed rooms being dragged, if any.
    moving: Option<Moving>,
    /// Layer being dragged by its on-map name label.
    moving_layer: Option<MovingLayer>,
    /// Rooms chosen by the selection rectangle (active layer only).
    selected: Vec<usize>,
    /// Screen position where the selection rectangle started.
    selecting: Option<Pos2>,
    /// Icon id of the room type legend being dragged onto a room, if any.
    pub(super) icon_drag: Option<String>,
    /// Biome paint brush: click a legend biome, then paint rooms.
    pub(super) paint_biome: Option<usize>,
    /// Undo snapshot already taken for the current paint stroke.
    paint_stroke_saved: bool,
}

impl Interaction {
    pub(super) fn new() -> Interaction {
        Interaction {
            panning: false,
            placing: None,
            placing_size: Vec2::ZERO,
            moving: None,
            moving_layer: None,
            selected: Vec::new(),
            selecting: None,
            icon_drag: None,
            paint_biome: None,
            paint_stroke_saved: false,
        }
    }

    pub(super) fn clear(&mut self) {
        self.moving = None;
        self.moving_layer = None;
        self.selected.clear();
        self.selecting = None;
        self.placing = None;
        self.icon_drag = None;
        self.paint_biome = None;
        self.paint_stroke_saved = false;
        self.panning = false;
    }

    /// Clears an open gesture. Returns whether one was open, so Escape is consumed.
    pub(super) fn cancel(&mut self) -> bool {
        let active = self.placing.is_some()
            || self.moving.is_some()
            || self.moving_layer.is_some()
            || self.selecting.is_some()
            || !self.selected.is_empty()
            || self.icon_drag.is_some()
            || self.paint_biome.is_some();
        if !active {
            return false;
        }
        self.placing = None;
        self.moving = None;
        self.moving_layer = None;
        self.selecting = None;
        self.selected.clear();
        self.icon_drag = None;
        self.paint_biome = None;
        self.paint_stroke_saved = false;
        true
    }

    pub(super) fn selected(&self) -> &[usize] {
        &self.selected
    }

    pub(super) fn select_rect(&self, pos: Pos2) -> Option<Rect> {
        self.selecting.map(|start| Rect::from_two_pos(start, pos))
    }

    /// Activates biome paint mode. Clicking the same biome again turns it off.
    pub(super) fn set_paint_biome(&mut self, biome: usize) {
        self.paint_biome = match self.paint_biome {
            Some(b) if b == biome => None,
            _ => Some(biome),
        };
        self.paint_stroke_saved = false;
        self.moving = None;
        self.selecting = None;
        self.icon_drag = None;
    }

    /// Paints the brush biome onto rooms under the cursor while the button is held.
    pub(super) fn update_biome_paint(
        &mut self,
        p: &Pointer,
        hovered: Option<(usize, usize)>,
        layers: &mut LayerStack,
        undo: &mut Vec<UndoEntry>,
        seed: u64,
        cells: u32,
        biomes: &[Biome],
    ) {
        let Some(biome) = self.paint_biome else {
            self.paint_stroke_saved = false;
            return;
        };
        if p.right {
            self.paint_biome = None;
            self.paint_stroke_saved = false;
            return;
        }
        if !p.left_down {
            self.paint_stroke_saved = false;
            return;
        }
        if !p.on_map() || p.consumed {
            return;
        }
        let Some((layer, room)) = hovered else {
            return;
        };
        if layer != layers.active() || layers.is_active_locked() {
            return;
        }
        let world = layers.active_world();
        if world.rooms.get(room).is_some_and(|r| r.biome == biome) {
            return;
        }
        if !self.paint_stroke_saved {
            remember(undo, layers, seed, cells, biomes);
            self.paint_stroke_saved = true;
        }
        layers.active_world_mut().set_room_biome(room, biome);
    }

    /// Room dragged from the list. On the map it snaps to the cell grid, centered on
    /// the cursor cell. Releasing on a free spot places it. Releasing elsewhere cancels.
    pub(super) fn update_placing(
        &mut self,
        layers: &mut LayerStack,
        view: &View,
        p: &mut Pointer,
        layout: &Layout,
        map: Rect,
        undo: &mut Vec<UndoEntry>,
        seed: u64,
        cells: u32,
        biomes: &[Biome],
    ) -> Option<Ghost> {
        let choice = self.placing?;
        if p.right {
            self.placing = None;
            p.consumed = true;
            return None;
        }
        let on_map = p.on_map() && !p.consumed;
        if !on_map {
            if !p.left_down {
                self.placing = None;
            }
            return None;
        }
        if layers.is_active_locked() {
            if !p.left_down {
                self.placing = None;
            }
            return None;
        }
        let world = layers.active_world();
        let (cx, cy) = layout.cell_at(view.screen_to_world(p.pos, map));
        let (x, y) = (cx - (choice.w / 2) as i32, cy - (choice.h / 2) as i32);
        let free = world.can_place(x, y, choice.w, choice.h);
        if p.left_down {
            return Some(Ghost { x, y, choice, free });
        }
        if free {
            let (kind, marker) = choice.pick.spec();
            remember(undo, layers, seed, cells, biomes);
            layers.active_world_mut().place_manual(
                (x, y),
                (choice.w, choice.h),
                kind,
                marker,
                self.paint_biome,
            );
        }
        self.placing = None;
        None
    }

    /// Drags the selection rectangle on an empty cell. Releasing selects every room
    /// the rectangle cuts on the active layer. A click without travel clears the selection.
    pub(super) fn update_selection(
        &mut self,
        p: &Pointer,
        view: &View,
        layers: &LayerStack,
        layout: &Layout,
        map: Rect,
    ) {
        if p.right {
            self.selecting = None;
            return;
        }
        let world = layers.active_world();
        let Some(start) = self.selecting else {
            if p.left
                && p.on_map()
                && !p.consumed
                && !self.panning
                && self.placing.is_none()
                && self.moving.is_none()
                && self.moving_layer.is_none()
                && self.icon_drag.is_none()
                && self.paint_biome.is_none()
            {
                let (x, y) = layout.cell_at(view.screen_to_world(p.pos, map));
                if layers.room_at(x, y).is_none() {
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
            self.selected = world
                .rooms
                .iter()
                .enumerate()
                .filter_map(|(i, room)| {
                    let world_rect = crate::render::room_rect(room, layout);
                    let screen = Rect::from_min_max(
                        view.world_to_screen(world_rect.min.to_vec2(), map),
                        view.world_to_screen(world_rect.max.to_vec2(), map),
                    );
                    screen.intersects(zone).then_some(i)
                })
                .collect();
        }
        self.selecting = None;
    }

    /// Drags an icon of the room type legend onto a room.
    pub(super) fn update_icon_drag(
        &mut self,
        p: &Pointer,
        legend: &RoomLegend,
        layers: &mut LayerStack,
        hovered: Option<(usize, usize)>,
        undo: &mut Vec<UndoEntry>,
        seed: u64,
        cells: u32,
        biomes: &[Biome],
    ) {
        let Some(icon) = self.icon_drag.clone() else {
            if p.left
                && !p.consumed
                && self.placing.is_none()
                && self.moving.is_none()
                && self.paint_biome.is_none()
            {
                self.icon_drag = legend.icon_at(p.pos);
            }
            return;
        };
        if p.right {
            self.icon_drag = None;
        } else if !p.left_down {
            if let Some((layer, room)) = hovered
                && layer == layers.active()
                && !layers.is_active_locked()
            {
                tag_room(layers, room, &icon, undo, seed, cells, biomes);
            }
            self.icon_drag = None;
        }
    }

    /// Deletes the selection, the dragged group, or the hovered room on the active layer.
    /// Returns how many rooms were removed.
    pub(super) fn delete_shortcut(
        &mut self,
        ctx: &Context,
        layers: &mut LayerStack,
        blocked: bool,
        hovered: Option<(usize, usize)>,
        undo: &mut Vec<UndoEntry>,
        seed: u64,
        cells: u32,
        biomes: &[Biome],
    ) -> usize {
        if blocked || layers.is_active_locked() {
            return 0;
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
            return 0;
        }
        let targets = if !self.selected.is_empty() {
            self.selected.clone()
        } else if let Some(moving) = &self.moving {
            moving.rooms.clone()
        } else if let Some((layer, idx)) = hovered {
            if layer != layers.active() {
                return 0;
            }
            vec![idx]
        } else {
            return 0;
        };
        remember(undo, layers, seed, cells, biomes);
        let removed = layers.active_world_mut().remove_rooms(&targets);
        if removed > 0 {
            self.moving = None;
            self.selected.clear();
        } else {
            undo.pop();
        }
        removed
    }

    pub(super) fn room_shortcuts(
        &mut self,
        ctx: &Context,
        layers: &mut LayerStack,
        blocked: bool,
        hit: (usize, usize),
        undo: &mut Vec<UndoEntry>,
        seed: u64,
        cells: u32,
        biomes: &[Biome],
    ) {
        if blocked {
            return;
        }
        let (layer, room) = hit;
        if layer != layers.active() || layers.is_active_locked() {
            return;
        }
        if ctx.input(|i| i.key_pressed(Key::B)) {
            tag_room(layers, room, ids::BOSS, undo, seed, cells, biomes);
        }
        if ctx.input(|i| i.key_pressed(Key::S) && !i.modifiers.command) {
            tag_room(layers, room, ids::SAVE, undo, seed, cells, biomes);
        }
        if ctx.input(|i| i.key_pressed(Key::H)) {
            tag_room(layers, room, ids::SECRET, undo, seed, cells, biomes);
        }
        if ctx.input(|i| i.key_pressed(Key::P)) {
            tag_room(layers, room, ids::PORTAL, undo, seed, cells, biomes);
        }
    }

    /// Activates a layer on name click. Starts a whole-layer drag when unlocked.
    pub(super) fn begin_layer_drag(
        &mut self,
        p: &mut Pointer,
        layers: &mut LayerStack,
        labels: &[(usize, Rect)],
        view: &View,
        layout: &Layout,
        map: Rect,
    ) {
        if self.moving_layer.is_some() || self.moving.is_some() || self.placing.is_some() {
            return;
        }
        if !(p.left && p.on_map() && !p.consumed) {
            return;
        }
        for &(idx, rect) in labels.iter().rev() {
            if !rect.contains(p.pos) {
                continue;
            }
            layers.set_active(idx);
            self.selected.clear();
            p.consumed = true;
            let layer = &layers.layers()[idx];
            if layer.locked || layer.world.rooms.is_empty() {
                return;
            }
            let origins = layer.world.rooms.iter().map(|r| (r.x, r.y)).collect();
            let anchor = layout.cell_at(view.screen_to_world(p.pos, map));
            self.moving_layer = Some(MovingLayer {
                layer: idx,
                origins,
                anchor,
            });
            return;
        }
    }
}

fn tag_room(
    layers: &mut LayerStack,
    room: usize,
    icon: &str,
    undo: &mut Vec<UndoEntry>,
    seed: u64,
    cells: u32,
    biomes: &[Biome],
) {
    let world = layers.active_world();
    if matches!(icon, ids::SECRET | ids::PORTAL) && world.rooms[room].kind == RoomKind::Corridor {
        return;
    }
    remember(undo, layers, seed, cells, biomes);
    layers.active_world_mut().toggle_icon(room, icon);
}
