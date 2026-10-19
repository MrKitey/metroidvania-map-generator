//! Edits made by the user on a generated map.

use super::{Door, Marker, RoomKind, World};
use std::collections::BTreeMap;

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
        // Per side (right, left, down, up): each touching room with the door cells it
        // could use. The door cell is on the new room for left and up, on the neighbor
        // for right and down, as `Door` expects.
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
        let biome = sides
            .iter()
            .flat_map(|s| s.keys())
            .next()
            .map_or(self.biomes[0], |&n| self.rooms[n].biome);

        let idx = self.place(x, y, w, h, biome, kind);
        self.rooms[idx].marker = marker;
        for (side, neighbors) in sides.iter().enumerate() {
            for (&n, cells) in neighbors {
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
        Some(idx)
    }
}
