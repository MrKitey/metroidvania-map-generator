//! Edits made by the user on a generated map.

use super::{Door, Marker, Room, RoomKind, World};
use std::collections::{BTreeMap, HashSet};

/// Kind of room offered by the room picker.
#[derive(Clone, Copy)]
pub enum Pick {
    Room,
    Boss,
    Corridor,
}

impl Pick {
    /// Room kind and marker of a room placed from this entry.
    pub fn spec(self) -> (RoomKind, Option<Marker>) {
        match self {
            Pick::Room => (RoomKind::Normal, None),
            Pick::Boss => (RoomKind::Normal, Some(Marker::Boss)),
            Pick::Corridor => (RoomKind::Corridor, None),
        }
    }
}

/// One entry of the room picker: a kind and a size in cells.
#[derive(Clone, Copy)]
pub struct Choice {
    pub pick: Pick,
    pub w: u32,
    pub h: u32,
}

impl World {
    /// Sets the marker, or clears it if the room already has this one.
    /// A new marker replaces the secret or portal kind.
    pub fn toggle_marker(&mut self, idx: usize, marker: Marker) {
        let room = &mut self.rooms[idx];
        if room.marker == Some(marker) {
            room.marker = None;
            return;
        }
        room.marker = Some(marker);
        if matches!(room.kind, RoomKind::Secret | RoomKind::Portal) {
            room.kind = RoomKind::Normal;
        }
    }

    /// Sets the kind (secret or portal), or goes back to normal if the room already has it.
    /// A new kind replaces the marker. Corridors keep their kind.
    pub fn toggle_kind(&mut self, idx: usize, kind: RoomKind) {
        let room = &mut self.rooms[idx];
        if room.kind == RoomKind::Corridor {
            return;
        }
        if room.kind == kind {
            room.kind = RoomKind::Normal;
        } else {
            room.kind = kind;
            room.marker = None;
        }
    }

    /// Places a room chosen by the user. It gets one door per touching room, in the middle
    /// of the shared side, and the biome of the first touching room. With no neighbor it
    /// gets no door and the first biome. Returns None if the spot is taken.
    pub fn place_manual(
        &mut self,
        (x, y): (i32, i32),
        (w, h): (u32, u32),
        kind: RoomKind,
        marker: Option<Marker>,
    ) -> Option<usize> {
        let (wi, hi) = (w as i32, h as i32);
        if !self.is_free(x, y, wi, hi) {
            return None;
        }
        let sides = self.touching(x, y, w, h);
        let biome = self.neighbor_biome(&sides).unwrap_or(self.biomes[0]);

        let idx = self.place(x, y, w, h, biome, kind);
        self.rooms[idx].marker = marker;
        self.add_doors(idx, &sides, None);
        Some(idx)
    }

    /// True if room `idx` can have its top-left cell at (x, y). Its own cells count as free.
    pub fn can_move(&self, idx: usize, x: i32, y: i32) -> bool {
        self.can_move_rooms(&[(idx, x, y)])
    }

    /// True if these rooms can move together. Each entry is an index and its new top-left.
    /// A cell is free when it is empty or held by one of these rooms.
    /// Two rooms of the group cannot land on the same cell.
    pub fn can_move_rooms(&self, moves: &[(usize, i32, i32)]) -> bool {
        if moves.is_empty() {
            return false;
        }
        let mut group = HashSet::new();
        for &(idx, _, _) in moves {
            if idx >= self.rooms.len() || !group.insert(idx) {
                return false;
            }
        }
        let mut taken = HashSet::new();
        for &(idx, x, y) in moves {
            let r = &self.rooms[idx];
            for cy in y..y + r.h as i32 {
                for cx in x..x + r.w as i32 {
                    if !taken.insert((cx, cy)) {
                        return false;
                    }
                    if self.room_at(cx, cy).is_some_and(|i| !group.contains(&i)) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Moves room `idx` so its top-left cell is at (x, y). Its doors are removed, then it
    /// gets one door per touching room like `place_manual`, and the biome of the first
    /// touching room. With no neighbor it keeps its biome. Its kind is kept.
    /// Returns false, and changes nothing, if the spot is taken.
    pub fn move_room(&mut self, idx: usize, x: i32, y: i32) -> bool {
        self.move_rooms(&[(idx, x, y)])
    }

    /// Moves several rooms at once. Each entry is a room index and its new top-left cell.
    /// Doors of the group are rebuilt once, so two moved rooms share a single door.
    /// A room takes the biome of the first neighbor outside the group, or keeps its own.
    /// Returns false, and changes nothing, if a spot is taken or two rooms overlap.
    pub fn move_rooms(&mut self, moves: &[(usize, i32, i32)]) -> bool {
        if !self.can_move_rooms(moves) {
            return false;
        }
        let group: HashSet<usize> = moves.iter().map(|&(idx, _, _)| idx).collect();
        for &(idx, _, _) in moves {
            let Room { x, y, w, h, .. } = self.rooms[idx];
            self.set_cells(x, y, w, h, None);
        }
        self.doors
            .retain(|d| !group.contains(&d.a) && !group.contains(&d.b));
        for &(idx, x, y) in moves {
            let room = &mut self.rooms[idx];
            let (w, h) = (room.w, room.h);
            room.x = x;
            room.y = y;
            self.set_cells(x, y, w, h, Some(idx));
        }
        let planned: Vec<_> = moves
            .iter()
            .map(|&(idx, x, y)| {
                let sides = self.touching(x, y, self.rooms[idx].w, self.rooms[idx].h);
                let biome = sides
                    .iter()
                    .flat_map(|side| side.keys())
                    .find(|n| !group.contains(*n))
                    .map(|n| self.rooms[*n].biome);
                (idx, biome, sides)
            })
            .collect();
        for (idx, biome, sides) in planned {
            if let Some(biome) = biome {
                self.rooms[idx].biome = biome;
            }
            self.add_doors(idx, &sides, Some(&group));
        }
        true
    }

    /// Removes room `idx` and the doors that touched it.
    /// Rooms after it move down, and door indices follow.
    /// Returns false, and changes nothing, if `idx` is out of range or it is the last room.
    pub fn remove_room(&mut self, idx: usize) -> bool {
        if self.rooms.len() <= 1 || idx >= self.rooms.len() {
            return false;
        }
        let room = self.rooms.remove(idx);
        self.set_cells(room.x, room.y, room.w, room.h, None);
        self.doors.retain(|d| d.a != idx && d.b != idx);
        for door in &mut self.doors {
            if door.a > idx {
                door.a -= 1;
            }
            if door.b > idx {
                door.b -= 1;
            }
        }
        for cell in self.grid.values_mut() {
            if *cell > idx {
                *cell -= 1;
            }
        }
        true
    }

    /// Per side (right, left, down, up): each room touching the rectangle, with the door
    /// cells it could use. The door cell is on the rectangle for left and up, on the
    /// neighbor for right and down, as `Door` expects.
    fn touching(&self, x: i32, y: i32, w: u32, h: u32) -> [BTreeMap<usize, Vec<(i32, i32)>>; 4] {
        let (wi, hi) = (w as i32, h as i32);
        let mut sides: [BTreeMap<usize, Vec<(i32, i32)>>; 4] = Default::default();
        for i in 0..hi {
            if let Some(n) = self.room_at(x + wi, y + i) {
                sides[0].entry(n).or_default().push((x + wi, y + i));
            }
            if let Some(n) = self.room_at(x - 1, y + i) {
                sides[1].entry(n).or_default().push((x, y + i));
            }
        }
        for i in 0..wi {
            if let Some(n) = self.room_at(x + i, y + hi) {
                sides[2].entry(n).or_default().push((x + i, y + hi));
            }
            if let Some(n) = self.room_at(x + i, y - 1) {
                sides[3].entry(n).or_default().push((x + i, y));
            }
        }
        sides
    }

    /// Biome of the first touching room, if any.
    fn neighbor_biome(&self, sides: &[BTreeMap<usize, Vec<(i32, i32)>>; 4]) -> Option<usize> {
        let n = sides.iter().flat_map(|s| s.keys()).next()?;
        Some(self.rooms[*n].biome)
    }

    /// Adds one door between room `idx` and each touching room, in the middle of the
    /// shared side. When `group` is set, a pair of moved rooms gets one door only:
    /// the lower index creates it.
    fn add_doors(
        &mut self,
        idx: usize,
        sides: &[BTreeMap<usize, Vec<(i32, i32)>>; 4],
        group: Option<&HashSet<usize>>,
    ) {
        for (side, neighbors) in sides.iter().enumerate() {
            for (&n, cells) in neighbors {
                if group.is_some_and(|g| g.contains(&n) && n < idx) {
                    continue;
                }
                let (dx, dy) = cells[cells.len() / 2];
                self.doors.push(Door {
                    a: n,
                    b: idx,
                    x: dx,
                    y: dy,
                    vertical: side < 2,
                });
            }
        }
    }
}
