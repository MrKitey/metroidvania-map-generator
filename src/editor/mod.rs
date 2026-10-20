//! Side panel to edit generation settings and biomes.

mod biomes_tab;
mod fields;
mod settings_tab;
mod widgets;

use crate::biome::Biome;
use crate::settings::Settings;
use crate::text;
use crate::world::Sizes;
use biomes_tab::BiomesTab;
use fields::Fields;
pub use widgets::PANEL_W;

/// What the panel asks the app to do.
pub enum EditorAction {
    /// Generate a new map with a fresh seed.
    Regenerate,
    /// Generate a map with this seed.
    UseSeed(u64),
}

pub struct Editor {
    pub open: bool,
    tab: usize,
    fields: Fields,
    /// Shows the current seed, can be edited and applied with "Use seed".
    seed: String,
    biomes_tab: BiomesTab,
    /// Feedback message shown at the bottom of the panel.
    status: String,
}

impl Editor {
    pub fn new(seed: u64, settings: &Settings) -> Editor {
        Editor {
            open: false,
            tab: 0,
            fields: Fields::new(settings),
            seed: seed.to_string(),
            biomes_tab: BiomesTab::new(),
            status: String::new(),
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

    /// All allowed room sizes, ready for the generator.
    pub fn sizes(&self) -> Sizes {
        self.fields.room_sizes()
    }

    /// Current field values, validated, in the format saved to disk.
    pub fn settings(&self) -> Settings {
        self.fields.settings()
    }

    /// Called after each regeneration so the field shows the seed in use.
    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed.to_string();
    }

    /// Draws the panel and returns what the user asked for, if anything.
    pub fn draw(&mut self, ui: &mut egui::Ui, biomes: &mut Vec<Biome>) -> Option<EditorAction> {
        ui.heading(text::editor::WINDOW_TITLE);
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, 0, text::editor::TABS[0]);
            ui.selectable_value(&mut self.tab, 1, text::editor::TABS[1]);
        });
        ui.separator();

        let mut action = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                action = if self.tab == 0 {
                    settings_tab::draw(ui, &mut self.fields, &mut self.seed, &mut self.status)
                } else {
                    self.biomes_tab.draw(ui, biomes, &mut self.status)
                };
            });
        action
    }
}
