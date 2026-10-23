//! Special-room icon definitions and their storage in the app folder.

use crate::color::{parse_hex, to_hex};
use crate::storage;
use crate::text::{files, hud as labels, icons as ids, status};
use egui::Color32;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

/// Relative folder under the app dir that holds icon image files.
pub const ASSETS_DIR: &str = "assets/icons";

/// Default fill/border matching the previous hard-coded theme colors.
const BOSS_FILL: Color32 = Color32::from_rgb(0x80, 0x00, 0x00);
const BOSS_BORDER: Color32 = Color32::from_rgb(0xFF, 0x00, 0x00);
const SAVE_FILL: Color32 = Color32::from_rgb(0x00, 0x80, 0x00);
const SAVE_BORDER: Color32 = Color32::from_rgb(0x00, 0xFF, 0x00);
const SECRET_FILL: Color32 = Color32::from_rgb(0x80, 0x80, 0x00);
const SECRET_BORDER: Color32 = Color32::from_rgb(0xFF, 0xFF, 0x00);
const PORTAL_FILL: Color32 = Color32::from_rgb(0x00, 0x80, 0x80);
const PORTAL_BORDER: Color32 = Color32::from_rgb(0x00, 0xFF, 0xFF);

const EMBEDDED: &[(&str, &str, Color32, Color32, &str)] = &[
    (
        ids::BOSS,
        labels::LEGEND_BOSS,
        BOSS_FILL,
        BOSS_BORDER,
        include_str!("../assets/icons/boss.svg"),
    ),
    (
        ids::SAVE,
        labels::LEGEND_SAVE,
        SAVE_FILL,
        SAVE_BORDER,
        include_str!("../assets/icons/save.svg"),
    ),
    (
        ids::SECRET,
        labels::LEGEND_SECRET,
        SECRET_FILL,
        SECRET_BORDER,
        include_str!("../assets/icons/secret.svg"),
    ),
    (
        ids::PORTAL,
        labels::LEGEND_PORTAL,
        PORTAL_FILL,
        PORTAL_BORDER,
        include_str!("../assets/icons/portal.svg"),
    ),
];

/// One special-room icon: label, colors and an image file in the app assets folder.
#[derive(Clone, Debug, PartialEq)]
pub struct IconDef {
    pub id: String,
    pub label: String,
    pub fill: Color32,
    pub border: Color32,
    /// Path relative to the app dir, e.g. `assets/icons/boss.svg`.
    pub file: String,
}

impl IconDef {
    /// Absolute path of the image file.
    pub fn abs_path(&self) -> PathBuf {
        storage::app_dir().join(&self.file)
    }
}

/// True for the four ids that still map to `Marker` / `RoomKind`.
pub fn is_reserved(id: &str) -> bool {
    matches!(id, ids::BOSS | ids::SAVE | ids::SECRET | ids::PORTAL)
}

/// JSON shape of one icon entry.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Entry {
    id: String,
    label: String,
    fill: String,
    border: String,
    file: String,
}

impl From<&IconDef> for Entry {
    fn from(d: &IconDef) -> Entry {
        Entry {
            id: d.id.clone(),
            label: d.label.clone(),
            fill: to_hex(d.fill),
            border: to_hex(d.border),
            file: d.file.clone(),
        }
    }
}

fn from_entry(entry: Entry) -> Option<IconDef> {
    let fill = parse_hex(&entry.fill)?;
    let border = parse_hex(&entry.border)?;
    if entry.id.is_empty() || entry.file.is_empty() {
        return None;
    }
    Some(IconDef {
        id: entry.id,
        label: entry.label,
        fill,
        border,
        file: entry.file,
    })
}

/// Assets folder under the app dir.
pub fn assets_dir() -> PathBuf {
    storage::app_dir().join(ASSETS_DIR)
}

fn default_file(id: &str) -> String {
    format!("{ASSETS_DIR}/{id}.svg")
}

/// Seeds the four built-in icons and writes their SVG files under the app dir.
pub fn default_icons() -> Vec<IconDef> {
    let _ = fs::create_dir_all(assets_dir());
    EMBEDDED
        .iter()
        .map(|&(id, label, fill, border, svg)| {
            let file = default_file(id);
            let path = storage::app_dir().join(&file);
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&path, svg);
            IconDef {
                id: id.to_owned(),
                label: label.to_owned(),
                fill,
                border,
                file,
            }
        })
        .collect()
}

/// Reads a JSON list. Invalid entries are skipped. Empty list fails.
pub fn read(path: &Path) -> Result<Vec<IconDef>, Box<dyn Error>> {
    let entries: Vec<Entry> = storage::read_json(path)?;
    let icons: Vec<IconDef> = entries.into_iter().filter_map(from_entry).collect();
    if icons.is_empty() {
        return Err(status::NO_VALID_ICON.into());
    }
    Ok(icons)
}

/// Icons saved by the user, or the defaults if the file is missing or invalid.
pub fn load() -> Vec<IconDef> {
    let path = storage::app_dir().join(files::SPECIAL_ROOM);
    match read(&path) {
        Ok(list) => {
            ensure_default_assets(&list);
            list
        }
        Err(_) => {
            let list = default_icons();
            let _ = save(&list);
            list
        }
    }
}

/// Writes missing default SVG files so a partial install still has art.
fn ensure_default_assets(list: &[IconDef]) {
    for &(id, _, _, _, svg) in EMBEDDED {
        let Some(def) = list.iter().find(|d| d.id == id) else {
            continue;
        };
        let path = def.abs_path();
        if path.exists() {
            continue;
        }
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, svg);
    }
}

/// Writes the icons list. Returns the path written to.
pub fn save(icons: &[IconDef]) -> Result<PathBuf, Box<dyn Error>> {
    let _ = fs::create_dir_all(assets_dir());
    let entries: Vec<Entry> = icons.iter().map(Entry::from).collect();
    storage::write_json(files::SPECIAL_ROOM, &entries)
}

/// Looks up an icon by id.
pub fn find<'a>(icons: &'a [IconDef], id: &str) -> Option<&'a IconDef> {
    icons.iter().find(|d| d.id == id)
}

/// Allowed image extensions for import.
pub fn is_image_path(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        matches!(
            e.to_ascii_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "svg"
        )
    })
}

/// Builds a unique slug from a label.
pub fn unique_id(label: &str, existing: &[IconDef]) -> String {
    let base = slugify(label);
    let base = if base.is_empty() {
        "icon".to_owned()
    } else {
        base
    };
    if !existing.iter().any(|d| d.id == base) {
        return base;
    }
    for n in 2.. {
        let candidate = format!("{base}_{n}");
        if !existing.iter().any(|d| d.id == candidate) {
            return candidate;
        }
    }
    unreachable!()
}

fn slugify(label: &str) -> String {
    let mut out = String::new();
    let mut prev_sep = true;
    for c in label.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_sep = false;
        } else if !prev_sep {
            out.push('_');
            prev_sep = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    out
}

/// Copies `src` into the assets folder as `{id}.{ext}` and returns the relative path.
pub fn import_file(id: &str, src: &Path) -> Result<String, Box<dyn Error>> {
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .filter(|e| matches!(e.as_str(), "png" | "jpg" | "jpeg" | "svg"))
        .ok_or("unsupported image format")?;
    let _ = fs::create_dir_all(assets_dir());
    let rel = format!("{ASSETS_DIR}/{id}.{ext}");
    let dest = storage::app_dir().join(&rel);
    fs::copy(src, &dest)?;
    Ok(rel)
}

/// Creates a new icon with default gray colors and no file yet.
pub fn new_blank(existing: &[IconDef]) -> IconDef {
    let label = "New".to_owned();
    let id = unique_id(&label, existing);
    IconDef {
        id: id.clone(),
        label,
        fill: Color32::from_rgb(128, 128, 128),
        border: Color32::from_rgb(200, 200, 200),
        file: default_file(&id),
    }
}
