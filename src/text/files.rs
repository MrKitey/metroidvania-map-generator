//! File and folder names, and the dialog filters built on them.

/// Folder created in the user's Documents to hold the app files.
pub const APP_DIR: &str = "Metroidvania Map Generator";
pub const BIOMES: &str = "biomes.json";
pub const SETTINGS: &str = "settings.json";
pub const JSON_FILTER: &str = "JSON";
pub const JSON_EXT: &str = "json";
pub const ZIP_FILTER: &str = "Zip";
pub const ZIP_EXT: &str = "zip";
pub const MMG_FILTER: &str = "MMG";
pub const MMG_EXT: &str = "mmg";
pub const RECENT: &str = "recent.json";
pub const MIME_STAMP: &str = "mime.stamp";
pub const MIME_XML: &str = "metroidvania-mmg.xml";
/// Old package name. xdg-mime rejects it because it has no vendor prefix.
pub const MIME_XML_OLD: &str = "mmg.xml";
/// Files inside the export zip.
pub const EXPORT_PNG: &str = "map.png";
pub const EXPORT_SVG: &str = "map.svg";
pub const EXPORT_JSON: &str = "map.json";
/// Files inside a project `.mmg` zip.
pub const PROJECT_JSON: &str = "project.json";
pub const PROJECT_THUMB: &str = "thumbnail.png";
pub const PROJECT_BIOMES: &str = "biomes.json";
pub const PROJECT_ROOMS: &str = "rooms.json";

/// Default name of the export zip.
pub fn zip_name(seed: u64) -> String {
    format!("map-{seed}.{ZIP_EXT}")
}
