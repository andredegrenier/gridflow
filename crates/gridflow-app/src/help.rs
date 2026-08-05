//! In-app documentation browser: the docs/ book is compiled into the binary
//! and rendered with the same markdown engine as the notes pane, with a topic
//! sidebar and full-text search.

use eframe::egui::{self, Context};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

pub const TOPICS: &[(&str, &str)] = &[
    ("Getting Started", include_str!("../../../docs/01-getting-started.md")),
    ("Language Reference", include_str!("../../../docs/02-language-reference.md")),
    ("Shapes", include_str!("../../../docs/03-shapes.md")),
    ("Placement & Layout", include_str!("../../../docs/04-placement-and-layout.md")),
    ("Variables & Classes", include_str!("../../../docs/05-variables-and-classes.md")),
    ("Libraries", include_str!("../../../docs/06-libraries.md")),
    ("Editor, Canvas & UI", include_str!("../../../docs/07-editor-canvas-ui.md")),
    ("Export", include_str!("../../../docs/08-export.md")),
    ("Cookbook", include_str!("../../../docs/09-cookbook.md")),
    ("Architecture", include_str!("../../../docs/10-architecture.md")),
    ("FAQ", include_str!("../../../docs/11-faq.md")),
    ("Mermaid", include_str!("../../../docs/12-mermaid.md")),
];

pub struct HelpWindow {
    pub open: bool,
    selected: usize,
    search: String,
    cache: CommonMarkCache,
}

impl HelpWindow {
    pub fn new() -> Self {
        HelpWindow { open: false, selected: 0, search: String::new(), cache: CommonMarkCache::default() }
    }

    pub fn show(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }
        let mut open = self.open;
        egui::Window::new("gridflow documentation")
            .default_size([860.0, 620.0])
            .min_size([520.0, 320.0])
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Panel::left("help-topics")
                    .resizable(true)
                    .default_size(220.0)
                    .min_size(160.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.search)
                                .hint_text("Search docs…")
                                .desired_width(f32::INFINITY),
                        );
                        ui.separator();
                        let needle = self.search.trim().to_lowercase();
                        egui::ScrollArea::vertical().id_salt("help-list").show(ui, |ui| {
                            for (i, (title, body)) in TOPICS.iter().enumerate() {
                                let hit = needle.is_empty()
                                    || title.to_lowercase().contains(&needle)
                                    || body.to_lowercase().contains(&needle);
                                if !hit {
                                    continue;
                                }
                                let label = if needle.is_empty() {
                                    (*title).to_string()
                                } else {
                                    // Show how many matches each chapter has.
                                    let n = body.to_lowercase().matches(&needle).count();
                                    format!("{title}  ({n})")
                                };
                                if ui
                                    .selectable_label(self.selected == i, label)
                                    .clicked()
                                {
                                    self.selected = i;
                                }
                            }
                        });
                    });
                egui::CentralPanel::default().show(ui, |ui| {
                    egui::ScrollArea::vertical().id_salt("help-body").show(ui, |ui| {
                        let (_, body) = TOPICS[self.selected];
                        CommonMarkViewer::new().show(ui, &mut self.cache, body);
                    });
                });
            });
        self.open = open;
    }
}
