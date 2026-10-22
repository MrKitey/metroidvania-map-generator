//! Dragging placed rooms, and the cursor that follows the gesture.

use super::super::{UndoEntry, remember};
use super::{Interaction, Moving, Pointer};
use crate::biome::Biome;
use crate::layout::Layout;
use crate::render::{Ghost, Picker};
use crate::view::View;
use crate::world::{Choice, Pick, World};
use egui::{Context, CursorIcon, Rect};

impl Interaction {
    /// Drags placed rooms with the left button. A selected room drags the whole
    /// selection. Another room is dragged alone, and the selection is cleared.
    /// Releasing on the map drops them. A taken spot, Escape or a right click
    /// leaves them in place. A drag of no cells changes nothing.
    pub(in super::super) fn update_moving(
        &mut self,
        p: &Pointer,
        hovered: Option<usize>,
        world: &mut World,
        view: &View,
        layout: &Layout,
        map: Rect,
        undo: &mut Vec<UndoEntry>,
        seed: u64,
        cells: u32,
        biomes: &[Biome],
    ) -> Vec<Ghost> {
        let cell = layout.cell_at(view.screen_to_world(p.pos, map));
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
        if p.left_down {
            let free = match moves.as_slice() {
                [(idx, x, y)] => world.can_move(*idx, *x, *y),
                _ => world.can_move_rooms(&moves),
            };
            return moves
                .into_iter()
                .map(|(idx, x, y)| {
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
        }
        if p.on_map() && delta != (0, 0) {
            let free = match moves.as_slice() {
                [(idx, x, y)] => world.can_move(*idx, *x, *y),
                _ => world.can_move_rooms(&moves),
            };
            if free {
                remember(undo, world, seed, cells, biomes);
                match moves.as_slice() {
                    [(idx, x, y)] => {
                        world.move_room(*idx, *x, *y);
                    }
                    _ => {
                        world.move_rooms(&moves);
                    }
                }
            }
        }
        self.moving = None;
        Vec::new()
    }

    pub(in super::super) fn update_cursor(
        &self,
        ctx: &Context,
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
                CursorIcon::Grab
            }
        } else if self.panning {
            CursorIcon::Move
        } else if picker.is_some_and(|ui| ui.items.iter().any(|(_, r)| r.contains(p.pos))) {
            CursorIcon::PointingHand
        } else if p.over_seed || p.over_legend {
            CursorIcon::PointingHand
        } else if hovered.is_some() {
            CursorIcon::Grab
        } else {
            CursorIcon::Default
        };
        ctx.set_cursor_icon(icon);
    }
}
