//! Project name, path, dirty flag and the notices shown in the header.

use super::{MAX_CELLS, MIN_CELLS, START_CELLS, build_world};
use crate::export::{self, MapData};
use crate::layer::LayerStack;
use crate::layout::Layout;
use crate::project::{self, Loaded, ProjectFile};
use crate::storage;
use crate::text::status;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Seconds the export result stays in the header.
const NOTICE_TIME: f64 = 4.0;

pub(super) struct Session {
    pub(super) name: String,
    path: Option<PathBuf>,
    created_at: String,
    pub(super) dirty: bool,
    dirty_at: f64,
    pub(super) renaming: bool,
    pub(super) rename_focus: bool,
    name_before: String,
    saved: Option<ProjectFile>,
    last: Option<ProjectFile>,
    notice: Option<(String, f64)>,
    pending_notice: Option<String>,
}

impl Session {
    pub(super) fn new(name: String) -> Session {
        Session {
            name,
            path: None,
            created_at: project::utc_now(),
            dirty: true,
            dirty_at: 0.0,
            renaming: false,
            rename_focus: false,
            name_before: String::new(),
            saved: None,
            last: None,
            notice: None,
            pending_notice: None,
        }
    }

    pub(super) fn take_pending(&mut self, now: f64) {
        if let Some(text) = self.pending_notice.take() {
            self.notice = Some((text, now));
        }
    }

    pub(super) fn notice_visible(&self, now: f64) -> bool {
        self.notice
            .as_ref()
            .is_some_and(|(_, at)| now - at < NOTICE_TIME)
    }

    pub(super) fn notice_text(&self, now: f64) -> Option<String> {
        self.notice
            .as_ref()
            .filter(|(_, at)| now - at < NOTICE_TIME)
            .map(|(text, _)| text.clone())
    }

    pub(super) fn start_rename(&mut self) {
        self.name_before = self.name.clone();
        self.renaming = true;
        self.rename_focus = true;
    }

    pub(super) fn cancel_rename(&mut self) {
        self.name = self.name_before.clone();
        self.renaming = false;
    }

    pub(super) fn finish_rename(&mut self, seed: u64) {
        self.renaming = false;
        let trimmed = self.name.trim().to_owned();
        self.name = if trimmed.is_empty() {
            seed.to_string()
        } else {
            trimmed
        };
    }

    pub(super) fn set_notice(&mut self, text: String, now: f64) {
        self.notice = Some((text, now));
    }

    fn set_pending(&mut self, text: String) {
        self.pending_notice = Some(text);
    }

    fn reset_new(&mut self, name: String, now: f64) {
        self.name = name;
        self.path = None;
        self.created_at = project::utc_now();
        self.renaming = false;
        self.saved = None;
        self.last = None;
        self.dirty = true;
        self.dirty_at = now;
    }

    fn install(&mut self, name: String, path: Option<PathBuf>, file: ProjectFile) {
        self.name = name;
        self.path = path;
        self.created_at = file.created_at().to_owned();
        self.renaming = false;
        self.saved = Some(file.clone());
        self.last = Some(file);
        self.dirty = false;
    }

    fn mark_saved(&mut self, path: &Path, file: ProjectFile) {
        self.path = Some(path.to_owned());
        self.saved = Some(file.clone());
        self.last = Some(file);
        self.dirty = false;
    }

    fn sync(&mut self, current: ProjectFile, now: f64) {
        if self.saved.as_ref() != Some(&current) {
            if self.last.as_ref() != Some(&current) {
                self.dirty_at = now;
            }
            self.dirty = true;
        } else {
            self.dirty = false;
        }
        self.last = Some(current);
    }

    fn confirm_discard(&self) -> bool {
        if !self.dirty {
            return true;
        }
        rfd::MessageDialog::new()
            .set_title(status::DISCARD_TITLE)
            .set_description(status::DISCARD)
            .set_buttons(rfd::MessageButtons::YesNo)
            .show()
            == rfd::MessageDialogResult::Yes
    }

    /// What the autosave timer wants this frame.
    fn autosave(&self, enabled: bool, secs: u64, now: f64) -> Autosave {
        if !enabled || !self.dirty || self.path.is_none() {
            return Autosave::Skip;
        }
        let delay = secs as f64;
        let elapsed = now - self.dirty_at;
        if elapsed >= delay {
            Autosave::Write(self.path.clone().unwrap())
        } else {
            Autosave::Wait((delay - elapsed).max(0.05))
        }
    }

    fn postpone(&mut self, now: f64) {
        self.dirty_at = now;
    }
}

enum Autosave {
    Skip,
    Write(PathBuf),
    Wait(f64),
}

/// Most recent project that is still on disk. Missing entries are dropped.
fn latest_project() -> Option<PathBuf> {
    let dir = storage::app_dir();
    for entry in project::read_recent(&dir) {
        if entry.path.is_file() {
            return Some(entry.path);
        }
        let _ = project::forget(&dir, &entry.path);
    }
    None
}

impl super::App {
    pub(super) fn open_startup(&mut self) {
        let path = project::startup_path().or_else(latest_project);
        let Some(path) = path else {
            return;
        };
        match project::read(&path) {
            Ok(loaded) => {
                self.session.set_pending(status::loaded_from(&path));
                self.apply_loaded(loaded, Some(path));
            }
            Err(err) => self.session.set_pending(status::error(err)),
        }
    }

    pub(super) fn new_project(&mut self, now: f64) {
        if !self.session.confirm_discard() {
            return;
        }
        self.seed = crate::world::time_seed();
        let name = self.seed.to_string();
        self.cells = START_CELLS;
        self.editor.set_seed(self.seed);
        self.layers =
            LayerStack::from_world(build_world(self.cells, self.seed, &self.biomes, &self.editor));
        self.interact.clear();
        self.undo.clear();
        self.fit = true;
        self.session.reset_new(name, now);
    }

    pub(super) fn load_project(&mut self, now: f64) {
        if !self.session.confirm_discard() {
            return;
        }
        let Some(path) = project::load_dialog() else {
            return;
        };
        self.load_path(path, now, false);
    }

    pub(super) fn save_project(&mut self, now: f64) {
        if let Some(path) = self.session.path.clone() {
            self.write_path(&path, now, true);
            return;
        }
        let Some(path) = project::save_dialog(&self.session.name, self.seed) else {
            return;
        };
        self.write_path(&path, now, true);
    }

    /// `confirm` asks before replacing a dirty project. The load dialog confirms first.
    pub(super) fn load_path(&mut self, path: PathBuf, now: f64, confirm: bool) {
        if confirm && !self.session.confirm_discard() {
            return;
        }
        if !path.is_file() {
            let _ = project::forget(&storage::app_dir(), &path);
            self.session.set_notice(status::MISSING.to_owned(), now);
            return;
        }
        match project::read(&path) {
            Ok(loaded) => {
                self.session.set_notice(status::loaded_from(&path), now);
                self.apply_loaded(loaded, Some(path));
            }
            Err(err) => self.session.set_notice(status::error(err), now),
        }
    }

    fn apply_loaded(&mut self, loaded: Loaded, path: Option<PathBuf>) {
        let current = self.editor.settings();
        let mut settings = loaded.settings;
        settings.autosave = current.autosave;
        settings.autosave_secs = current.autosave_secs;
        self.editor.apply_loaded(loaded.seed, &settings);
        self.seed = loaded.seed;
        self.cells = loaded.cells.clamp(MIN_CELLS, MAX_CELLS);
        self.grid = loaded.grid;
        self.biomes = loaded.biomes;
        self.layers = loaded.layers;
        self.interact.clear();
        self.undo.clear();
        self.fit = true;
        self.session.created_at = loaded.created_at;
        let modified_at = loaded.modified_at;
        let file = self.project_file_named(&loaded.name);
        self.session.install(loaded.name, path, file);
        if let Some(path) = &self.session.path {
            let _ = project::remember(
                &storage::app_dir(),
                path,
                &self.session.name,
                &modified_at,
            );
        }
    }

    /// `announce` shows the path in the header. Autosave stays quiet unless it fails.
    fn write_path(&mut self, path: &Path, now: f64, announce: bool) -> bool {
        let file = self.project_file();
        let settings = self.editor.settings();
        let layout = Layout::new(self.editor.viewport());
        let map = MapData {
            seed: self.seed,
            cells: self.cells,
            settings: &settings,
            layers: &self.layers,
            only_layer: None,
            biomes: &self.biomes,
            special: &self.special,
            layout: &layout,
        };
        let thumbnail = match export::thumbnail(&map) {
            Ok(png) => png,
            Err(err) => {
                self.session.set_notice(status::error(err), now);
                return false;
            }
        };
        match project::write(path, &file, &thumbnail) {
            Ok(modified_at) => {
                self.session.mark_saved(path, file);
                let _ = project::remember(
                    &storage::app_dir(),
                    path,
                    &self.session.name,
                    &modified_at,
                );
                if announce {
                    self.session.set_notice(status::saved_to(path), now);
                }
                true
            }
            Err(err) => {
                self.session.set_notice(status::error(err), now);
                false
            }
        }
    }

    fn project_file(&self) -> ProjectFile {
        self.project_file_named(&self.session.name)
    }

    fn project_file_named(&self, name: &str) -> ProjectFile {
        ProjectFile::capture(
            name,
            self.seed,
            self.cells,
            self.grid,
            &self.session.created_at,
            &self.editor.settings(),
            &self.biomes,
            &self.layers,
        )
    }

    pub(super) fn handle_dropped(&mut self, ctx: &egui::Context, now: f64) {
        let dropped: Vec<PathBuf> = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .map(|file| file.path().to_path_buf())
                .collect()
        });
        self.open_dropped(dropped, now);
    }

    /// Opens the first `.mmg`, or feeds an image to the Special Room tab when it is open.
    pub(super) fn open_dropped(&mut self, dropped: Vec<PathBuf>, now: f64) {
        if dropped.is_empty() {
            return;
        }
        if self.editor.special_tab_open() {
            let paths: Vec<_> = dropped.iter().cloned().collect();
            if self.editor.icons_tab.handle_drops(
                &paths,
                &mut self.special,
                &mut self.icons,
                &mut self.legend_icons,
            ) {
                if let Some(msg) = self.editor.icons_tab.take_status() {
                    self.session.set_notice(msg, now);
                }
                return;
            }
        }
        let Some(path) = dropped.into_iter().find(|path| project::is_mmg(path)) else {
            self.session.set_notice(status::NOT_MMG.to_owned(), now);
            return;
        };
        self.load_path(path, now, true);
    }

    pub(super) fn sync_dirty(&mut self, now: f64) {
        let current = self.project_file();
        self.session.sync(current, now);
    }

    pub(super) fn autosave(&mut self, ctx: &egui::Context, now: f64) {
        let settings = self.editor.settings();
        match self
            .session
            .autosave(settings.autosave, settings.autosave_secs, now)
        {
            Autosave::Skip => {}
            Autosave::Write(path) => {
                if !self.write_path(&path, now, false) {
                    self.session.postpone(now);
                    ctx.request_repaint_after(Duration::from_secs_f64(
                        settings.autosave_secs as f64,
                    ));
                }
            }
            Autosave::Wait(wait) => ctx.request_repaint_after(Duration::from_secs_f64(wait)),
        }
    }
}
