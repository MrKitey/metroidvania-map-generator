//! Saved project document: the JSON shape, and reading and writing it.

use crate::biome::{self, Biome, Entry};
use crate::settings::{self, Settings};
use crate::storage;
use crate::text::{self, status};
use crate::world::{Door, MapError, Room, World};
use std::error::Error;
use std::fs;
use std::path::Path;

const VERSION: u32 = 1;

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
    pub(super) version: u32,
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
