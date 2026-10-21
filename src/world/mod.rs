//! Map data: rooms, doors and the grid of occupied cells.
//!
//! The map is an unbounded grid of cells. Rooms are axis-aligned rectangles of cells
//! that never overlap. Generation is in `generate`, manual edits are in `edit`.

mod edit;
mod generate;
#[cfg(test)]
mod tests;

pub use edit::{Choice, Pick};
pub use generate::{Params, time_seed};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Allowed sizes of each kind of room, as (width, height) in cells. None may be empty.
pub struct Sizes {
    pub rooms: Vec<(u32, u32)>,
    pub boss: Vec<(u32, u32)>,
    pub corridor: Vec<(u32, u32)>,
}

/// Special rooms and corridors of one biome.
#[derive(Clone, Copy)]
pub struct Extras {
    /// Corridors of the biome. The first one links it to its parent biome, the others
    /// join it to a room of another biome. The first biome has no parent, so it only
    /// gets the others.
    pub corridors: u32,
    /// 1x1 dead ends.
    pub secrets: u32,
    /// Rooms with one entrance and one exit, like corridors.
    pub bosses: u32,
    /// 1x1 dead ends with a save marker.
    pub saves: u32,
    /// 1x1 dead ends.
    pub portals: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoomKind {
    Normal,
    /// Long thin room joining two biomes. Belongs to the biome it starts from.
    Corridor,
    /// 1x1 dead end added after a biome is grown. Nothing is attached to it.
    Secret,
    /// 1x1 dead end, same as a secret room but with its own look.
    Portal,
}

/// Icon on a room. Placed by the generator, and toggled by the user.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Marker {
    Boss,
    Save,
}

/// Rectangle of cells. (x, y) is the top-left cell, w and h are sizes in cells.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Room {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    /// Index into the biome list.
    pub biome: usize,
    pub kind: RoomKind,
    pub marker: Option<Marker>,
}

/// Why `World::from_parts` rejected a saved map.
#[derive(Debug, PartialEq)]
pub enum MapError {
    /// No room, or no biome list to index into.
    Empty,
    /// A room has a side of zero cells.
    EmptyRoom,
    /// Two rooms cover the same cell.
    Overlap,
    /// A door or a biome index does not point at anything.
    Index,
}

/// Opening between rooms `a` and `b`, which are indices into `World::rooms`.
/// `vertical` means the door sits on the left edge of cell (x, y).
/// Otherwise it sits on the top edge of cell (x, y).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Door {
    pub a: usize,
    pub b: usize,
    pub x: i32,
    pub y: i32,
    pub vertical: bool,
}

#[derive(Default)]
pub struct World {
    /// Maps each occupied cell to the index of the room covering it.
    grid: HashMap<(i32, i32), usize>,
    pub rooms: Vec<Room>,
    pub doors: Vec<Door>,
    /// Biomes actually used, in placement order.
    pub biomes: Vec<usize>,
}

impl World {
    /// Returns (min_x, min_y, max_x, max_y) in cells, max excluded.
    pub fn bounds(&self) -> (i32, i32, i32, i32) {
        self.rooms
            .iter()
            .fold((i32::MAX, i32::MAX, i32::MIN, i32::MIN), |b, r| {
                (
                    b.0.min(r.x),
                    b.1.min(r.y),
                    b.2.max(r.x + r.w as i32),
                    b.3.max(r.y + r.h as i32),
                )
            })
    }

    /// Index of the room covering cell (x, y), if any.
    pub fn room_at(&self, x: i32, y: i32) -> Option<usize> {
        self.grid.get(&(x, y)).copied()
    }

    /// True if a w x h room with its top-left cell at (x, y) overlaps no room.
    pub fn can_place(&self, x: i32, y: i32, w: u32, h: u32) -> bool {
        self.is_free(x, y, w as i32, h as i32)
    }

    /// True if no cell of the rectangle is occupied.
    fn is_free(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
        (y..y + h).all(|cy| (x..x + w).all(|cx| !self.grid.contains_key(&(cx, cy))))
    }

    /// Adds a room without any door and marks its cells. Returns its index.
    fn place(&mut self, x: i32, y: i32, w: u32, h: u32, biome: usize, kind: RoomKind) -> usize {
        let idx = self.rooms.len();
        self.set_cells(x, y, w, h, Some(idx));
        self.rooms.push(Room {
            x,
            y,
            w,
            h,
            biome,
            kind,
            marker: None,
        });
        idx
    }

    /// Removes the last room and the door that attached it.
    fn remove_last(&mut self) {
        self.truncate(self.rooms.len() - 1, self.doors.len() - 1);
    }

    /// Rolls back to an earlier state by dropping the rooms and doors added after it.
    fn truncate(&mut self, rooms_len: usize, doors_len: usize) {
        while self.rooms.len() > rooms_len {
            let r = self.rooms.pop().unwrap();
            self.set_cells(r.x, r.y, r.w, r.h, None);
        }
        self.doors.truncate(doors_len);
    }

    /// Rebuilds the cell grid from a saved map. `biome_count` is the length of the biome list
    /// the room and `biomes` indices point into.
    pub fn from_parts(
        rooms: Vec<Room>,
        doors: Vec<Door>,
        biomes: Vec<usize>,
        biome_count: usize,
    ) -> Result<World, MapError> {
        if rooms.is_empty() || biome_count == 0 {
            return Err(MapError::Empty);
        }
        if biomes.is_empty() || biomes.iter().any(|biome| *biome >= biome_count) {
            return Err(MapError::Index);
        }
        for room in &rooms {
            if room.w == 0 || room.h == 0 {
                return Err(MapError::EmptyRoom);
            }
            if room.biome >= biome_count {
                return Err(MapError::Index);
            }
        }
        for door in &doors {
            if door.a >= rooms.len() || door.b >= rooms.len() {
                return Err(MapError::Index);
            }
        }
        let mut world = World {
            grid: HashMap::new(),
            rooms,
            doors,
            biomes,
        };
        for idx in 0..world.rooms.len() {
            let (x, y, w, h) = {
                let room = &world.rooms[idx];
                (room.x, room.y, room.w, room.h)
            };
            if !world.is_free(x, y, w as i32, h as i32) {
                return Err(MapError::Overlap);
            }
            world.set_cells(x, y, w, h, Some(idx));
        }
        Ok(world)
    }

    /// Marks the rectangle cells as owned by room `value`, or frees them with None.
    fn set_cells(&mut self, x: i32, y: i32, w: u32, h: u32, value: Option<usize>) {
        for cy in y..y + h as i32 {
            for cx in x..x + w as i32 {
                match value {
                    Some(idx) => self.grid.insert((cx, cy), idx),
                    None => self.grid.remove(&(cx, cy)),
                };
            }
        }
    }
}
