//! Project files: one JSON document with the `.mmg` extension.
//!
//! The file holds the map, the generation settings and the biomes. Autosave preferences
//! stay in settings.json and are not written here.

use crate::biome::{self, Biome, Entry};
use crate::settings::{self, Settings};
use crate::storage;
use crate::text::{self, files, status};
use crate::world::{Door, MapError, Room, World};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

const VERSION: u32 = 1;
const RECENT_LIMIT: usize = 10;

/// Generation settings stored in a project. Autosave is left out on purpose.
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct FileSettings {
    viewport: (u32, u32),
    min_rooms: usize,
    max_rooms: usize,
    room_sizes: Vec<(u32, u32)>,
    boss_sizes: Vec<(u32, u32)>,
    corridor_sizes: Vec<(u32, u32)>,
}

impl FileSettings {
    fn from_settings(settings: &Settings) -> FileSettings {
        FileSettings {
            viewport: settings.viewport,
            min_rooms: settings.min_rooms,
            max_rooms: settings.max_rooms,
            room_sizes: settings.room_sizes.clone(),
            boss_sizes: settings.boss_sizes.clone(),
            corridor_sizes: settings.corridor_sizes.clone(),
        }
    }

    fn into_settings(self) -> Settings {
        Settings {
            viewport: self.viewport,
            min_rooms: self.min_rooms,
            max_rooms: self.max_rooms,
            room_sizes: self.room_sizes,
            boss_sizes: self.boss_sizes,
            corridor_sizes: self.corridor_sizes,
            autosave: settings::DEFAULT_AUTOSAVE,
            autosave_secs: settings::DEFAULT_AUTOSAVE_SECS,
        }
    }
}

/// One saved project. Compared each frame to tell whether the map still matches the file.
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProjectFile {
    version: u32,
    name: String,
    seed: u64,
    cells: u32,
    grid: bool,
    settings: FileSettings,
    biomes: Vec<Entry>,
    used_biomes: Vec<usize>,
    rooms: Vec<Room>,
    doors: Vec<Door>,
}

impl ProjectFile {
    /// Snapshot of the open map. The name and the seed are stored as they are now.
    pub fn capture(
        name: &str,
        seed: u64,
        cells: u32,
        grid: bool,
        settings: &Settings,
        biomes: &[Biome],
        world: &World,
    ) -> ProjectFile {
        ProjectFile {
            version: VERSION,
            name: name.to_owned(),
            seed,
            cells,
            grid,
            settings: FileSettings::from_settings(settings),
            biomes: biomes.iter().map(Entry::from).collect(),
            used_biomes: world.biomes.clone(),
            rooms: world.rooms.clone(),
            doors: world.doors.clone(),
        }
    }
}

/// A project file turned into values the app can install.
pub struct Loaded {
    pub name: String,
    pub seed: u64,
    pub cells: u32,
    pub grid: bool,
    /// Generation settings. Autosave fields are the defaults, not the file's.
    pub settings: Settings,
    pub biomes: Vec<Biome>,
    pub world: World,
}

/// True when the path ends with `.mmg`, in any letter case.
pub fn is_mmg(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case(files::MMG_EXT))
}

/// First launch argument when it names a project file.
pub fn startup_path() -> Option<PathBuf> {
    let path = PathBuf::from(std::env::args_os().nth(1)?);
    is_mmg(&path).then_some(path)
}

/// File name proposed in the save dialog. A blank or unsafe name becomes the seed.
pub fn file_name(name: &str, seed: u64) -> String {
    format!("{}.{}", file_stem(name, seed), files::MMG_EXT)
}

fn file_stem(name: &str, seed: u64) -> String {
    let clean: String = name
        .chars()
        .filter(|c| *c != '/' && *c != '\\' && !c.is_control())
        .collect();
    let clean = clean.trim().trim_matches('.');
    if clean.is_empty() {
        seed.to_string()
    } else {
        clean.to_owned()
    }
}

fn dialog_dir() -> PathBuf {
    let dir = storage::app_dir();
    let _ = fs::create_dir_all(&dir);
    dir
}

/// Asks where to write. Returns None when the dialog is cancelled.
pub fn save_dialog(name: &str, seed: u64) -> Option<PathBuf> {
    let mut path = rfd::FileDialog::new()
        .add_filter(files::MMG_FILTER, &[files::MMG_EXT])
        .set_directory(dialog_dir())
        .set_file_name(file_name(name, seed))
        .save_file()?;
    if path.extension().is_none() {
        path.set_extension(files::MMG_EXT);
    }
    Some(path)
}

/// Asks which project to open. Returns None when the dialog is cancelled.
pub fn load_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter(files::MMG_FILTER, &[files::MMG_EXT])
        .set_directory(dialog_dir())
        .pick_file()
}

/// Reads a project. Fails when the version, the biomes or the map is unusable.
pub fn read(path: &Path) -> Result<Loaded, Box<dyn Error>> {
    let file: ProjectFile = storage::read_json(path)?;
    if file.version != VERSION {
        return Err(status::BAD_VERSION.into());
    }
    let biomes = file
        .biomes
        .into_iter()
        .map(biome::from_entry)
        .collect::<Result<Vec<_>, _>>()?;
    if biomes.is_empty() {
        return Err(text::biomes::NO_VALID_BIOME.into());
    }
    let world = World::from_parts(file.rooms, file.doors, file.used_biomes, biomes.len())
        .map_err(map_error)?;
    let name = if file.name.trim().is_empty() {
        file.seed.to_string()
    } else {
        file.name
    };
    Ok(Loaded {
        name,
        seed: file.seed,
        cells: file.cells,
        grid: file.grid,
        settings: file.settings.into_settings(),
        biomes,
        world,
    })
}

fn map_error(err: MapError) -> Box<dyn Error> {
    match err {
        MapError::Empty => status::EMPTY_MAP,
        MapError::EmptyRoom => status::EMPTY_ROOM,
        MapError::Overlap => status::OVERLAP,
        MapError::Index => status::BAD_INDEX,
    }
    .into()
}

/// Writes the project as indented JSON.
pub fn write(path: &Path, file: &ProjectFile) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(file)?)?;
    Ok(())
}

/// Recent paths, newest first. A missing or broken file is an empty list.
pub fn read_recent(dir: &Path) -> Vec<PathBuf> {
    storage::read_json(&dir.join(files::RECENT)).unwrap_or_default()
}

/// Puts `path` first and keeps at most ten entries.
pub fn remember(dir: &Path, path: &Path) -> Result<(), Box<dyn Error>> {
    let mut paths = read_recent(dir);
    paths.retain(|saved| saved != path);
    paths.insert(0, path.to_owned());
    paths.truncate(RECENT_LIMIT);
    write_recent(dir, &paths)
}

/// Drops one path from the recent list.
pub fn forget(dir: &Path, path: &Path) -> Result<(), Box<dyn Error>> {
    let mut paths = read_recent(dir);
    paths.retain(|saved| saved != path);
    write_recent(dir, &paths)
}

fn write_recent(dir: &Path, paths: &[PathBuf]) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    fs::write(
        dir.join(files::RECENT),
        serde_json::to_string_pretty(paths)?,
    )?;
    Ok(())
}

/// Registers the `.mmg` type with the desktop, once. Failures are ignored.
/// A missing `xdg-mime` is retried on a later launch.
pub fn register_mime() {
    #[cfg(target_os = "linux")]
    linux_mime();
}

#[cfg(target_os = "linux")]
fn linux_mime() {
    let dir = storage::app_dir();
    let stamp = dir.join(files::MIME_STAMP);
    let legacy = dir.join(files::MIME_XML_OLD);
    if stamp.exists() && !legacy.exists() {
        return;
    }
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let xml = dir.join(files::MIME_XML);
    let png = dir.join("icon.png");
    if fs::write(&xml, include_str!("../packaging/linux/metroidvania-mmg.xml")).is_err() {
        return;
    }
    let bytes = include_bytes!("../assets/icon.png");
    if fs::write(&png, bytes).is_err() {
        return;
    }
    let Ok(status) = std::process::Command::new("xdg-mime")
        .args(["install", "--mode", "user"])
        .arg(&xml)
        .status()
    else {
        return;
    };
    if !status.success() {
        return;
    }
    if legacy.exists() {
        let _ = std::process::Command::new("xdg-mime")
            .args(["uninstall", "--mode", "user"])
            .arg(&legacy)
            .status();
        let _ = fs::remove_file(&legacy);
    }
    if let Some(side) = png_side(bytes) {
        let _ = std::process::Command::new("xdg-icon-resource")
            .args([
                "install",
                "--mode",
                "user",
                "--noupdate",
                "--context",
                "apps",
                "--size",
                &side.to_string(),
            ])
            .arg(&png)
            .arg("metroidvania-map-generator")
            .status();
        let _ = std::process::Command::new("xdg-icon-resource")
            .arg("forceupdate")
            .status();
    }
    let _ = fs::write(&stamp, "");
}

/// Width of a square PNG, read from the IHDR chunk. None when the file is not square.
#[cfg(target_os = "linux")]
fn png_side(bytes: &[u8]) -> Option<u32> {
    if bytes.len() < 24 || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    (width == height && width > 0).then_some(width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Marker, RoomKind};

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
        assert_eq!(paths.len(), RECENT_LIMIT);
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
}
