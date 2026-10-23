//! Tools panel body: Layers, Rooms, Biomes and Special rooms.

use super::App;
use crate::layout::Layout;
use crate::render::{self, Picker};
use crate::text;

/// Which page the Tools panel shows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ToolsTab {
    Rooms,
    Biomes,
    Layers,
    Special,
}

/// Open flag and the page remembered while the panel is closed.
pub(super) struct Tools {
    pub open: bool,
    pub tab: ToolsTab,
}

impl Tools {
    pub(super) fn new() -> Tools {
        Tools {
            open: false,
            tab: ToolsTab::Rooms,
        }
    }

    /// Special rooms can take a dropped image only while that tab is open.
    pub(super) fn special_open(&self) -> bool {
        self.open && self.tab == ToolsTab::Special
    }
}

impl App {
    /// Draws the Tools heading, the tabs and the current page.
    pub(super) fn draw_tools_body(&mut self, ui: &mut egui::Ui) -> Option<Picker> {
        ui.add_space(8.0);
        ui.heading(text::hud::TOOLS);
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.tools.tab, ToolsTab::Layers, text::hud::LAYERS);
            ui.selectable_value(&mut self.tools.tab, ToolsTab::Rooms, text::rooms::ROOMS);
            ui.selectable_value(&mut self.tools.tab, ToolsTab::Biomes, text::hud::BIOMES);
            ui.selectable_value(
                &mut self.tools.tab,
                ToolsTab::Special,
                text::settings::TAB_SPECIAL_ROOM,
            );
        });
        ui.separator();
        self.draw_tool_tab(ui, self.tools.tab)
    }

    /// Draws one tool tab. Returns the room list for the Rooms tab.
    fn draw_tool_tab(&mut self, ui: &mut egui::Ui, tab: ToolsTab) -> Option<Picker> {
        match tab {
            ToolsTab::Rooms => {
                let sizes = self.settings.sizes();
                let layout = Layout::new(self.settings.viewport());
                let (rooms, bosses, corridors) = self.settings.size_lists_mut();
                let (picker, started, changed) =
                    render::draw_room_list(ui, &sizes, &layout, rooms, bosses, corridors);
                if changed {
                    self.settings.save_settings();
                }
                if let Some((choice, size)) = started {
                    self.interact.placing = Some(choice);
                    self.interact.placing_size = size;
                }
                Some(picker)
            }
            ToolsTab::Biomes => {
                if let Some(action) =
                    self.biomes_tab
                        .draw(ui, &mut self.biomes, self.layers.active_world_mut())
                {
                    self.apply_action(action);
                }
                None
            }
            ToolsTab::Layers => {
                let action = super::layers_panel::draw_layers(
                    ui,
                    &mut self.layers,
                    &mut self.undo,
                    self.seed,
                    self.cells,
                    &self.biomes,
                    &self.layer_icons,
                );
                if action.changed {
                    self.interact.clear();
                }
                None
            }
            ToolsTab::Special => {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.settings.icons_tab.draw(
                            ui,
                            &mut self.special,
                            self.layers.active_world_mut(),
                            &mut self.icons,
                            &mut self.legend_icons,
                        );
                    });
                None
            }
        }
    }
}
