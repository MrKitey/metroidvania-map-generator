//! Common types used across the application.

/// What the panel asks the app to do.
pub enum SettingsAction {
    /// Generate a new map with a fresh seed.
    Regenerate,
    /// Generate a map with this seed.
    UseSeed(u64),
}
