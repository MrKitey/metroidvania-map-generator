//! Location of the app files and JSON reading and writing.

use serde::Serialize;
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

/// Folder created in the user's Documents to hold the app files.
const APP_DIR: &str = "Metroidvania Map Generator";

/// `Documents/<APP_DIR>`, or the working directory when Documents is unknown.
pub fn app_dir() -> PathBuf {
    dirs::document_dir()
        .map(|d| d.join(APP_DIR))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, Box<dyn Error>> {
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

/// Writes `value` as indented JSON to `file` in the app folder, creating the folder
/// if needed. Returns the full path.
pub fn write_json<T: Serialize>(file: &str, value: &T) -> Result<PathBuf, Box<dyn Error>> {
    let dir = app_dir();
    fs::create_dir_all(&dir)?;
    let path = dir.join(file);
    fs::write(&path, serde_json::to_string_pretty(value)?)?;
    Ok(path)
}
