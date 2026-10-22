//! Side panel to edit generation settings and special-room icons.

mod fields;
mod icons_tab;
mod settings_tab;
mod widgets;

use crate::common::EditorAction;
use crate::icon_lib::IconDef;
use crate::icons::Icons;
use crate::settings::{self, Settings};
use crate::text::{self, status};
use crate::world::{Sizes, World};
use fields::Fields;
pub use fields::{MAX_ROOM_SIDE, SizeFields};
pub use icons_tab::IconsTab;
pub use widgets::{PANEL_W, size_list};

const TAB_SETTINGS: usize = 0;
const TAB_SPECIAL: usize = 1;

pub struct Editor {
    pub open: bool,
    tab: usize,
    fields: Fields,
    /// Shows the current seed, can be edited and applied with "Use seed".
    seed: String,
    /// Feedback message shown at the bottom of the panel.
    pub status: String,
    pub icons_tab: IconsTab,
}

impl Editor {
    pub fn new(seed: u64, settings: &Settings) -> Editor {
        Editor {
            open: false,
            tab: TAB_SETTINGS,
            fields: Fields::new(settings),
            seed: seed.to_string(),
            status: String::new(),
            icons_tab: IconsTab::new(),
        }
    }

    pub fn special_tab_open(&self) -> bool {
        self.open && self.tab == TAB_SPECIAL
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
    pub fn settings(&self) -> Settings {
        self.fields.settings()
    }

    /// Called after each regeneration so the field shows the seed in use.
    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed.to_string();
    }

    /// Replaces the generation fields after a project load. The panel stays as it was.
    /// Keeps the current UI theme, which is not part of a project file.
    pub fn apply_loaded(&mut self, seed: u64, settings: &Settings) {
        let dark = self.fields.dark;
        self.fields = Fields::new(settings);
        self.fields.dark = dark;
        self.set_seed(seed);
    }

    /// Draws the panel and returns what the user asked for, if anything.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        special: &mut Vec<IconDef>,
        world: &mut World,
        icons: &mut Icons,
        legend_icons: &mut Icons,
    ) -> Option<EditorAction> {
        ui.heading(text::editor::WINDOW_TITLE);
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, TAB_SETTINGS, text::editor::TAB_SETTINGS);
            ui.selectable_value(
                &mut self.tab,
                TAB_SPECIAL,
                text::editor::TAB_SPECIAL_ROOM,
            );
        });
        ui.separator();

        let mut action = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| match self.tab {
                TAB_SPECIAL => {
                    self.icons_tab
                        .draw(ui, special, world, icons, legend_icons);
                }
                _ => {
                    action =
                        settings_tab::draw(ui, &mut self.fields, &mut self.seed, &mut self.status);
                }
            });
        action
    }
}
