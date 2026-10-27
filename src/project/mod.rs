//! Project files: a zip archive with the `.mmg` extension.
//!
//! The archive holds `project.json`, `thumbnail.png`, `biomes.json` and `layers.json`.
//! Version 2 files with `rooms.json` are still loaded and migrated.
//! Autosave preferences stay in settings.json and are not written here.

mod file;
mod io;
mod mime;

#[cfg(test)]
mod tests;

pub use file::{Loaded, ProjectFile, Snapshot, read, utc_now, write};
pub use io::{forget, is_mmg, load_dialog, read_recent, remember, save_dialog, startup_path};
pub use mime::register_mime;
