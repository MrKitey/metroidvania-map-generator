//! Procedural map generation.
//!
//! Each new room is attached to an existing one through a door, so the whole map is
//! always a connected tree of rooms.
//!
//! Biomes are grown one after another: the first one starts at the origin, then each
//! next biome is linked to an already placed biome by a corridor.
//!
//! Once every biome is placed, extra corridors can join two biomes that face each other.
//! They create loops, so the map is a tree only before that step.

use super::{Door, Extras, Marker, RoomKind, Sizes, World};
use fastrand::Rng;
use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

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

/// Inputs of a generation. The same params always give the same map.
pub struct Params<'a> {
    /// Target number of room cells.
    pub cells: u32,
    pub seed: u64,
    /// Special rooms of each biome, so its length is the biome count. Must not be empty.
    pub extras: &'a [Extras],
    /// Normal rooms per biome, not counting the special rooms and the room behind each
    /// boss. `min_rooms` must be at least 1, and `max_rooms` at least `min_rooms`.
    pub min_rooms: usize,
    pub max_rooms: usize,
    pub sizes: &'a Sizes,
}

/// Side of a room that a new room is attached to.
#[derive(Clone, Copy, PartialEq)]
enum Dir {
    Right,
    Left,
    Down,
    Up,
}

impl Dir {
    const ALL: [Dir; 4] = [Dir::Right, Dir::Left, Dir::Down, Dir::Up];

    /// Random side among `ALL[low..high]`.
    fn random(rng: &mut Rng, low: usize, high: usize) -> Dir {
        Dir::ALL[rng.usize(low..high)]
    }

    fn is_horizontal(self) -> bool {
        matches!(self, Dir::Right | Dir::Left)
    }

    /// Cell offset when moving one cell this way.
    fn step(self) -> (i32, i32) {
        match self {
            Dir::Right => (1, 0),
            Dir::Left => (-1, 0),
            Dir::Down => (0, 1),
            Dir::Up => (0, -1),
        }
    }
}

/// Top-left cell and size of an extra corridor of `len` cells whose first cell is (x, y),
/// going in direction `dir`.
fn bridge_rect(dir: Dir, x: i32, y: i32, len: i32) -> (i32, i32, i32, i32) {
    match dir {
        Dir::Right => (x, y, len, 1),
        Dir::Left => (x - len + 1, y, len, 1),
        Dir::Down => (x, y, 1, len),
        Dir::Up => (x, y - len + 1, 1, len),
    }
}

/// Uniform in [low, high).
fn range(rng: &mut Rng, low: i32, high: i32) -> i32 {
    rng.i32(low..high)
}

/// Random item of a non-empty slice.
fn pick<'a, T>(rng: &mut Rng, items: &'a [T]) -> &'a T {
    &items[rng.usize(..items.len())]
}

/// New seed from the clock, so each regeneration gives a different map.
pub fn time_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1)
}

impl World {
    pub fn generate(params: &Params) -> World {
        // Same seed and settings always give the same map.
        Generator {
            world: World::default(),
            p: params,
            rng: Rng::with_seed(params.seed),
        }
        .run()
    }
}

struct Generator<'a> {
    world: World,
    p: &'a Params<'a>,
    rng: Rng,
}

impl Generator<'_> {
    fn run(mut self) -> World {
        let p = self.p;
        let biome_count = p.extras.len();

        // Pick which biomes appear and in which order. Each gets the same cell quota.
        let mut order: Vec<usize> = (0..biome_count).collect();
        self.rng.shuffle(&mut order);
        let count = (p.cells / CELLS_PER_BIOME).max(2).min(biome_count as u32) as usize;
        order.truncate(count);
        let quota = p.cells / count as u32;

        // First biome: one room at the origin, then grow. Restart from scratch until
        // it reaches the minimum room count.
        let root = order[0];
        loop {
            let (w, h) = *pick(&mut self.rng, &p.sizes.rooms);
            self.world.place(0, 0, w, h, root, RoomKind::Normal);
            if self.grow(root, quota).1 >= p.min_rooms {
                break;
            }
            self.world.truncate(0, 0);
        }
        self.world.biomes.push(root);
        self.add_extras(root, p.extras[root]);

        // Next biomes: link to a random placed biome, then grow. A failed try is rolled
        // back so the next one starts from a clean state.
        // The map is unbounded, so retrying always ends up finding free space.
        for &child in &order[1..] {
            for attempt in 0.. {
                let (rooms_len, doors_len) = (self.world.rooms.len(), self.world.doors.len());
                let parent = self.world.biomes[self.rng.usize(..self.world.biomes.len())];
                if !self.link(parent, child) {
                    continue;
                }
                let (cells, rooms) = self.grow(child, quota);
                // The minimum is mandatory. Size is only preferred: a small biome is
                // accepted once the early attempts are used up.
                if rooms >= p.min_rooms && (cells >= quota / 3 || attempt >= BIOME_ATTEMPTS - 1) {
                    self.world.biomes.push(child);
                    self.add_extras(child, p.extras[child]);
                    break;
                }
                self.world.truncate(rooms_len, doors_len);
            }
        }

        // Extra corridors come last, once every biome has its neighbors.
        for i in 0..self.world.biomes.len() {
            let biome = self.world.biomes[i];
            for _ in 1..p.extras[biome].corridors {
                self.add_bridge(biome);
            }
        }
        self.world
    }

    /// Indices of the rooms of a biome that can receive new rooms. Corridors, secrets and
    /// rooms with a marker are left out, so their door count never changes.
    fn normal_rooms(&self, biome: usize) -> Vec<usize> {
        let rooms = &self.world.rooms;
        (0..rooms.len())
            .filter(|&i| {
                let r = &rooms[i];
                r.biome == biome && r.kind == RoomKind::Normal && r.marker.is_none()
            })
            .collect()
    }

    /// Attaches random rooms to random rooms of the biome. Stops at `max_rooms`, or once
    /// both the cell quota and `min_rooms` are reached, or when the try budget runs out.
    /// Returns the biome cell count and normal room count.
    fn grow(&mut self, biome: usize, quota: u32) -> (u32, usize) {
        let (min_rooms, max_rooms) = (self.p.min_rooms, self.p.max_rooms);
        let mut rooms = self.normal_rooms(biome);
        let mut cells: u32 = rooms
            .iter()
            .map(|&i| self.world.rooms[i].w * self.world.rooms[i].h)
            .sum();
        for _ in 0..(quota + min_rooms as u32) * 30 {
            if rooms.len() >= max_rooms || (cells >= quota && rooms.len() >= min_rooms) {
                break;
            }
            let from = *pick(&mut self.rng, &rooms);
            let (w, h) = *pick(&mut self.rng, &self.p.sizes.rooms);
            let dir = Dir::random(&mut self.rng, 0, 4);
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
            let from = *pick(&mut self.rng, &rooms);
            let dir = Dir::random(&mut self.rng, 0, 4);
            if let Some(idx) = self.try_attach(from, dir, 1, 1, biome, kind) {
                self.world.rooms[idx].marker = marker;
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
            let from = *pick(&mut self.rng, &rooms);
            let (w, h) = *pick(&mut self.rng, &self.p.sizes.boss);
            let dir = Dir::random(&mut self.rng, 0, 4);
            let Some(boss) = self.try_attach(from, dir, w, h, biome, RoomKind::Normal) else {
                continue;
            };
            let (w, h) = *pick(&mut self.rng, &self.p.sizes.rooms);
            let dir = Dir::random(&mut self.rng, 0, 4);
            if self
                .try_attach(boss, dir, w, h, biome, RoomKind::Normal)
                .is_some()
            {
                self.world.rooms[boss].marker = Some(Marker::Boss);
                return;
            }
            self.world.remove_last();
        }
    }

    /// Builds a corridor from a parent biome room, then the first child room at its end.
    /// The corridor is removed if the child room does not fit.
    fn link(&mut self, parent: usize, child: usize) -> bool {
        for _ in 0..LINK_ATTEMPTS {
            let candidates = self.normal_rooms(parent);
            let from = *pick(&mut self.rng, &candidates);
            let (cw, ch) = *pick(&mut self.rng, &self.p.sizes.corridor);
            // A wide corridor goes left or right, a tall one goes up or down.
            let dir = match cw.cmp(&ch) {
                Ordering::Greater => Dir::random(&mut self.rng, 0, 2),
                Ordering::Less => Dir::random(&mut self.rng, 2, 4),
                Ordering::Equal => Dir::random(&mut self.rng, 0, 4),
            };
            let Some(corridor) = self.try_attach(from, dir, cw, ch, parent, RoomKind::Corridor)
            else {
                continue;
            };
            let c = &self.world.rooms[corridor];
            if self.corridor_near(c.x, c.y, c.w as i32, c.h as i32, Some(corridor)) {
                self.world.remove_last();
                continue;
            }
            let (w, h) = *pick(&mut self.rng, &self.p.sizes.rooms);
            if self
                .try_attach(corridor, dir, w, h, child, RoomKind::Normal)
                .is_some()
            {
                return true;
            }
            self.world.remove_last();
        }
        false
    }

    /// True if another corridor lies within `CORRIDOR_SPACING` cells of the rectangle,
    /// diagonals included. `except` is the corridor that owns the rectangle, if placed.
    fn corridor_near(&self, x: i32, y: i32, w: i32, h: i32, except: Option<usize>) -> bool {
        let s = CORRIDOR_SPACING;
        (y - s..y + h + s).any(|cy| {
            (x - s..x + w + s).any(|cx| {
                self.world.room_at(cx, cy).is_some_and(|i| {
                    Some(i) != except && self.world.rooms[i].kind == RoomKind::Corridor
                })
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
            let r = &self.world.rooms[from];
            let (rx, ry, rw, rh) = (r.x, r.y, r.w as i32, r.h as i32);
            for dir in Dir::ALL {
                let side = if dir.is_horizontal() { rh } else { rw };
                let (dx, dy) = dir.step();
                for i in 0..side {
                    let (x, y) = match dir {
                        Dir::Right => (rx + rw, ry + i),
                        Dir::Left => (rx - 1, ry + i),
                        Dir::Down => (rx + i, ry + rh),
                        Dir::Up => (rx + i, ry - 1),
                    };
                    // Walk over free cells until a room is hit.
                    let Some((len, to)) = (0..=MAX_BRIDGE)
                        .find_map(|k| Some((k, self.world.room_at(x + dx * k, y + dy * k)?)))
                    else {
                        continue;
                    };
                    let size = if dir.is_horizontal() {
                        (len as u32, 1)
                    } else {
                        (1, len as u32)
                    };
                    let target = &self.world.rooms[to];
                    let (cx, cy, cw, ch) = bridge_rect(dir, x, y, len);
                    if len > 0
                        && target.biome != biome
                        && target.kind == RoomKind::Normal
                        && target.marker.is_none()
                        && self.p.sizes.corridor.contains(&size)
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
        let (from, dir, x, y, len, to) = *pick(&mut self.rng, &spots);
        // Corridor top-left cell and size, then the door cells at both ends.
        let (cx, cy, w, h) = bridge_rect(dir, x, y, len);
        let ((x1, y1), (x2, y2)) = match dir {
            Dir::Right => ((x, y), (x + len, y)),
            Dir::Left => ((x + 1, y), (x - len + 1, y)),
            Dir::Down => ((x, y), (x, y + len)),
            Dir::Up => ((x, y + 1), (x, y - len + 1)),
        };
        let vertical = dir.is_horizontal();
        let idx = self
            .world
            .place(cx, cy, w as u32, h as u32, biome, RoomKind::Corridor);
        self.world.doors.push(Door {
            a: from,
            b: idx,
            x: x1,
            y: y1,
            vertical,
        });
        self.world.doors.push(Door {
            a: idx,
            b: to,
            x: x2,
            y: y2,
            vertical,
        });
    }

    /// Places a w x h room against one side of room `from`, plus a door between them.
    /// Returns the new room index, or None if the spot is taken.
    fn try_attach(
        &mut self,
        from: usize,
        dir: Dir,
        w: u32,
        h: u32,
        biome: usize,
        kind: RoomKind,
    ) -> Option<usize> {
        let r = &self.world.rooms[from];
        let (rx, ry, rw, rh) = (r.x, r.y, r.w as i32, r.h as i32);
        let (wi, hi) = (w as i32, h as i32);

        // (nx, ny) is the new room top-left, (dx, dy) the door cell.
        // The new room slides along the shared side so that both rooms overlap on at
        // least one cell. The door is picked inside that overlap.
        let (nx, ny, dx, dy) = if dir.is_horizontal() {
            let ny = range(&mut self.rng, ry - hi + 1, ry + rh);
            let dy = range(&mut self.rng, ry.max(ny), (ry + rh).min(ny + hi));
            if dir == Dir::Right {
                (rx + rw, ny, rx + rw, dy)
            } else {
                (rx - wi, ny, rx, dy)
            }
        } else {
            let nx = range(&mut self.rng, rx - wi + 1, rx + rw);
            let dx = range(&mut self.rng, rx.max(nx), (rx + rw).min(nx + wi));
            if dir == Dir::Down {
                (nx, ry + rh, dx, ry + rh)
            } else {
                (nx, ry - hi, dx, ry)
            }
        };

        if !self.world.is_free(nx, ny, wi, hi) {
            return None;
        }
        let idx = self.world.place(nx, ny, w, h, biome, kind);
        self.world.doors.push(Door {
            a: from,
            b: idx,
            x: dx,
            y: dy,
            vertical: dir.is_horizontal(),
        });
        Some(idx)
    }
}
