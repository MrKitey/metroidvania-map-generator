//! Project files: a zip archive with the `.mmg` extension.
//!
//! The archive holds `project.json`, `thumbnail.png`, `biomes.json` and `rooms.json`.
//! Autosave preferences stay in settings.json and are not written here.

mod file;
mod io;
mod mime;

#[cfg(test)]
mod tests;

pub use file::{Loaded, ProjectFile, read, utc_now, write};
pub use io::{forget, is_mmg, load_dialog, read_recent, remember, save_dialog, startup_path};
pub use mime::register_mime;
