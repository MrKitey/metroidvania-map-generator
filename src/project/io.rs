//! Dialogs and the recent-project list.

use crate::storage;
use crate::text::files;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) const RECENT_LIMIT: usize = 10;

/// One row in the recent-project list.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecentEntry {
    pub path: PathBuf,
    pub name: String,
    pub modified_at: String,
}

impl RecentEntry {
    fn from_path(path: PathBuf) -> RecentEntry {
        let name = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        RecentEntry {
            path,
            name,
            modified_at: String::new(),
        }
    }

    /// Menu label: `Name - YYYY-MM-DD HH:MM`.
    pub fn label(&self) -> String {
        let date = display_modified(&self.modified_at);
        if date.is_empty() {
            self.name.clone()
        } else {
            format!("{} - {}", self.name, date)
        }
    }
}

/// Turns `2026-03-21T20:57:00Z` into `2026-03-21 20:57`.
fn display_modified(iso: &str) -> String {
    if iso.len() >= 16 && iso.as_bytes().get(10) == Some(&b'T') {
        format!("{} {}", &iso[..10], &iso[11..16])
    } else {
        String::new()
    }
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

/// Recent entries, newest first. A missing or broken file is an empty list.
pub fn read_recent(dir: &Path) -> Vec<RecentEntry> {
    let path = dir.join(files::RECENT);
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    if let Ok(entries) = serde_json::from_str::<Vec<RecentEntry>>(&text) {
        return entries;
    }
    // Older builds stored a bare path list.
    if let Ok(paths) = serde_json::from_str::<Vec<PathBuf>>(&text) {
        return paths.into_iter().map(RecentEntry::from_path).collect();
    }
    Vec::new()
}

/// Puts the project first and keeps at most ten entries.
pub fn remember(
    dir: &Path,
    path: &Path,
    name: &str,
    modified_at: &str,
) -> Result<(), Box<dyn Error>> {
    let mut entries = read_recent(dir);
    entries.retain(|saved| saved.path != path);
    entries.insert(
        0,
        RecentEntry {
            path: path.to_owned(),
            name: name.to_owned(),
            modified_at: modified_at.to_owned(),
        },
    );
    entries.truncate(RECENT_LIMIT);
    write_recent(dir, &entries)
}

/// Drops one path from the recent list.
pub fn forget(dir: &Path, path: &Path) -> Result<(), Box<dyn Error>> {
    let mut entries = read_recent(dir);
    entries.retain(|saved| saved.path != path);
    write_recent(dir, &entries)
}

fn write_recent(dir: &Path, entries: &[RecentEntry]) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    fs::write(
        dir.join(files::RECENT),
        serde_json::to_string_pretty(entries)?,
    )?;
    Ok(())
}

#[cfg(test)]
mod label_tests {
    use super::*;

    #[test]
    fn label_uses_name_and_date() {
        let entry = RecentEntry {
            path: PathBuf::from("/tmp/castle.mmg"),
            name: "Castle".into(),
            modified_at: "2026-03-21T20:57:00Z".into(),
        };
        assert_eq!(entry.label(), "Castle - 2026-03-21 20:57");
    }

    #[test]
    fn label_without_date_is_name_only() {
        let entry = RecentEntry {
            path: PathBuf::from("/tmp/castle.mmg"),
            name: "Castle".into(),
            modified_at: String::new(),
        };
        assert_eq!(entry.label(), "Castle");
    }
}
