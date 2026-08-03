//! Markdown notes pane: renders the sidecar `<name>.notes.md` next to the
//! open diagram, with an edit/preview toggle.

use eframe::egui::{self, Ui};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::path::{Path, PathBuf};

pub struct NotesPane {
    pub text: String,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub editing: bool,
    cache: CommonMarkCache,
}

impl NotesPane {
    pub fn new() -> Self {
        NotesPane {
            text: String::new(),
            path: None,
            dirty: false,
            editing: false,
            cache: CommonMarkCache::default(),
        }
    }

    pub fn sidecar_path(gfd_path: &Path) -> PathBuf {
        let stem = gfd_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "diagram".into());
        gfd_path.with_file_name(format!("{stem}.notes.md"))
    }

    pub fn load_for(&mut self, gfd_path: &Path) {
        let path = Self::sidecar_path(gfd_path);
        self.text = std::fs::read_to_string(&path).unwrap_or_default();
        self.path = Some(path);
        self.dirty = false;
    }

    pub fn save(&mut self) {
        if let Some(path) = &self.path {
            if self.dirty && std::fs::write(path, &self.text).is_ok() {
                self.dirty = false;
            }
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.strong("Notes");
            if let Some(p) = &self.path {
                ui.weak(p.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default());
            }
            if self.dirty {
                ui.weak("•");
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = if self.editing { "Preview" } else { "Edit" };
                if ui.small_button(label).clicked() {
                    self.editing = !self.editing;
                    if !self.editing {
                        self.save();
                    }
                }
            });
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("notes-scroll")
            .show(ui, |ui| {
                if self.editing {
                    let response = ui.add(
                        egui::TextEdit::multiline(&mut self.text)
                            .font(egui::FontId::monospace(13.0))
                            .desired_width(f32::INFINITY)
                            .desired_rows(30),
                    );
                    if response.changed() {
                        self.dirty = true;
                    }
                } else if self.text.trim().is_empty() {
                    ui.weak("No notes yet. Click Edit to write some — they save next to the .gfd file.");
                } else {
                    CommonMarkViewer::new().show(ui, &mut self.cache, &self.text);
                }
            });
    }
}
