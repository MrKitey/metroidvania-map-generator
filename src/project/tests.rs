use super::file::{ProjectFile, read, write};
use super::io::{self, file_name, forget, read_recent, remember};
use crate::biome;
use crate::settings::Settings;
use crate::text::status;
use crate::world::{Door, Marker, Room, RoomKind, World};
use std::fs;
use std::path::{Path, PathBuf};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mmg-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn sample() -> (ProjectFile, World) {
    let world = World::from_parts(
        vec![Room {
            x: 0,
            y: 0,
            w: 2,
            h: 2,
            biome: 0,
            kind: RoomKind::Normal,
            marker: Some(Marker::Boss),
        }],
        vec![Door {
            a: 0,
            b: 0,
            x: 1,
            y: 0,
            vertical: true,
        }],
        vec![0],
        1,
    )
    .unwrap();
    let biomes = biome::default_biomes();
    let file = ProjectFile::capture(
        "Castle",
        42,
        768,
        false,
        &Settings::default(),
        &biomes,
        &world,
    );
    (file, world)
}

#[test]
fn project_round_trip() {
    let dir = temp_dir("round");
    let (file, world) = sample();
    let path = dir.join("castle.mmg");
    write(&path, &file).unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert!(!text.contains("autosave"));
    let loaded = read(&path).unwrap();
    assert_eq!(loaded.name, "Castle");
    assert_eq!(loaded.seed, 42);
    assert_eq!(loaded.cells, 768);
    assert!(!loaded.grid);
    assert_eq!(loaded.world.rooms, world.rooms);
    assert_eq!(loaded.world.doors, world.doors);
    assert_eq!(loaded.world.biomes, world.biomes);
    assert_eq!(loaded.world.room_at(0, 0), Some(0));
    assert_eq!(loaded.world.room_at(3, 3), None);
    assert_eq!(loaded.settings.viewport, Settings::default().viewport);
    assert!(loaded.settings.autosave);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn bad_version_is_rejected() {
    let dir = temp_dir("version");
    let (mut file, _) = sample();
    file.version = 99;
    let path = dir.join("old.mmg");
    write(&path, &file).unwrap();
    match read(&path) {
        Err(err) => assert_eq!(err.to_string(), status::BAD_VERSION),
        Ok(_) => panic!("old version was accepted"),
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn blank_name_becomes_the_seed() {
    assert_eq!(file_name("  ", 7), "7.mmg");
    assert_eq!(file_name("../Castle", 7), "Castle.mmg");
    assert_eq!(file_name("a/b", 7), "ab.mmg");
}

#[test]
fn recent_is_capped_and_promotes_duplicates() {
    let dir = temp_dir("recent");
    for i in 0..12 {
        remember(&dir, Path::new(&format!("/tmp/p{i}.mmg"))).unwrap();
    }
    let paths = read_recent(&dir);
    assert_eq!(paths.len(), io::RECENT_LIMIT);
    assert_eq!(paths[0], PathBuf::from("/tmp/p11.mmg"));
    remember(&dir, Path::new("/tmp/p5.mmg")).unwrap();
    let paths = read_recent(&dir);
    assert_eq!(paths[0], PathBuf::from("/tmp/p5.mmg"));
    assert_eq!(
        paths
            .iter()
            .filter(|path| *path == Path::new("/tmp/p5.mmg"))
            .count(),
        1
    );
    forget(&dir, Path::new("/tmp/p5.mmg")).unwrap();
    assert!(
        !read_recent(&dir)
            .iter()
            .any(|path| path == Path::new("/tmp/p5.mmg"))
    );
    let _ = fs::remove_dir_all(&dir);
}
