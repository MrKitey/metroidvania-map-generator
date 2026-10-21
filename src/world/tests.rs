use super::*;
use crate::settings::{DEFAULT_BOSS_SIZES, DEFAULT_CORRIDOR_SIZES, DEFAULT_ROOM_SIZES};

const SEEDS: [u64; 4] = [1, 42, 99, 12345];
const MIN_ROOMS: usize = 5;

fn sizes() -> Sizes {
    Sizes {
        rooms: DEFAULT_ROOM_SIZES.to_vec(),
        boss: DEFAULT_BOSS_SIZES.to_vec(),
        corridor: DEFAULT_CORRIDOR_SIZES.to_vec(),
    }
}

fn generate_with(cells: u32, seed: u64, corridors: u32) -> World {
    let extras = [Extras {
        corridors,
        secrets: 1,
        bosses: 1,
        saves: 1,
        portals: 1,
    }; 6];
    World::generate(&Params {
        cells,
        seed,
        extras: &extras,
        min_rooms: MIN_ROOMS,
        max_rooms: 50,
        sizes: &sizes(),
    })
}

fn generate(cells: u32, seed: u64) -> World {
    generate_with(cells, seed, 2)
}

fn json(world: &World) -> String {
    serde_json::to_string(&(&world.rooms, &world.doors)).unwrap()
}

#[test]
fn same_seed_gives_same_map() {
    for seed in SEEDS {
        assert_eq!(json(&generate(768, seed)), json(&generate(768, seed)));
    }
}

/// Guards the generator against changes that alter existing maps. Update the numbers
/// only when the algorithm is changed on purpose.
#[test]
fn known_seeds_give_known_maps() {
    for (cells, seed, rooms, doors) in [
        (768, 1, 215, 214),
        (768, 42, 206, 205),
        (96, 7, 38, 37),
        (2048, 99, 335, 334),
    ] {
        let world = generate_with(cells, seed, 1);
        assert_eq!((world.rooms.len(), world.doors.len()), (rooms, doors));
    }
}

#[test]
fn rooms_never_overlap() {
    for seed in SEEDS {
        let world = generate(768, seed);
        let cells: u32 = world.rooms.iter().map(|r| r.w * r.h).sum();
        assert_eq!(cells as usize, world.grid.len());
        for (i, r) in world.rooms.iter().enumerate() {
            for y in r.y..r.y + r.h as i32 {
                for x in r.x..r.x + r.w as i32 {
                    assert_eq!(world.room_at(x, y), Some(i));
                }
            }
        }
    }
}

#[test]
fn doors_join_touching_rooms() {
    for seed in SEEDS {
        let world = generate(768, seed);
        for d in &world.doors {
            assert!(d.a < world.rooms.len() && d.b < world.rooms.len());
            let before = if d.vertical {
                world.room_at(d.x - 1, d.y)
            } else {
                world.room_at(d.x, d.y - 1)
            };
            let after = world.room_at(d.x, d.y);
            let pair = [before, after];
            assert!(pair.contains(&Some(d.a)) && pair.contains(&Some(d.b)));
        }
    }
}

#[test]
fn every_room_is_reachable() {
    for seed in SEEDS {
        let world = generate(768, seed);
        let mut seen = vec![false; world.rooms.len()];
        let mut stack = vec![0];
        seen[0] = true;
        while let Some(i) = stack.pop() {
            for d in &world.doors {
                for (from, to) in [(d.a, d.b), (d.b, d.a)] {
                    if from == i && !seen[to] {
                        seen[to] = true;
                        stack.push(to);
                    }
                }
            }
        }
        assert!(seen.iter().all(|&s| s));
    }
}

#[test]
fn each_used_biome_has_the_minimum_rooms() {
    for seed in SEEDS {
        let world = generate(768, seed);
        for &b in &world.biomes {
            let normal = world
                .rooms
                .iter()
                .filter(|r| r.biome == b && r.kind == RoomKind::Normal)
                .count();
            assert!(normal >= MIN_ROOMS);
        }
    }
}

#[test]
fn manual_room_gets_a_door_to_its_neighbor() {
    let mut world = generate(768, 1);
    let (_, _, max_x, _) = world.bounds();
    // Far from the map: no neighbor, no door.
    let doors = world.doors.len();
    assert!(world.can_place(max_x + 5, 0, 1, 1));
    world.place_manual((max_x + 5, 0), (1, 1), RoomKind::Normal, None);
    assert_eq!(world.doors.len(), doors);
    // Against a room: at least one door.
    let spot = world
        .rooms
        .iter()
        .map(|r| (r.x + r.w as i32, r.y))
        .find(|&(x, y)| world.can_place(x, y, 1, 1))
        .unwrap();
    world.place_manual(spot, (1, 1), RoomKind::Normal, None);
    assert!(world.doors.len() > doors);
    // Taken spot.
    assert!(!world.can_place(0, 0, 1, 1));
    assert!(
        world
            .place_manual((0, 0), (1, 1), RoomKind::Normal, None)
            .is_none()
    );
}

#[test]
fn toggles_follow_the_rules() {
    let mut world = generate(768, 1);
    let normal = world
        .rooms
        .iter()
        .position(|r| r.kind == RoomKind::Normal && r.marker.is_none())
        .unwrap();
    world.toggle_kind(normal, RoomKind::Secret);
    assert!(world.rooms[normal].kind == RoomKind::Secret);
    // A marker replaces the secret kind.
    world.toggle_marker(normal, Marker::Boss);
    assert!(world.rooms[normal].kind == RoomKind::Normal);
    assert!(world.rooms[normal].marker == Some(Marker::Boss));
    // Same marker again clears it.
    world.toggle_marker(normal, Marker::Boss);
    assert!(world.rooms[normal].marker.is_none());
    // Corridors keep their kind.
    let corridor = world
        .rooms
        .iter()
        .position(|r| r.kind == RoomKind::Corridor)
        .unwrap();
    world.toggle_kind(corridor, RoomKind::Portal);
    assert!(world.rooms[corridor].kind == RoomKind::Corridor);
}

#[test]
fn moving_a_room_rebuilds_its_doors() {
    let mut world = generate(768, 1);
    let idx = world.rooms.len() - 1;
    let (w, h) = (world.rooms[idx].w, world.rooms[idx].h);
    let (_, _, max_x, _) = world.bounds();
    // Taken spot: nothing changes.
    let (ox, oy) = (world.rooms[idx].x, world.rooms[idx].y);
    let doors = world.doors.len();
    assert!(!world.move_room(idx, world.rooms[0].x, world.rooms[0].y));
    assert_eq!(world.doors.len(), doors);
    // Far away: no neighbor, so no door left on this room.
    assert!(world.move_room(idx, max_x + 5, 0));
    assert!(world.doors.iter().all(|d| d.a != idx && d.b != idx));
    assert_eq!(world.room_at(max_x + 5, 0), Some(idx));
    assert_eq!(world.room_at(ox, oy), None);
    // Back to its place: the spot is free again and it touches rooms again.
    assert!(world.can_move(idx, ox, oy));
    assert!(world.move_room(idx, ox, oy));
    assert!(world.doors.iter().any(|d| d.a == idx || d.b == idx));
    assert_eq!((world.rooms[idx].w, world.rooms[idx].h), (w, h));
}

#[test]
fn moved_room_takes_the_biome_of_its_neighbor() {
    let mut world = generate(768, 1);
    let idx = world.rooms.len() - 1;
    let (w, h) = (world.rooms[idx].w as i32, world.rooms[idx].h as i32);
    // Biomes of the rooms touching a w x h rectangle at (x, y), the moved room excluded.
    let around = |x: i32, y: i32| {
        let mut biomes: Vec<usize> = (0..h)
            .flat_map(|i| [(x + w, y + i), (x - 1, y + i)])
            .chain((0..w).flat_map(|i| [(x + i, y + h), (x + i, y - 1)]))
            .filter_map(|(cx, cy)| world.room_at(cx, cy))
            .filter(|&n| n != idx)
            .map(|n| world.rooms[n].biome)
            .collect();
        biomes.sort_unstable();
        biomes.dedup();
        biomes
    };
    // A free spot touching rooms of one other biome only.
    let own = world.rooms[idx].biome;
    let target = world
        .rooms
        .iter()
        .map(|r| (r.x + r.w as i32, r.y))
        .find_map(|(x, y)| match around(x, y)[..] {
            [biome] if biome != own && world.can_move(idx, x, y) => Some((x, y, biome)),
            _ => None,
        });
    let (x, y, biome) = target.unwrap();
    assert!(world.move_room(idx, x, y));
    assert_eq!(world.rooms[idx].biome, biome);
}

#[test]
fn removing_a_room_frees_its_cells_and_keeps_the_last_one() {
    let mut world = generate(768, 1);
    assert!(world.rooms.len() > 2);
    let idx = 1;
    let (x, y, w, h) = {
        let r = &world.rooms[idx];
        (r.x, r.y, r.w, r.h)
    };
    let before = world.rooms.len();
    assert!(world.remove_room(idx));
    assert_eq!(world.rooms.len(), before - 1);
    for cy in y..y + h as i32 {
        for cx in x..x + w as i32 {
            assert_eq!(world.room_at(cx, cy), None);
        }
    }
    for (i, r) in world.rooms.iter().enumerate() {
        for cy in r.y..r.y + r.h as i32 {
            for cx in r.x..r.x + r.w as i32 {
                assert_eq!(world.room_at(cx, cy), Some(i));
            }
        }
    }
    for d in &world.doors {
        assert!(d.a < world.rooms.len() && d.b < world.rooms.len());
        let before = if d.vertical {
            world.room_at(d.x - 1, d.y)
        } else {
            world.room_at(d.x, d.y - 1)
        };
        let after = world.room_at(d.x, d.y);
        let pair = [before, after];
        assert!(pair.contains(&Some(d.a)) && pair.contains(&Some(d.b)));
    }
    while world.rooms.len() > 1 {
        assert!(world.remove_room(0));
    }
    assert!(!world.remove_room(0));
    assert_eq!(world.rooms.len(), 1);
    assert!(world.doors.is_empty());
}
