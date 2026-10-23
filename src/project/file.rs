//! Saved project archive: zip with metadata, thumbnail, biomes and layers.

use crate::biome::{self, Biome, Entry};
use crate::layer::{LayerStack, SavedLayer};
use crate::settings::{self, Settings};
use crate::text::{self, files, status};
use crate::world::{Door, MapError, Room, World};
use std::error::Error;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

const VERSION: u32 = 3;
const VERSION_V2: u32 = 2;

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
            // UI theme and icon size live in settings.json, not in the project archive.
            dark: true,
            icon_size: settings::DEFAULT_ICON_SIZE,
        }
    }
}

/// Metadata written to `project.json`.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct ProjectMeta {
    version: u32,
    name: String,
    seed: u64,
    cells: u32,
    grid: bool,
    created_at: String,
    modified_at: String,
    settings: FileSettings,
}

/// Rooms written to `rooms.json` (v2).
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct RoomsFile {
    rooms: Vec<Room>,
    doors: Vec<Door>,
    used_biomes: Vec<usize>,
}

/// Layers written to `layers.json` (v3).
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct LayersFile {
    active: usize,
    layers: Vec<SavedLayer>,
}

/// One saved project. Compared each frame to tell whether the map still matches the file.
/// Thumbnail bytes and `modified_at` are left out of equality on purpose.
#[derive(Clone, PartialEq)]
pub struct ProjectFile {
    pub(super) version: u32,
    name: String,
    seed: u64,
    cells: u32,
    grid: bool,
    created_at: String,
    settings: FileSettings,
    biomes: Vec<Entry>,
    layers: LayersFile,
}

impl ProjectFile {
    /// Snapshot of the open map. The name, seed and creation date are stored as they are now.
    pub fn capture(
        name: &str,
        seed: u64,
        cells: u32,
        grid: bool,
        created_at: &str,
        settings: &Settings,
        biomes: &[Biome],
        layers: &LayerStack,
    ) -> ProjectFile {
        ProjectFile {
            version: VERSION,
            name: name.to_owned(),
            seed,
            cells,
            grid,
            created_at: created_at.to_owned(),
            settings: FileSettings::from_settings(settings),
            biomes: biomes.iter().map(Entry::from).collect(),
            layers: LayersFile {
                active: layers.active(),
                layers: layers.to_saved(),
            },
        }
    }

    pub fn created_at(&self) -> &str {
        &self.created_at
    }
}

/// A project file turned into values the app can install.
pub struct Loaded {
    pub name: String,
    pub seed: u64,
    pub cells: u32,
    pub grid: bool,
    pub created_at: String,
    pub modified_at: String,
    /// Generation settings. Autosave fields are the defaults, not the file's.
    pub settings: Settings,
    pub biomes: Vec<Biome>,
    pub layers: LayerStack,
}

/// UTC timestamp `YYYY-MM-DDTHH:MM:SSZ`.
pub fn utc_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_utc(secs)
}

fn format_utc(secs: u64) -> String {
    const SECS_PER_DAY: u64 = 86_400;

    let days = (secs / SECS_PER_DAY) as i64;
    let time = secs % SECS_PER_DAY;
    let hour = time / 3600;
    let min = (time % 3600) / 60;
    let sec = time % 60;

    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

/// Howard Hinnant's civil-from-days algorithm.
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

/// Reads a project zip. Accepts v3 (`layers.json`) and migrates v2 (`rooms.json`).
pub fn read(path: &Path) -> Result<Loaded, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut zip = zip::ZipArchive::new(file)?;
    let meta: ProjectMeta = read_json_entry(&mut zip, files::PROJECT_JSON)?;
    if meta.version != VERSION && meta.version != VERSION_V2 {
        return Err(status::BAD_VERSION.into());
    }
    let biomes_raw: Vec<Entry> = read_json_entry(&mut zip, files::PROJECT_BIOMES)?;
    // Thumbnail is required in the archive even though the app does not display it yet.
    let _thumb = read_bytes_entry(&mut zip, files::PROJECT_THUMB)?;

    let biomes = biomes_raw
        .into_iter()
        .map(biome::from_entry)
        .collect::<Result<Vec<_>, _>>()?;
    if biomes.is_empty() {
        return Err(text::biomes::NO_VALID_BIOME.into());
    }

    let layers = if meta.version == VERSION {
        let file: LayersFile = read_json_entry(&mut zip, files::PROJECT_LAYERS)?;
        LayerStack::from_saved(file.active, file.layers, biomes.len()).map_err(map_error)?
    } else {
        let rooms: RoomsFile = read_json_entry(&mut zip, files::PROJECT_ROOMS)?;
        let world = World::from_parts(rooms.rooms, rooms.doors, rooms.used_biomes, biomes.len())
            .map_err(map_error)?;
        LayerStack::from_world(world)
    };

    let name = if meta.name.trim().is_empty() {
        meta.seed.to_string()
    } else {
        meta.name
    };
    Ok(Loaded {
        name,
        seed: meta.seed,
        cells: meta.cells,
        grid: meta.grid,
        created_at: meta.created_at,
        modified_at: meta.modified_at,
        settings: meta.settings.into_settings(),
        biomes,
        layers,
    })
}

fn read_json_entry<T: serde::de::DeserializeOwned>(
    zip: &mut zip::ZipArchive<File>,
    name: &str,
) -> Result<T, Box<dyn Error>> {
    let mut entry = zip.by_name(name)?;
    let mut text = String::new();
    entry.read_to_string(&mut text)?;
    Ok(serde_json::from_str(&text)?)
}

fn read_bytes_entry(
    zip: &mut zip::ZipArchive<File>,
    name: &str,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut entry = zip.by_name(name)?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    Ok(bytes)
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

/// Writes the project as a zip with metadata, thumbnail, biomes and layers.
/// Returns the `modified_at` stamp written into `project.json`.
pub fn write(path: &Path, file: &ProjectFile, thumbnail: &[u8]) -> Result<String, Box<dyn Error>> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    let modified_at = utc_now();
    let meta = ProjectMeta {
        version: file.version,
        name: file.name.clone(),
        seed: file.seed,
        cells: file.cells,
        grid: file.grid,
        created_at: file.created_at.clone(),
        modified_at: modified_at.clone(),
        settings: file.settings.clone(),
    };

    let mut zip = zip::ZipWriter::new(File::create(path)?);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

    zip.start_file(files::PROJECT_JSON, deflated)?;
    zip.write_all(serde_json::to_string_pretty(&meta)?.as_bytes())?;
    zip.start_file(files::PROJECT_BIOMES, deflated)?;
    zip.write_all(serde_json::to_string_pretty(&file.biomes)?.as_bytes())?;
    zip.start_file(files::PROJECT_LAYERS, deflated)?;
    zip.write_all(serde_json::to_string_pretty(&file.layers)?.as_bytes())?;
    zip.start_file(files::PROJECT_THUMB, stored)?;
    zip.write_all(thumbnail)?;
    zip.finish()?;
    Ok(modified_at)
}

/// Writes a project zip with a custom `project.json` version. Used by tests.
#[cfg(test)]
pub(super) fn write_with_version(
    path: &Path,
    file: &ProjectFile,
    thumbnail: &[u8],
    version: u32,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    let meta = ProjectMeta {
        version,
        name: file.name.clone(),
        seed: file.seed,
        cells: file.cells,
        grid: file.grid,
        created_at: file.created_at.clone(),
        modified_at: utc_now(),
        settings: file.settings.clone(),
    };

    let mut zip = zip::ZipWriter::new(File::create(path)?);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

    zip.start_file(files::PROJECT_JSON, deflated)?;
    zip.write_all(serde_json::to_string_pretty(&meta)?.as_bytes())?;
    zip.start_file(files::PROJECT_BIOMES, deflated)?;
    zip.write_all(serde_json::to_string_pretty(&file.biomes)?.as_bytes())?;
    if version == VERSION_V2 {
        let first = file.layers.layers.first();
        let rooms = RoomsFile {
            rooms: first.map(|l| l.rooms.clone()).unwrap_or_default(),
            doors: first.map(|l| l.doors.clone()).unwrap_or_default(),
            used_biomes: first.map(|l| l.used_biomes.clone()).unwrap_or_default(),
        };
        zip.start_file(files::PROJECT_ROOMS, deflated)?;
        zip.write_all(serde_json::to_string_pretty(&rooms)?.as_bytes())?;
    } else {
        zip.start_file(files::PROJECT_LAYERS, deflated)?;
        zip.write_all(serde_json::to_string_pretty(&file.layers)?.as_bytes())?;
    }
    zip.start_file(files::PROJECT_THUMB, stored)?;
    zip.write_all(thumbnail)?;
    zip.finish()?;
    Ok(())
}

/// Writes a v2-shaped archive for migration tests.
#[cfg(test)]
pub(super) fn write_v2(
    path: &Path,
    name: &str,
    seed: u64,
    cells: u32,
    grid: bool,
    created_at: &str,
    settings: &Settings,
    biomes: &[Biome],
    world: &World,
    thumbnail: &[u8],
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let meta = ProjectMeta {
        version: VERSION_V2,
        name: name.to_owned(),
        seed,
        cells,
        grid,
        created_at: created_at.to_owned(),
        modified_at: utc_now(),
        settings: FileSettings::from_settings(settings),
    };
    let rooms = RoomsFile {
        rooms: world.rooms.clone(),
        doors: world.doors.clone(),
        used_biomes: world.biomes.clone(),
    };
    let biomes_entries: Vec<Entry> = biomes.iter().map(Entry::from).collect();

    let mut zip = zip::ZipWriter::new(File::create(path)?);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    zip.start_file(files::PROJECT_JSON, deflated)?;
    zip.write_all(serde_json::to_string_pretty(&meta)?.as_bytes())?;
    zip.start_file(files::PROJECT_BIOMES, deflated)?;
    zip.write_all(serde_json::to_string_pretty(&biomes_entries)?.as_bytes())?;
    zip.start_file(files::PROJECT_ROOMS, deflated)?;
    zip.write_all(serde_json::to_string_pretty(&rooms)?.as_bytes())?;
    zip.start_file(files::PROJECT_THUMB, stored)?;
    zip.write_all(thumbnail)?;
    zip.finish()?;
    Ok(())
}

/// Reads `project.json` from a project zip. Used by tests.
#[cfg(test)]
pub(super) fn read_meta(path: &Path) -> Result<serde_json::Value, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut zip = zip::ZipArchive::new(file)?;
    let mut entry = zip.by_name(files::PROJECT_JSON)?;
    let mut text = String::new();
    entry.read_to_string(&mut text)?;
    Ok(serde_json::from_str(&text)?)
}
