//! Pointer gestures on the map: place, select, move, and room shortcuts.

mod drag;

use crate::icons::IconId;
use crate::layout::Layout;
use crate::render::{Ghost, RoomLegend};
use crate::view::View;
use crate::world::{Choice, Marker, RoomKind, World};
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

pub(super) struct Interaction {
    pub(super) panning: bool,
    /// Room being dragged out of the room list.
    pub(super) placing: Option<Choice>,
    /// Size of that entry in the list, used while the cursor is off the map.
    pub(super) placing_size: Vec2,
    /// Placed rooms being dragged, if any.
    moving: Option<Moving>,
    /// Rooms chosen by the selection rectangle.
    selected: Vec<usize>,
    /// Screen position where the selection rectangle started.
    selecting: Option<Pos2>,
    /// Icon of the room type legend being dragged onto a room, if any.
    pub(super) icon_drag: Option<IconId>,
}

impl Interaction {
    pub(super) fn new() -> Interaction {
        Interaction {
            panning: false,
            placing: None,
            placing_size: Vec2::ZERO,
            moving: None,
            selected: Vec::new(),
            selecting: None,
            icon_drag: None,
        }
    }

    pub(super) fn clear(&mut self) {
        self.moving = None;
        self.selected.clear();
        self.selecting = None;
        self.placing = None;
        self.icon_drag = None;
        self.panning = false;
    }

    /// Clears an open gesture. Returns whether one was open, so Escape is consumed.
    pub(super) fn cancel(&mut self) -> bool {
        let active = self.placing.is_some()
            || self.moving.is_some()
            || self.selecting.is_some()
            || !self.selected.is_empty()
            || self.icon_drag.is_some();
        if !active {
            return false;
        }
        self.placing = None;
        self.moving = None;
        self.selecting = None;
        self.selected.clear();
        self.icon_drag = None;
        true
    }

    pub(super) fn selected(&self) -> &[usize] {
        &self.selected
    }

    pub(super) fn select_rect(&self, pos: Pos2) -> Option<Rect> {
        self.selecting.map(|start| Rect::from_two_pos(start, pos))
    }

    pub(super) fn select_biome(&mut self, world: &World, biome: usize) {
        self.selected = world
            .rooms
            .iter()
            .enumerate()
            .filter(|(_, room)| room.biome == biome)
            .map(|(i, _)| i)
            .collect();
        self.moving = None;
        self.selecting = None;
    }

    /// Room dragged from the list. On the map it snaps to the cell grid, centered on
    /// the cursor cell. Releasing on a free spot places it. Releasing elsewhere cancels.
    pub(super) fn update_placing(
        &mut self,
        world: &mut World,
        view: &View,
        p: &mut Pointer,
        layout: &Layout,
        map: Rect,
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
        let (cx, cy) = layout.cell_at(view.screen_to_world(p.pos, map));
        let (x, y) = (cx - (choice.w / 2) as i32, cy - (choice.h / 2) as i32);
        let free = world.can_place(x, y, choice.w, choice.h);
        if p.left_down {
            return Some(Ghost { x, y, choice, free });
        }
        if free {
            let (kind, marker) = choice.pick.spec();
            world.place_manual((x, y), (choice.w, choice.h), kind, marker);
        }
        self.placing = None;
        None
    }

    /// Drags the selection rectangle on an empty cell. Releasing selects every room
    /// the rectangle cuts. A click without travel clears the selection.
    pub(super) fn update_selection(
        &mut self,
        p: &Pointer,
        view: &View,
        world: &World,
        layout: &Layout,
        map: Rect,
    ) {
        if p.right {
            self.selecting = None;
            return;
        }
        let Some(start) = self.selecting else {
            if p.left
                && p.on_map()
                && !p.consumed
                && !self.panning
                && self.placing.is_none()
                && self.moving.is_none()
                && self.icon_drag.is_none()
            {
                let (x, y) = layout.cell_at(view.screen_to_world(p.pos, map));
                if world.room_at(x, y).is_none() {
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
        world: &mut World,
        hovered: Option<usize>,
    ) {
        let Some(icon) = self.icon_drag else {
            if p.left && !p.consumed && self.placing.is_none() && self.moving.is_none() {
                self.icon_drag = legend.icon_at(p.pos);
            }
            return;
        };
        if p.right {
            self.icon_drag = None;
        } else if !p.left_down {
            if let Some(room) = hovered {
                tag_room(world, room, icon);
            }
            self.icon_drag = None;
        }
    }

    /// Deletes the selection, the dragged group, or the hovered room.
    /// At least one room stays. Returns how many rooms were removed.
    pub(super) fn delete_shortcut(
        &mut self,
        ctx: &Context,
        world: &mut World,
        blocked: bool,
        hovered: Option<usize>,
    ) -> usize {
        if blocked {
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
        } else if let Some(idx) = hovered {
            vec![idx]
        } else {
            return 0;
        };
        let removed = world.remove_rooms(&targets);
        if removed > 0 {
            self.moving = None;
            self.selected.clear();
        }
        removed
    }

    pub(super) fn room_shortcuts(
        &mut self,
        ctx: &Context,
        world: &mut World,
        blocked: bool,
        room: usize,
    ) {
        if blocked {
            return;
        }
        if ctx.input(|i| i.key_pressed(Key::B)) {
            tag_room(world, room, IconId::Boss);
        }
        if ctx.input(|i| i.key_pressed(Key::S) && !i.modifiers.command) {
            tag_room(world, room, IconId::Save);
        }
        if ctx.input(|i| i.key_pressed(Key::H)) {
            tag_room(world, room, IconId::Secret);
        }
        if ctx.input(|i| i.key_pressed(Key::P)) {
            tag_room(world, room, IconId::Portal);
        }
    }
}

fn tag_room(world: &mut World, room: usize, icon: IconId) {
    match icon {
        IconId::Boss => world.toggle_marker(room, Marker::Boss),
        IconId::Save => world.toggle_marker(room, Marker::Save),
        IconId::Secret => world.toggle_kind(room, RoomKind::Secret),
        IconId::Portal => world.toggle_kind(room, RoomKind::Portal),
    }
}
