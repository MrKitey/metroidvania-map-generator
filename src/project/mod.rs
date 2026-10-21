//! Project files: one JSON document with the `.mmg` extension.
//!
//! The file holds the map, the generation settings and the biomes. Autosave preferences
//! stay in settings.json and are not written here.

mod file;
mod io;
mod mime;

#[cfg(test)]
mod tests;

pub use file::{Loaded, ProjectFile, read, write};
pub use io::{forget, is_mmg, load_dialog, read_recent, remember, save_dialog, startup_path};
pub use mime::register_mime;
