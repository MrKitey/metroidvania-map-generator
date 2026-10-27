//! Dragging placed rooms, whole layers, and the cursor that follows the gesture.

use super::super::History;
use super::{Interaction, Moving, Pointer};
use crate::layer::LayerStack;
use crate::render::{Ghost, Picker};
use crate::world::{Choice, Pick};
use egui::{Context, CursorIcon};

impl Interaction {
    /// Drags placed rooms with the left button. A selected room drags the whole
    /// selection. Another room is dragged alone, and the selection is cleared.
    /// Releasing on the map drops them. A taken spot, Escape or a right click
    /// leaves them in place. A drag of no cells changes nothing.
    pub(in super::super) fn update_moving(
        &mut self,
        p: &Pointer,
        hovered: Option<(usize, usize)>,
        layers: &mut LayerStack,
        cell: (i32, i32),
        history: &mut History,
    ) -> Vec<Ghost> {
        if self.moving_layer.is_some() {
            return Vec::new();
        }
        let Some(m) = &self.moving else {
            if self.paint_biome.is_some() {
                return Vec::new();
            }
            if let (true, Some((layer, room))) = (p.left, hovered)
                && self.selecting.is_none()
                && !p.consumed
            {
                if layer != layers.active() || layers.is_active_locked() {
                    return Vec::new();
                }
                let world = layers.active_world();
                let rooms = if self.selected.contains(&room) {
                    self.selected.clone()
                } else {
                    self.selected.clear();
                    vec![room]
                };
                let origins = rooms
                    .iter()
                    .map(|&i| {
                        let r = &world.rooms[i];
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
        let active = layers.active();
        drag_rooms(p, layers, active, &moves, delta, history).unwrap_or_else(|| {
            self.moving = None;
            Vec::new()
        })
    }

    /// Drags every room of a layer when the name label was grabbed.
    pub(in super::super) fn update_layer_moving(
        &mut self,
        p: &Pointer,
        layers: &mut LayerStack,
        cell: (i32, i32),
        history: &mut History,
    ) -> Vec<Ghost> {
        let Some(m) = &self.moving_layer else {
            return Vec::new();
        };
        let layer_idx = m.layer;
        if layers.layers()[layer_idx].locked {
            self.moving_layer = None;
            return Vec::new();
        }
        let delta = (cell.0 - m.anchor.0, cell.1 - m.anchor.1);
        let moves: Vec<_> = (0..m.origins.len())
            .zip(&m.origins)
            .map(|(idx, &(x, y))| (idx, x + delta.0, y + delta.1))
            .collect();
        if p.right {
            self.moving_layer = None;
            return Vec::new();
        }
        drag_rooms(p, layers, layer_idx, &moves, delta, history).unwrap_or_else(|| {
            self.moving_layer = None;
            Vec::new()
        })
    }

    pub(in super::super) fn update_cursor(
        &self,
        ctx: &Context,
        p: &Pointer,
        picker: Option<&Picker>,
        ghost: Option<&Ghost>,
        move_ghosts: &[Ghost],
        hovered: Option<(usize, usize)>,
    ) {
        let moving = self.moving.is_some() || self.moving_layer.is_some();
        let blocked = ghost.is_some_and(|g| !g.free) || move_ghosts.iter().any(|g| !g.free);
        let icon = if self.paint_biome.is_some() {
            CursorIcon::Cell
        } else if self.selecting.is_some() {
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
                CursorIcon::Grab
            }
        } else if self.panning {
            CursorIcon::Move
        } else if p.over_seed
            || p.over_legend
            || picker.is_some_and(|ui| ui.items.iter().any(|(_, r)| r.contains(p.pos)))
        {
            CursorIcon::PointingHand
        } else if hovered.is_some() {
            CursorIcon::Grab
        } else if !p.on_map_rect {
            // Leave the cursor to egui widgets in the header and side panels.
            return;
        } else {
            CursorIcon::Default
        };
        ctx.set_cursor_icon(icon);
    }
}

/// Rooms of `layer` moved by `delta`, as (index, new top-left) in `moves`.
/// While the button is held, returns their ghosts, red when the spot is taken.
/// On release over the map, moves them if they fit, after an undo snapshot,
/// and returns None so the caller ends the gesture.
fn drag_rooms(
    p: &Pointer,
    layers: &mut LayerStack,
    layer: usize,
    moves: &[(usize, i32, i32)],
    delta: (i32, i32),
    history: &mut History,
) -> Option<Vec<Ghost>> {
    let world = &layers.layers()[layer].world;
    if p.left_down {
        let free = world.can_move_rooms(moves);
        let ghosts = moves
            .iter()
            .map(|&(idx, x, y)| {
                let r = &world.rooms[idx];
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
        return Some(ghosts);
    }
    if p.on_map() && delta != (0, 0) && world.can_move_rooms(moves) {
        history.remember(layers);
        layers.layers_mut()[layer].world.move_rooms(moves);
    }
    None
}
