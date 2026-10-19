//! Procedural map generation.
//!
//! The map is an unbounded grid of cells. Rooms are axis-aligned rectangles of cells
//! that never overlap. Each new room is attached to an existing one through a door,
//! so the whole map is always a connected tree of rooms.
//!
//! Biomes are grown one after another: the first one starts at the origin, then each
//! next biome is linked to an already placed biome by a corridor.

use macroquad::rand::{ChooseRandom, gen_range, srand};
use std::collections::HashMap;

/// Average number of room cells per biome. Drives how many biomes a map uses.
const CELLS_PER_BIOME: u32 = 75;
/// Tries to build a corridor plus the first room of a new biome.
const LINK_ATTEMPTS: u32 = 200;
/// After this many tries, a biome is kept even if it stayed small.
const BIOME_ATTEMPTS: u32 = 5;
/// Tries to find a free spot for one secret room.
const SECRET_ATTEMPTS: u32 = 200;

/// Uniform in [low, high). `gen_range` truncates toward zero, which can return `high` for negative ranges.
fn range(low: i32, high: i32) -> i32 {
    low + gen_range(0, high - low)
}

#[derive(Clone, Copy, PartialEq)]
pub enum RoomKind {
    Normal,
    /// Long thin room joining two biomes. Belongs to the parent biome.
    Corridor,
    /// 1x1 dead end added after a biome is grown. Nothing is attached to it.
    Secret,
}

/// Icon placed by the user on a room.
#[derive(Clone, Copy, PartialEq)]
pub enum Marker {
    Boss,
    Save,
}

/// Rectangle of cells. (x, y) is the top-left cell, w and h are sizes in cells.
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

/// Opening between rooms `a` and `b`, which are indices into `World::rooms`.
/// `vertical` means the door sits on the left edge of cell (x, y).
/// Otherwise it sits on the top edge of cell (x, y).
pub struct Door {
    pub a: usize,
    pub b: usize,
    pub x: i32,
    pub y: i32,
    pub vertical: bool,
}

pub struct World {
    /// Maps each occupied cell to the index of the room covering it.
    grid: HashMap<(i32, i32), usize>,
    pub rooms: Vec<Room>,
    /// One door per room except the first, pushed in the same order as `rooms`.
    pub doors: Vec<Door>,
    /// Biomes actually used, in placement order.
    pub biomes: Vec<usize>,
    /// Allowed room sizes, as (width, height) in cells.
    sizes: Vec<(u32, u32)>,
}

impl World {
    /// `cells` is the target number of room cells. `secrets` holds the secret room count of
    /// each biome, so its length is the biome count. It and `min_rooms` must be at least 1,
    /// and `max_rooms` at least `min_rooms`. Every biome ends with that many normal rooms.
    /// `sizes` must not be empty.
    pub fn generate(
        cells: u32,
        seed: u64,
        secrets: &[u32],
        min_rooms: usize,
        max_rooms: usize,
        sizes: &[(u32, u32)],
    ) -> World {
        let biome_count = secrets.len();
        // Same seed and settings always give the same map.
        srand(seed);
        let mut world = World {
            grid: HashMap::new(),
            rooms: Vec::new(),
            doors: Vec::new(),
            biomes: Vec::new(),
            sizes: sizes.to_vec(),
        };

        // Pick which biomes appear and in which order. Each gets the same cell quota.
        let mut order: Vec<usize> = (0..biome_count).collect();
        order.shuffle();
        let count = (cells / CELLS_PER_BIOME).max(2).min(biome_count as u32) as usize;
        order.truncate(count);
        let quota = cells / count as u32;

        // First biome: one room at the origin, then grow. Restart from scratch until
        // it reaches the minimum room count.
        let root = order[0];
        loop {
            let (w, h) = *world.sizes.choose().unwrap();
            world.place(0, 0, w, h, root, RoomKind::Normal);
            if world.grow(root, quota, min_rooms, max_rooms).1 >= min_rooms {
                break;
            }
            world.truncate(0, 0);
        }
        world.biomes.push(root);
        world.add_secrets(root, secrets[root]);

        // Next biomes: link to a random placed biome, then grow. A failed try is rolled
        // back so the next one starts from a clean state.
        // The map is unbounded, so retrying always ends up finding free space.
        for &child in &order[1..] {
            for attempt in 0.. {
                let (rooms_len, doors_len) = (world.rooms.len(), world.doors.len());
                let parent = world.biomes[gen_range(0, world.biomes.len())];
                if !world.link(parent, child) {
                    continue;
                }
                let (cells, rooms) = world.grow(child, quota, min_rooms, max_rooms);
                // The minimum is mandatory. Size is only preferred: a small biome is
                // accepted once the early attempts are used up.
                if rooms >= min_rooms && (cells >= quota / 3 || attempt >= BIOME_ATTEMPTS - 1) {
                    world.biomes.push(child);
                    world.add_secrets(child, secrets[child]);
                    break;
                }
                world.truncate(rooms_len, doors_len);
            }
        }
        world
    }

    /// Returns (min_x, min_y, max_x, max_y) in cells, max excluded.
    pub fn bounds(&self) -> (i32, i32, i32, i32) {
        self.rooms.iter().fold((i32::MAX, i32::MAX, i32::MIN, i32::MIN), |b, r| {
            (
                b.0.min(r.x),
                b.1.min(r.y),
                b.2.max(r.x + r.w as i32),
                b.3.max(r.y + r.h as i32),
            )
        })
    }

    /// Indices of the non-corridor rooms of a biome.
    fn normal_rooms(&self, biome: usize) -> Vec<usize> {
        (0..self.rooms.len())
            .filter(|&i| self.rooms[i].biome == biome && self.rooms[i].kind == RoomKind::Normal)
            .collect()
    }

    /// Attaches random rooms to random rooms of the biome. Stops at `max_rooms`, or once
    /// both the cell quota and `min_rooms` are reached, or when the try budget runs out.
    /// Returns the biome cell count and normal room count.
    fn grow(&mut self, biome: usize, quota: u32, min_rooms: usize, max_rooms: usize) -> (u32, usize) {
        let mut rooms = self.normal_rooms(biome);
        let mut cells: u32 = rooms.iter().map(|&i| self.rooms[i].w * self.rooms[i].h).sum();
        for _ in 0..(quota + min_rooms as u32) * 30 {
            if rooms.len() >= max_rooms || (cells >= quota && rooms.len() >= min_rooms) {
                break;
            }
            let from = *rooms.choose().unwrap();
            let (w, h) = *self.sizes.choose().unwrap();
            let dir = gen_range(0, 4);
            if let Some(idx) = self.try_attach(from, dir, w, h, biome, RoomKind::Normal) {
                cells += w * h;
                rooms.push(idx);
            }
        }
        (cells, rooms.len())
    }

    /// Attaches `count` 1x1 secret rooms to random normal rooms of the biome.
    /// A secret is skipped if no free spot is found.
    fn add_secrets(&mut self, biome: usize, count: u32) {
        let rooms = self.normal_rooms(biome);
        for _ in 0..count {
            for _ in 0..SECRET_ATTEMPTS {
                let from = *rooms.choose().unwrap();
                if self.try_attach(from, gen_range(0, 4), 1, 1, biome, RoomKind::Secret).is_some() {
                    break;
                }
            }
        }
    }

    /// Builds a corridor from a parent biome room, then the first child room at its end.
    /// The corridor is removed if the child room does not fit.
    fn link(&mut self, parent: usize, child: usize) -> bool {
        for _ in 0..LINK_ATTEMPTS {
            let from = *self.normal_rooms(parent).choose().unwrap();
            let dir = gen_range(0, 4);
            let len = gen_range(3, 6);
            // Horizontal corridor for left/right, vertical for up/down.
            let (cw, ch) = if dir < 2 { (len, 1) } else { (1, len) };
            let Some(corridor) = self.try_attach(from, dir, cw, ch, parent, RoomKind::Corridor) else {
                continue;
            };
            let (w, h) = *self.sizes.choose().unwrap();
            if self.try_attach(corridor, dir, w, h, child, RoomKind::Normal).is_some() {
                return true;
            }
            self.remove_last();
        }
        false
    }

    /// Places a w x h room against one side of room `from`, plus a door between them.
    /// Returns the new room index, or None if the spot is taken.
    /// dir: 0 right, 1 left, 2 down, 3 up.
    fn try_attach(
        &mut self,
        from: usize,
        dir: u32,
        w: u32,
        h: u32,
        biome: usize,
        kind: RoomKind,
    ) -> Option<usize> {
        let r = &self.rooms[from];
        let (rx, ry, rw, rh) = (r.x, r.y, r.w as i32, r.h as i32);
        let (wi, hi) = (w as i32, h as i32);

        // (nx, ny) is the new room top-left, (dx, dy) the door cell.
        // The new room slides along the shared side so that both rooms overlap on at
        // least one cell. The door is picked inside that overlap.
        let (nx, ny, dx, dy, vertical) = if dir < 2 {
            let ny = range(ry - hi + 1, ry + rh);
            let dy = range(ry.max(ny), (ry + rh).min(ny + hi));
            if dir == 0 {
                (rx + rw, ny, rx + rw, dy, true)
            } else {
                (rx - wi, ny, rx, dy, true)
            }
        } else {
            let nx = range(rx - wi + 1, rx + rw);
            let dx = range(rx.max(nx), (rx + rw).min(nx + wi));
            if dir == 2 {
                (nx, ry + rh, dx, ry + rh, false)
            } else {
                (nx, ry - hi, dx, ry, false)
            }
        };

        if !self.is_free(nx, ny, wi, hi) {
            return None;
        }
        let idx = self.place(nx, ny, w, h, biome, kind);
        self.doors.push(Door {
            a: from,
            b: idx,
            x: dx,
            y: dy,
            vertical,
        });
        Some(idx)
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

    /// Sets the marker, or clears it if the room already has this one.
    pub fn toggle_marker(&mut self, idx: usize, marker: Marker) {
        let room = &mut self.rooms[idx];
        room.marker = if room.marker == Some(marker) { None } else { Some(marker) };
    }

    /// Index of the room covering cell (x, y), if any.
    pub fn room_at(&self, x: i32, y: i32) -> Option<usize> {
        self.grid.get(&(x, y)).copied()
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
