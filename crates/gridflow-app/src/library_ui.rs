//! Library browser: lists the global class library (~/.config/gridflow/lib),
//! inserts `use` lines, and saves classes defined in the current document into
//! a library file.

use eframe::egui::{self, Ui};
use gridflow_core::ast::Stmt;
use gridflow_core::library::FsLibrary;
use gridflow_core::ops::TextEdit;
use gridflow_core::parser::parse;

pub struct LibraryUi {
    pub target_file: String,
    pub status: Option<String>,
}

impl LibraryUi {
    pub fn new() -> Self {
        LibraryUi { target_file: "shared".into(), status: None }
    }

    /// Shows the pane. Returns a text edit to apply to the document (e.g.
    /// inserting a `use` line), if the user asked for one.
    pub fn show(&mut self, ui: &mut Ui, doc_text: &str, library: &FsLibrary) -> Option<TextEdit> {
        let mut result = None;
        ui.strong("Class library");
        ui.weak(library.root().display().to_string());
        ui.separator();

        let names = {
            use gridflow_core::library::LibraryProvider;
            library.list()
        };
        if names.is_empty() {
            ui.weak("Library is empty. Save a class from the current file below.");
        }
        for name in names {
            ui.horizontal(|ui| {
                ui.label(&name);
                let already = doc_text
                    .lines()
                    .any(|l| {
                        let l = l.trim();
                        l.starts_with("use ")
                            && l[4..].split(',').any(|n| n.trim() == name)
                    });
                if already {
                    ui.weak("(used)");
                } else if ui.small_button("use").clicked() {
                    result = Some(TextEdit::new(0, 0, format!("use {name}\n")));
                }
            });
        }

        ui.separator();
        ui.strong("Save class to library");
        let classes: Vec<(String, String)> = parse(doc_text)
            .stmts
            .iter()
            .filter_map(|s| match s {
                Stmt::Class(c) => Some((
                    c.name.to_string(),
                    doc_text[c.stmt_span.start..c.stmt_span.end].to_string(),
                )),
                _ => None,
            })
            .collect();
        if classes.is_empty() {
            ui.weak("No classes defined in the current file.");
        } else {
            ui.horizontal(|ui| {
                ui.label("Target file:");
                ui.text_edit_singleline(&mut self.target_file);
            });
            for (name, src) in classes {
                ui.horizontal(|ui| {
                    ui.label(&name);
                    if ui.small_button("save").clicked() {
                        let file = self.target_file.trim();
                        if file.is_empty() {
                            self.status = Some("Choose a target file name".into());
                        } else {
                            match library.append_class(file, &src) {
                                Ok(()) => {
                                    self.status =
                                        Some(format!("Saved `{name}` to {file}.gfd"))
                                }
                                Err(e) => self.status = Some(format!("Save failed: {e}")),
                            }
                        }
                    }
                });
            }
        }
        if let Some(s) = &self.status {
            ui.separator();
            ui.weak(s);
        }
        let _ = ui;
        result
    }
}

pub fn _unused(_: &egui::Context) {}
