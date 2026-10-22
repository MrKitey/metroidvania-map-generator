//! Biomes panel: one folding entry per biome, plus add, import and save.

use crate::biome::{self, Biome, MAX_COUNT, MIN_CORRIDORS};
use crate::common::EditorAction;
use crate::text::{biomes, editor as text, files, rooms, status};
use crate::world::World;

pub struct BiomesTab {
    status: String,
}

impl BiomesTab {
    pub fn new() -> BiomesTab {
        BiomesTab {
            status: String::new(),
        }
    }

    /// Returns `Regenerate` only after a successful import.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        biomes_list: &mut Vec<Biome>,
        world: &mut World,
    ) -> Option<EditorAction> {
        let mut action = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                action = self.draw_list(ui, biomes_list, world);
            });
        action
    }

    fn draw_list(
        &mut self,
        ui: &mut egui::Ui,
        biomes_list: &mut Vec<Biome>,
        world: &mut World,
    ) -> Option<EditorAction> {
        let mut action = None;
        let mut remove = None;
        let mut changed = false;
        let can_delete = biomes_list.len() > 1;

        for (i, b) in biomes_list.iter_mut().enumerate() {
            let used = world.uses_biome(i);
            ui.push_id(i, |ui| {
                egui::CollapsingHeader::new(b.name.clone())
                    .id_salt(i)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(text::NAME);
                            if ui.text_edit_singleline(&mut b.name).changed() {
                                changed = true;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(text::COLOR);
                            let mut rgb = [b.fill.r(), b.fill.g(), b.fill.b()];
                            if ui.color_edit_button_srgb(&mut rgb).changed() {
                                b.fill = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                                changed = true;
                            }
                        });
                        if count_slider_changed(ui, rooms::SECRET_ROOMS, 0, &mut b.secrets) {
                            changed = true;
                        }
                        if count_slider_changed(ui, rooms::BOSS_ROOMS, 0, &mut b.bosses) {
                            changed = true;
                        }
                        if count_slider_changed(ui, rooms::SAVE_ROOMS, 0, &mut b.saves) {
                            changed = true;
                        }
                        if count_slider_changed(ui, rooms::PORTAL_ROOMS, 0, &mut b.portals) {
                            changed = true;
                        }
                        if count_slider_changed(
                            ui,
                            rooms::CORRIDORS,
                            MIN_CORRIDORS,
                            &mut b.corridors,
                        ) {
                            changed = true;
                        }
                        if ui
                            .add_enabled(can_delete && !used, egui::Button::new(text::DELETE))
                            .clicked()
                        {
                            remove = Some(i);
                        }
                    });
            });
        }
        ui.separator();

        ui.horizontal(|ui| {
            if ui.button(text::ADD).clicked() {
                biomes_list.push(Biome::new(biomes::NEW, 128, 128, 128));
                changed = true;
            }
            if ui.button(text::IMPORT).clicked() {
                let picked = rfd::FileDialog::new()
                    .add_filter(files::JSON_FILTER, &[files::JSON_EXT])
                    .pick_file();
                if let Some(path) = picked {
                    match biome::read(&path) {
                        Ok(list) => {
                            *biomes_list = list;
                            action = Some(EditorAction::Regenerate);
                            self.status = match biome::save(biomes_list) {
                                Ok(_) => status::imported(&path),
                                Err(e) => status::error(e),
                            };
                        }
                        Err(e) => self.status = status::error(e),
                    }
                }
            }
        });

        if let Some(i) = remove {
            if biomes_list.len() > 1 && !world.uses_biome(i) {
                biomes_list.remove(i);
                world.reindex_after_biome_removal(i);
                changed = true;
            } else if biomes_list.len() <= 1 {
                self.status = status::AT_LEAST_ONE_BIOME.to_owned();
            }
        }
        if changed {
            self.status = match biome::save(biomes_list) {
                Ok(path) => status::saved_to(&path),
                Err(e) => status::error(e),
            };
        }
        if !self.status.is_empty() {
            ui.label(self.status.as_str());
        }
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biome::Biome;

    fn input(time: f64) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(400.0, 800.0),
            )),
            time: Some(time),
            ..Default::default()
        }
    }

    fn draw(
        ctx: &egui::Context,
        time: f64,
        events: Vec<egui::Event>,
        tab: &mut BiomesTab,
        list: &mut Vec<Biome>,
        world: &mut World,
    ) -> egui::FullOutput {
        let mut input = input(time);
        input.events = events;
        ctx.run_ui(input, |ui| {
            tab.draw(ui, list, world);
        })
    }

    /// Center of a painted string, if that string is on screen.
    fn text_center(output: &egui::FullOutput, text: &str) -> Option<egui::Pos2> {
        fn walk(shape: &egui::Shape, text: &str) -> Option<egui::Pos2> {
            match shape {
                egui::Shape::Text(painted) if painted.galley.job.text == text => {
                    Some(painted.pos + painted.galley.rect.center().to_vec2())
                }
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| walk(shape, text)),
                _ => None,
            }
        }
        output
            .shapes
            .iter()
            .find_map(|clipped| walk(&clipped.shape, text))
    }

    fn shows(output: &egui::FullOutput, text: &str) -> bool {
        text_center(output, text).is_some()
    }

    fn click(pos: egui::Pos2) -> Vec<egui::Event> {
        let button = egui::PointerButton::Primary;
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            },
            egui::Event::PointerButton {
                pos,
                button,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            },
        ]
    }

    #[test]
    fn renaming_a_biome_keeps_its_section_open() {
        let ctx = egui::Context::default();
        let mut list = vec![Biome::new("Castle", 1, 2, 3)];
        let mut tab = BiomesTab::new();
        let mut world = World::default();

        let closed = draw(&ctx, 0.0, Vec::new(), &mut tab, &mut list, &mut world);
        let header = text_center(&closed, "Castle").unwrap();
        closed.drop_without_applying_deltas();

        draw(
            &ctx,
            0.05,
            click(header),
            &mut tab,
            &mut list,
            &mut world,
        )
        .drop_without_applying_deltas();
        let open = draw(&ctx, 1.0, Vec::new(), &mut tab, &mut list, &mut world);
        let name_visible = shows(&open, text::NAME);
        open.drop_without_applying_deltas();
        assert!(name_visible);

        list[0].name.push('x');
        let renamed = draw(&ctx, 2.0, Vec::new(), &mut tab, &mut list, &mut world);
        let name_still_visible = shows(&renamed, text::NAME);
        renamed.drop_without_applying_deltas();
        assert!(name_still_visible);
    }
}

fn count_slider_changed(ui: &mut egui::Ui, label: &str, min: u32, value: &mut u32) -> bool {
    ui.add(
        egui::Slider::new(value, min..=MAX_COUNT)
            .integer()
            .text(label),
    )
    .changed()
}
