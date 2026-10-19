//! Procedural map generation.
//!
//! The map is an unbounded grid of cells. Rooms are axis-aligned rectangles of cells
//! that never overlap. Each new room is attached to an existing one through a door,
//! so the whole map is always a connected tree of rooms.
//!
//! Biomes are grown one after another: the first one starts at the origin, then each
//! next biome is linked to an already placed biome by a corridor.
//!
//! Once every biome is placed, extra corridors can join two biomes that face each other.
//! They create loops, so the map is a tree only before that step.

use macroquad::rand::{ChooseRandom, gen_range, srand};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

/// Average number of room cells per biome. Drives how many biomes a map uses.
const CELLS_PER_BIOME: u32 = 75;
/// Tries to build a corridor plus the first room of a new biome.
const LINK_ATTEMPTS: u32 = 200;
/// After this many tries, a biome is kept even if it stayed small.
const BIOME_ATTEMPTS: u32 = 5;
/// Tries to find a free spot for one secret, save or boss room.
const EXTRA_ATTEMPTS: u32 = 200;
/// Cells kept free of other corridors around a corridor, diagonals included.
const CORRIDOR_SPACING: i32 = 2;
/// Longest gap, in cells, that an extra corridor can cross.
const MAX_BRIDGE: i32 = 10;

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

/// Top-left cell and size of an extra corridor of `len` cells whose first cell is (x, y),
/// going in direction `dir` (0 right, 1 left, 2 down, 3 up).
fn bridge_rect(dir: i32, x: i32, y: i32, len: i32) -> (i32, i32, i32, i32) {
    match dir {
        0 => (x, y, len, 1),
        1 => (x - len + 1, y, len, 1),
        2 => (x, y, 1, len),
        _ => (x, y - len + 1, 1, len),
    }
}

/// Uniform in [low, high). `gen_range` truncates toward zero, which can return `high` for negative ranges.
fn range(low: i32, high: i32) -> i32 {
    low + gen_range(0, high - low)
}

#[derive(Clone, Copy, PartialEq)]
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
    pub doors: Vec<Door>,
    /// Biomes actually used, in placement order.
    pub biomes: Vec<usize>,
    sizes: Sizes,
}

impl World {
    /// `cells` is the target number of room cells. `extras` holds the special rooms of
    /// each biome, so its length is the biome count. It and `min_rooms` must be at least 1,
    /// and `max_rooms` at least `min_rooms`. Every biome ends with that many normal rooms,
    /// not counting the special rooms and the room behind each boss.
    pub fn generate(
        cells: u32,
        seed: u64,
        extras: &[Extras],
        min_rooms: usize,
        max_rooms: usize,
        sizes: Sizes,
    ) -> World {
        let biome_count = extras.len();
        // Same seed and settings always give the same map.
        srand(seed);
        let mut world = World {
            grid: HashMap::new(),
            rooms: Vec::new(),
            doors: Vec::new(),
            biomes: Vec::new(),
            sizes,
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
            let (w, h) = *world.sizes.rooms.choose().unwrap();
            world.place(0, 0, w, h, root, RoomKind::Normal);
            if world.grow(root, quota, min_rooms, max_rooms).1 >= min_rooms {
                break;
            }
            world.truncate(0, 0);
        }
        world.biomes.push(root);
        world.add_extras(root, extras[root]);

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
                    world.add_extras(child, extras[child]);
                    break;
                }
                world.truncate(rooms_len, doors_len);
            }
        }

        // Extra corridors come last, once every biome has its neighbors.
        for i in 0..world.biomes.len() {
            let biome = world.biomes[i];
            for _ in 1..extras[biome].corridors {
                world.add_bridge(biome);
            }
        }
        world
    }

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

    /// Indices of the rooms of a biome that can receive new rooms. Corridors, secrets and
    /// rooms with a marker are left out, so their door count never changes.
    fn normal_rooms(&self, biome: usize) -> Vec<usize> {
        (0..self.rooms.len())
            .filter(|&i| {
                let r = &self.rooms[i];
                r.biome == biome && r.kind == RoomKind::Normal && r.marker.is_none()
            })
            .collect()
    }

    /// Attaches random rooms to random rooms of the biome. Stops at `max_rooms`, or once
    /// both the cell quota and `min_rooms` are reached, or when the try budget runs out.
    /// Returns the biome cell count and normal room count.
    fn grow(
        &mut self,
        biome: usize,
        quota: u32,
        min_rooms: usize,
        max_rooms: usize,
    ) -> (u32, usize) {
        let mut rooms = self.normal_rooms(biome);
        let mut cells: u32 = rooms
            .iter()
            .map(|&i| self.rooms[i].w * self.rooms[i].h)
            .sum();
        for _ in 0..(quota + min_rooms as u32) * 30 {
            if rooms.len() >= max_rooms || (cells >= quota && rooms.len() >= min_rooms) {
                break;
            }
            let from = *rooms.choose().unwrap();
            let (w, h) = *self.sizes.rooms.choose().unwrap();
            let dir = gen_range(0, 4);
            if let Some(idx) = self.try_attach(from, dir, w, h, biome, RoomKind::Normal) {
                cells += w * h;
                rooms.push(idx);
            }
        }
        (cells, rooms.len())
    }

    /// Adds the special rooms of a biome. Bosses go first because they need the most space.
    /// A room is skipped if no free spot is found.
    fn add_extras(&mut self, biome: usize, extras: Extras) {
        for _ in 0..extras.bosses {
            self.add_boss(biome);
        }
        for _ in 0..extras.saves {
            self.add_dead_end(biome, RoomKind::Normal, Some(Marker::Save));
        }
        for _ in 0..extras.secrets {
            self.add_dead_end(biome, RoomKind::Secret, None);
        }
        for _ in 0..extras.portals {
            self.add_dead_end(biome, RoomKind::Portal, None);
        }
    }

    /// Attaches a 1x1 room to a random normal room of the biome. Nothing is attached to it.
    fn add_dead_end(&mut self, biome: usize, kind: RoomKind, marker: Option<Marker>) {
        let rooms = self.normal_rooms(biome);
        for _ in 0..EXTRA_ATTEMPTS {
            let from = *rooms.choose().unwrap();
            if let Some(idx) = self.try_attach(from, gen_range(0, 4), 1, 1, biome, kind) {
                self.rooms[idx].marker = marker;
                return;
            }
        }
    }

    /// Attaches a boss room to a random normal room of the biome, then a normal room
    /// behind it. The boss room has one entrance and one exit. Both rooms are removed
    /// if the second one does not fit.
    fn add_boss(&mut self, biome: usize) {
        let rooms = self.normal_rooms(biome);
        for _ in 0..EXTRA_ATTEMPTS {
            let from = *rooms.choose().unwrap();
            let (w, h) = *self.sizes.boss.choose().unwrap();
            let Some(boss) = self.try_attach(from, gen_range(0, 4), w, h, biome, RoomKind::Normal)
            else {
                continue;
            };
            let (w, h) = *self.sizes.rooms.choose().unwrap();
            if self
                .try_attach(boss, gen_range(0, 4), w, h, biome, RoomKind::Normal)
                .is_some()
            {
                self.rooms[boss].marker = Some(Marker::Boss);
                return;
            }
            self.remove_last();
        }
    }

    /// Builds a corridor from a parent biome room, then the first child room at its end.
    /// The corridor is removed if the child room does not fit.
    fn link(&mut self, parent: usize, child: usize) -> bool {
        for _ in 0..LINK_ATTEMPTS {
            let from = *self.normal_rooms(parent).choose().unwrap();
            let (cw, ch) = *self.sizes.corridor.choose().unwrap();
            // A wide corridor goes left or right, a tall one goes up or down.
            let dir = match cw.cmp(&ch) {
                Ordering::Greater => gen_range(0, 2),
                Ordering::Less => gen_range(2, 4),
                Ordering::Equal => gen_range(0, 4),
            };
            let Some(corridor) = self.try_attach(from, dir, cw, ch, parent, RoomKind::Corridor)
            else {
                continue;
            };
            let c = &self.rooms[corridor];
            if self.corridor_near(c.x, c.y, c.w as i32, c.h as i32, Some(corridor)) {
                self.remove_last();
                continue;
            }
            let (w, h) = *self.sizes.rooms.choose().unwrap();
            if self
                .try_attach(corridor, dir, w, h, child, RoomKind::Normal)
                .is_some()
            {
                return true;
            }
            self.remove_last();
        }
        false
    }

    /// True if another corridor lies within `CORRIDOR_SPACING` cells of the rectangle,
    /// diagonals included. `except` is the corridor that owns the rectangle, if placed.
    fn corridor_near(&self, x: i32, y: i32, w: i32, h: i32, except: Option<usize>) -> bool {
        let s = CORRIDOR_SPACING;
        (y - s..y + h + s).any(|cy| {
            (x - s..x + w + s).any(|cx| {
                self.room_at(cx, cy)
                    .is_some_and(|i| Some(i) != except && self.rooms[i].kind == RoomKind::Corridor)
            })
        })
    }

    /// Builds a corridor in a straight line from a normal room of the biome to a normal
    /// room of another biome, with a door at each end. The corridor must fit a size of
    /// the corridor list exactly, 1 cell thick. Nothing is built if no spot exists.
    fn add_bridge(&mut self, biome: usize) {
        // (from, dir, x, y, len, to): (x, y) is the first corridor cell next to `from`.
        let mut spots = Vec::new();
        for from in self.normal_rooms(biome) {
            let r = &self.rooms[from];
            let (rx, ry, rw, rh) = (r.x, r.y, r.w as i32, r.h as i32);
            for dir in 0..4 {
                let side = if dir < 2 { rh } else { rw };
                for i in 0..side {
                    let (x, y, dx, dy) = match dir {
                        0 => (rx + rw, ry + i, 1, 0),
                        1 => (rx - 1, ry + i, -1, 0),
                        2 => (rx + i, ry + rh, 0, 1),
                        _ => (rx + i, ry - 1, 0, -1),
                    };
                    // Walk over free cells until a room is hit.
                    let Some((len, to)) = (0..=MAX_BRIDGE)
                        .find_map(|k| Some((k, self.room_at(x + dx * k, y + dy * k)?)))
                    else {
                        continue;
                    };
                    let size = if dir < 2 {
                        (len as u32, 1)
                    } else {
                        (1, len as u32)
                    };
                    let target = &self.rooms[to];
                    let (cx, cy, cw, ch) = bridge_rect(dir, x, y, len);
                    if len > 0
                        && target.biome != biome
                        && target.kind == RoomKind::Normal
                        && target.marker.is_none()
                        && self.sizes.corridor.contains(&size)
                        && !self.corridor_near(cx, cy, cw, ch, None)
                    {
                        spots.push((from, dir, x, y, len, to));
                    }
                }
            }
        }
        if spots.is_empty() {
            return;
        }
        let (from, dir, x, y, len, to) = spots[gen_range(0, spots.len())];
        // Corridor top-left cell and size, then the door cells at both ends.
        let (cx, cy, w, h) = bridge_rect(dir, x, y, len);
        let ((x1, y1), (x2, y2), vertical) = match dir {
            0 => ((x, y), (x + len, y), true),
            1 => ((x + 1, y), (x - len + 1, y), true),
            2 => ((x, y), (x, y + len), false),
            _ => ((x, y + 1), (x, y - len + 1), false),
        };
        let idx = self.place(cx, cy, w as u32, h as u32, biome, RoomKind::Corridor);
        self.doors.push(Door {
            a: from,
            b: idx,
            x: x1,
            y: y1,
            vertical,
        });
        self.doors.push(Door {
            a: idx,
            b: to,
            x: x2,
            y: y2,
            vertical,
        });
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

    /// True if a w x h room with its top-left cell at (x, y) overlaps no room.
    pub fn can_place(&self, x: i32, y: i32, w: u32, h: u32) -> bool {
        self.is_free(x, y, w as i32, h as i32)
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
