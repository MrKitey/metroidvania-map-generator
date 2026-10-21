//! Settings fields kept as raw text, and their validation.

use crate::settings::{
    DEFAULT_AUTOSAVE_SECS, DEFAULT_BOSS_SIZES, DEFAULT_CORRIDOR_SIZES, DEFAULT_MAX_ROOMS,
    DEFAULT_MIN_ROOMS, DEFAULT_ROOM_SIZES, DEFAULT_VIEWPORT, MAX_AUTOSAVE_SECS, MIN_AUTOSAVE_SECS,
    Settings,
};
use crate::world::Sizes;

/// Upper bound of the minimum, to keep generation fast.
pub const MAX_MIN_ROOMS: usize = 50;
/// Largest room side, in cells.
pub const MAX_ROOM_SIDE: u32 = 100;

/// One width and one height field per allowed room size.
pub type SizeFields = Vec<(String, String)>;

/// Fields are kept as raw text so the user can type freely. They are parsed on read,
/// and invalid input falls back to the default value.
pub struct Fields {
    pub viewport_w: String,
    pub viewport_h: String,
    pub min_rooms: String,
    pub max_rooms: String,
    /// Allowed room sizes.
    pub sizes: SizeFields,
    /// Allowed boss room sizes.
    pub boss_sizes: SizeFields,
    /// Allowed corridor sizes.
    pub corridor_sizes: SizeFields,
    pub autosave: bool,
    /// Delay in seconds, kept as typed.
    pub autosave_secs: String,
}

/// Width and height field texts for each size.
pub fn size_fields(sizes: &[(u32, u32)]) -> SizeFields {
    sizes
        .iter()
        .map(|(w, h)| (w.to_string(), h.to_string()))
        .collect()
}

/// Parses one room side, between 1 and `MAX_ROOM_SIDE`.
fn parse_side(text: &str) -> Option<u32> {
    text.trim()
        .parse()
        .ok()
        .filter(|v| (1..=MAX_ROOM_SIDE).contains(v))
}

/// Valid sizes only. Falls back to `defaults` if none is valid.
fn valid_sizes(fields: &[(String, String)], defaults: &[(u32, u32)]) -> Vec<(u32, u32)> {
    let sizes: Vec<(u32, u32)> = fields
        .iter()
        .filter_map(|(w, h)| Some((parse_side(w)?, parse_side(h)?)))
        .collect();
    if sizes.is_empty() {
        defaults.to_vec()
    } else {
        sizes
    }
}

impl Fields {
    pub fn new(settings: &Settings) -> Fields {
        Fields {
            viewport_w: settings.viewport.0.to_string(),
            viewport_h: settings.viewport.1.to_string(),
            min_rooms: settings.min_rooms.to_string(),
            max_rooms: settings.max_rooms.to_string(),
            sizes: size_fields(&settings.room_sizes),
            boss_sizes: size_fields(&settings.boss_sizes),
            corridor_sizes: size_fields(&settings.corridor_sizes),
            autosave: settings.autosave,
            autosave_secs: settings.autosave_secs.to_string(),
        }
    }

    /// Clamped between 1 and `MAX_MIN_ROOMS`.
    pub fn min_rooms(&self) -> usize {
        self.min_rooms
            .trim()
            .parse()
            .unwrap_or(DEFAULT_MIN_ROOMS)
            .clamp(1, MAX_MIN_ROOMS)
    }

    /// 0 means no limit. Never below the minimum otherwise.
    fn max_rooms_setting(&self) -> usize {
        match self.max_rooms.trim().parse().unwrap_or(DEFAULT_MAX_ROOMS) {
            0 => 0,
            n => n.max(self.min_rooms()),
        }
    }

    /// Same as the setting, with `usize::MAX` for no limit.
    pub fn max_rooms(&self) -> usize {
        match self.max_rooms_setting() {
            0 => usize::MAX,
            n => n,
        }
    }

    /// Seconds before an autosave. Invalid text uses the default, then the value is clamped.
    pub fn autosave_secs(&self) -> u64 {
        self.autosave_secs
            .trim()
            .parse()
            .unwrap_or(DEFAULT_AUTOSAVE_SECS)
            .clamp(MIN_AUTOSAVE_SECS, MAX_AUTOSAVE_SECS)
    }

    /// Size of one in-game screen in pixels. Zero or invalid values use the default.
    pub fn viewport(&self) -> (u32, u32) {
        let parse =
            |s: &str, default: u32| s.trim().parse().ok().filter(|&v| v > 0).unwrap_or(default);
        (
            parse(&self.viewport_w, DEFAULT_VIEWPORT.0),
            parse(&self.viewport_h, DEFAULT_VIEWPORT.1),
        )
    }

    /// All allowed sizes, ready for the generator. Each list falls back to its defaults
    /// if no size is valid.
    pub fn room_sizes(&self) -> Sizes {
        Sizes {
            rooms: valid_sizes(&self.sizes, DEFAULT_ROOM_SIZES),
            boss: valid_sizes(&self.boss_sizes, DEFAULT_BOSS_SIZES),
            corridor: valid_sizes(&self.corridor_sizes, DEFAULT_CORRIDOR_SIZES),
        }
    }

    /// Current values, validated, in the format saved to disk.
    pub fn settings(&self) -> Settings {
        let sizes = self.room_sizes();
        Settings {
            viewport: self.viewport(),
            min_rooms: self.min_rooms(),
            max_rooms: self.max_rooms_setting(),
            room_sizes: sizes.rooms,
            boss_sizes: sizes.boss,
            corridor_sizes: sizes.corridor,
            autosave: self.autosave,
            autosave_secs: self.autosave_secs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields() -> Fields {
        Fields::new(&Settings::default())
    }

    #[test]
    fn defaults_round_trip() {
        let settings = fields().settings();
        let defaults = Settings::default();
        assert_eq!(settings.viewport, defaults.viewport);
        assert_eq!(settings.min_rooms, defaults.min_rooms);
        assert_eq!(settings.max_rooms, defaults.max_rooms);
        assert_eq!(settings.room_sizes, defaults.room_sizes);
        assert_eq!(settings.autosave, defaults.autosave);
        assert_eq!(settings.autosave_secs, defaults.autosave_secs);
    }

    #[test]
    fn autosave_delay_is_clamped() {
        let mut f = fields();
        f.autosave_secs = "1".to_owned();
        assert_eq!(f.autosave_secs(), MIN_AUTOSAVE_SECS);
        f.autosave_secs = "99999".to_owned();
        assert_eq!(f.autosave_secs(), MAX_AUTOSAVE_SECS);
        f.autosave_secs = "abc".to_owned();
        assert_eq!(f.autosave_secs(), DEFAULT_AUTOSAVE_SECS);
    }

    #[test]
    fn invalid_text_falls_back_to_defaults() {
        let mut f = fields();
        f.viewport_w = "abc".to_owned();
        f.viewport_h = "0".to_owned();
        f.min_rooms = "-1".to_owned();
        assert_eq!(f.viewport(), DEFAULT_VIEWPORT);
        assert_eq!(f.min_rooms(), DEFAULT_MIN_ROOMS);
    }

    #[test]
    fn min_rooms_is_clamped() {
        let mut f = fields();
        f.min_rooms = "0".to_owned();
        assert_eq!(f.min_rooms(), 1);
        f.min_rooms = "999".to_owned();
        assert_eq!(f.min_rooms(), MAX_MIN_ROOMS);
    }

    #[test]
    fn max_rooms_never_below_min() {
        let mut f = fields();
        f.min_rooms = "20".to_owned();
        f.max_rooms = "10".to_owned();
        assert_eq!(f.max_rooms(), 20);
        assert_eq!(f.settings().max_rooms, 20);
        f.max_rooms = "0".to_owned();
        assert_eq!(f.max_rooms(), usize::MAX);
        assert_eq!(f.settings().max_rooms, 0);
    }

    #[test]
    fn invalid_sizes_are_dropped() {
        let mut f = fields();
        f.sizes = vec![
            ("2".to_owned(), "3".to_owned()),
            ("0".to_owned(), "1".to_owned()),
            ("101".to_owned(), "1".to_owned()),
            ("x".to_owned(), "1".to_owned()),
        ];
        assert_eq!(f.room_sizes().rooms, vec![(2, 3)]);
        f.sizes.clear();
        assert_eq!(f.room_sizes().rooms, DEFAULT_ROOM_SIZES);
    }
}
