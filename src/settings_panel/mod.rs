//! Side panel to edit generation settings. Special rooms live in the Tools panel.

mod fields;
mod icons_tab;
mod settings_tab;
mod widgets;

use crate::common::SettingsAction;
use crate::settings::{self, Settings as Saved};
use crate::text::{self, status};
use crate::world::Sizes;
use fields::Fields;
pub use fields::{MAX_ROOM_SIDE, SizeFields};
pub use icons_tab::IconsTab;
pub use widgets::{PANEL_W, size_list};

pub struct Settings {
    pub open: bool,
    fields: Fields,
    /// Shows the current seed, can be edited and applied with "Use seed".
    seed: String,
    /// Feedback message shown at the bottom of the panel.
    pub status: String,
    pub icons_tab: IconsTab,
}

impl Settings {
    pub fn new(seed: u64, saved: &Saved) -> Settings {
        Settings {
            open: false,
            fields: Fields::new(saved),
            seed: seed.to_string(),
            status: String::new(),
            icons_tab: IconsTab::new(),
        }
    }

    /// Normal rooms per biome, at least 1.
    pub fn min_rooms(&self) -> usize {
        self.fields.min_rooms()
    }

    /// Never below the minimum. `usize::MAX` means no limit.
    pub fn max_rooms(&self) -> usize {
        self.fields.max_rooms()
    }

    /// Size of one in-game screen in pixels.
    pub fn viewport(&self) -> (u32, u32) {
        self.fields.viewport()
    }

    /// Special-room icon size on the map, 1 to 10.
    pub fn icon_size(&self) -> u32 {
        self.fields.icon_size()
    }

    /// All allowed room sizes, ready for the generator.
    pub fn sizes(&self) -> Sizes {
        self.fields.room_sizes()
    }

    /// Editable size lists for the rooms panel.
    pub fn size_lists_mut(&mut self) -> (&mut SizeFields, &mut SizeFields, &mut SizeFields) {
        (
            &mut self.fields.sizes,
            &mut self.fields.boss_sizes,
            &mut self.fields.corridor_sizes,
        )
    }

    /// Persists the current fields and updates the status message.
    pub fn save_settings(&mut self) {
        self.status = match settings::save(&self.fields.settings()) {
            Ok(path) => status::saved_to(&path),
            Err(e) => status::error(e),
        };
    }

    /// Keeps the theme flag in sync with the header switch.
    pub fn set_dark(&mut self, dark: bool) {
        self.fields.dark = dark;
    }

    /// Current field values, validated, in the format saved to disk.
    pub fn settings(&self) -> Saved {
        self.fields.settings()
    }

    /// Called after each regeneration so the field shows the seed in use.
    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed.to_string();
    }

    /// Replaces the generation fields after a project load. The panel stays as it was.
    /// Keeps theme and icon size, which are not part of a project file.
    pub fn apply_loaded(&mut self, seed: u64, saved: &Saved) {
        let dark = self.fields.dark;
        let icon_size = self.fields.icon_size;
        self.fields = Fields::new(saved);
        self.fields.dark = dark;
        self.fields.icon_size = icon_size;
        self.set_seed(seed);
    }

    /// Draws the panel and returns what the user asked for, if anything.
    pub fn draw(&mut self, ui: &mut egui::Ui) -> Option<SettingsAction> {
        ui.add_space(8.0);
        ui.heading(text::settings::WINDOW_TITLE);
        ui.separator();

        let mut action = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                action = settings_tab::draw(ui, &mut self.fields, &mut self.seed, &mut self.status);
            });
        action
    }
}
