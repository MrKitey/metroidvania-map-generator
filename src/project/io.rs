//! Dialogs and the recent-project list.

use crate::storage;
use crate::text::files;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) const RECENT_LIMIT: usize = 10;

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
