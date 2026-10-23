//! Every text of the app: window title, labels, messages, file names and default names.
//!
//! One module per area. Fixed texts are constants, texts built from values are small
//! functions, so a format string never appears in the drawing code. Not here: SVG
//! markup, JSON field names, panic messages and test data.

pub mod app;
pub mod biomes;
pub mod files;
pub mod hud;
pub mod icons;
pub mod rooms;
pub mod settings;
pub mod status;
