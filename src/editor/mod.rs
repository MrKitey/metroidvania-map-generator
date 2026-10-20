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
use macroquad::prelude::*;
use macroquad::ui::{Skin, root_ui, widgets as ui_widgets};
use widgets::PANEL_MARGIN;
pub use widgets::PANEL_W;

/// Widget ids of the window and its tab bar. The other ids are in the tab modules.
const WINDOW_ID: u64 = 1;
const TABS_ID: u64 = 7;
const TABS_SIZE: Vec2 = vec2(widgets::INNER_W, 24.0);
/// Vertical space below the tabs, in pixels.
const TABS_GAP: f32 = 8.0;

/// What the panel asks the app to do.
pub enum EditorAction {
    /// Generate a new map with a fresh seed.
    Regenerate,
    /// Generate a map with this seed.
    UseSeed(u64),
}

pub struct Editor {
    pub open: bool,
    fields: Fields,
    /// Shows the current seed, can be edited and applied with "Use seed".
    seed: String,
    biomes_tab: BiomesTab,
    /// Default skin with `PANEL_MARGIN`, applied to the panel only.
    skin: Skin,
    /// Feedback message shown at the bottom of the panel.
    status: String,
}

impl Editor {
    pub fn new(seed: u64, settings: &Settings) -> Editor {
        Editor {
            open: false,
            fields: Fields::new(settings),
            seed: seed.to_string(),
            biomes_tab: BiomesTab::new(),
            skin: Skin {
                margin: PANEL_MARGIN,
                ..root_ui().default_skin()
            },
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

    /// True when the cursor is over the open panel, so map clicks must be ignored.
    pub fn is_mouse_over(&self) -> bool {
        let (x, y) = mouse_position();
        self.open && root_ui().is_mouse_over(vec2(x, y))
    }

    /// Draws the panel and returns what the user asked for, if anything.
    pub fn draw(&mut self, biomes: &mut Vec<Biome>) -> Option<EditorAction> {
        let mut action = None;
        let pos = vec2(screen_width() - PANEL_W, 0.0);
        let size = vec2(PANEL_W, screen_height());
        self.biomes_tab.sync(biomes);

        root_ui().push_skin(&self.skin);
        ui_widgets::Window::new(WINDOW_ID, pos, size)
            .label(text::editor::WINDOW_TITLE)
            .movable(false)
            .ui(&mut root_ui(), |ui| {
                let tab = ui.tabbar(TABS_ID, TABS_SIZE, text::editor::TABS);
                ui.canvas().request_space(vec2(0.0, TABS_GAP));
                action = if tab == 0 {
                    settings_tab::draw(ui, &mut self.fields, &mut self.seed, &mut self.status)
                } else {
                    self.biomes_tab.draw(ui, biomes, &mut self.status)
                };
            });
        root_ui().pop_skin();
        action
    }
}
