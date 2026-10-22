use super::file::{ProjectFile, read, read_meta, utc_now, write, write_with_version};
use super::io::{self, file_name, forget, read_recent, remember};
use crate::biome;
use crate::settings::Settings;
use crate::text::{files, status};
use crate::world::{Door, Marker, Room, RoomKind, World};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mmg-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn empty_thumb() -> Vec<u8> {
    // Minimal valid 1x1 transparent PNG.
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ]
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
            icon: None,
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
        "2024-01-02T03:04:05Z",
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
    write(&path, &file, &empty_thumb()).unwrap();

    let meta = read_meta(&path).unwrap();
    assert_eq!(meta["version"], 2);
    assert_eq!(meta["name"], "Castle");
    assert_eq!(meta["created_at"], "2024-01-02T03:04:05Z");
    assert!(meta.get("modified_at").is_some());
    assert!(meta.get("autosave").is_none());
    assert!(meta["settings"].get("autosave").is_none());

    let mut zip = zip::ZipArchive::new(fs::File::open(&path).unwrap()).unwrap();
    for name in [
        files::PROJECT_JSON,
        files::PROJECT_BIOMES,
        files::PROJECT_ROOMS,
        files::PROJECT_THUMB,
    ] {
        assert!(zip.by_name(name).is_ok(), "missing {name}");
    }
    let mut thumb = Vec::new();
    zip.by_name(files::PROJECT_THUMB)
        .unwrap()
        .read_to_end(&mut thumb)
        .unwrap();
    assert_eq!(thumb, empty_thumb());

    let loaded = read(&path).unwrap();
    assert_eq!(loaded.name, "Castle");
    assert_eq!(loaded.seed, 42);
    assert_eq!(loaded.cells, 768);
    assert!(!loaded.grid);
    assert_eq!(loaded.created_at, "2024-01-02T03:04:05Z");
    assert!(!loaded.modified_at.is_empty());
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
    let (file, _) = sample();
    let path = dir.join("old.mmg");
    write_with_version(&path, &file, &empty_thumb(), 99).unwrap();
    match read(&path) {
        Err(err) => assert_eq!(err.to_string(), status::BAD_VERSION),
        Ok(_) => panic!("old version was accepted"),
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn dates_are_written() {
    let dir = temp_dir("dates");
    let (file, _) = sample();
    let path = dir.join("castle.mmg");
    write(&path, &file, &empty_thumb()).unwrap();
    let meta = read_meta(&path).unwrap();
    assert_eq!(meta["created_at"], "2024-01-02T03:04:05Z");
    let modified = meta["modified_at"].as_str().unwrap();
    assert_eq!(modified.len(), 20);
    assert!(modified.ends_with('Z'));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn utc_now_looks_like_iso() {
    let stamp = utc_now();
    assert_eq!(stamp.len(), 20);
    assert!(stamp.ends_with('Z'));
    assert_eq!(&stamp[4..5], "-");
    assert_eq!(&stamp[7..8], "-");
    assert_eq!(&stamp[10..11], "T");
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
        remember(
            &dir,
            Path::new(&format!("/tmp/p{i}.mmg")),
            &format!("P{i}"),
            "2026-01-01T00:00:00Z",
        )
        .unwrap();
    }
    let entries = read_recent(&dir);
    assert_eq!(entries.len(), io::RECENT_LIMIT);
    assert_eq!(entries[0].path, PathBuf::from("/tmp/p11.mmg"));
    assert_eq!(entries[0].name, "P11");
    remember(&dir, Path::new("/tmp/p5.mmg"), "P5", "2026-03-21T20:57:00Z").unwrap();
    let entries = read_recent(&dir);
    assert_eq!(entries[0].path, PathBuf::from("/tmp/p5.mmg"));
    assert_eq!(entries[0].label(), "P5 - 2026-03-21 20:57");
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry.path == Path::new("/tmp/p5.mmg"))
            .count(),
        1
    );
    forget(&dir, Path::new("/tmp/p5.mmg")).unwrap();
    assert!(
        !read_recent(&dir)
            .iter()
            .any(|entry| entry.path == Path::new("/tmp/p5.mmg"))
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn recent_reads_legacy_path_list() {
    let dir = temp_dir("recent-legacy");
    fs::write(
        dir.join("recent.json"),
        serde_json::to_string_pretty(&["/tmp/old.mmg"]).unwrap(),
    )
    .unwrap();
    let entries = read_recent(&dir);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].path, PathBuf::from("/tmp/old.mmg"));
    assert_eq!(entries[0].name, "old");
    assert_eq!(entries[0].label(), "old");
    let _ = fs::remove_dir_all(&dir);
}
