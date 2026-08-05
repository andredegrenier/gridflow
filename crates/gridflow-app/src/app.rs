//! Application shell: three-pane layout (editor | canvas | notes), menu bar,
//! global shortcuts, file lifecycle, and the text<->canvas sync loop.

use crate::camera::Camera;
use crate::canvas::{self, CanvasEvent, CanvasState};
use crate::editor::EditorPane;
use crate::files;
use crate::library_ui::LibraryUi;
use crate::notes::NotesPane;
use crate::theme::{Theme, ThemeChoice};
use eframe::egui::{self, Color32, FontId, Key, KeyboardShortcut, Modifiers};
use gridflow_core::document::{Document, Language};
use gridflow_core::geometry::TextMeasurer;
use gridflow_core::layout::{LayoutEngine, LayoutResult};
use gridflow_core::model::NodeId;
use gridflow_core::ops::{Intent, Op};
use gridflow_core::rewrite;
use std::path::PathBuf;

const EXAMPLE: &str = include_str!("../../../examples/deploy-pipeline.gfd");

struct EguiMeasurer<'a>(&'a egui::Context);

impl TextMeasurer for EguiMeasurer<'_> {
    fn measure(&self, text: &str, size: f32) -> [f32; 2] {
        let galley = self.0.fonts_mut(|f| {
            f.layout(
                text.to_string(),
                FontId::proportional(size),
                Color32::WHITE,
                f32::INFINITY,
            )
        });
        [galley.size().x, galley.size().y]
    }
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Persisted {
    recents: Vec<PathBuf>,
    last_file: Option<PathBuf>,
    show_editor: bool,
    show_notes: bool,
    show_library: bool,
    camera: Option<Camera>,
    #[serde(default)]
    theme: ThemeChoice,
}

pub struct GridflowApp {
    doc: Document,
    path: Option<PathBuf>,
    engine: LayoutEngine,
    layout: LayoutResult,
    camera: Camera,
    editor: EditorPane,
    notes: NotesPane,
    library_ui: LibraryUi,
    help: crate::help::HelpWindow,
    canvas_state: CanvasState,
    selection: Option<NodeId>,
    recents: Vec<PathBuf>,
    show_editor: bool,
    show_notes: bool,
    show_library: bool,
    confirm_close: bool,
    status: String,
    last_layout_version: u64,
    theme_choice: ThemeChoice,
}

impl GridflowApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let persisted: Persisted = cc
            .storage
            .and_then(|s| eframe::get_value(s, "gridflow"))
            .unwrap_or(Persisted {
                show_editor: true,
                show_notes: true,
                ..Default::default()
            });

        files::seed_library();
        let mut app = GridflowApp {
            doc: Document::new(EXAMPLE.to_string(), files::new_library(None)),
            path: None,
            engine: LayoutEngine::new(),
            layout: LayoutResult::default(),
            camera: persisted.camera.unwrap_or_default(),
            editor: EditorPane::new(),
            notes: NotesPane::new(),
            library_ui: LibraryUi::new(),
            help: crate::help::HelpWindow::new(),
            canvas_state: CanvasState::default(),
            selection: None,
            recents: persisted.recents,
            show_editor: persisted.show_editor,
            show_notes: persisted.show_notes,
            show_library: persisted.show_library,
            confirm_close: false,
            status: String::new(),
            last_layout_version: u64::MAX,
            theme_choice: persisted.theme,
        };
        if let Some(path) = persisted.last_file.clone() {
            app.load(path);
        }
        app
    }

    fn load(&mut self, path: PathBuf) {
        match files::open_document(&path) {
            Ok(doc) => {
                self.doc = doc;
                self.notes.load_for(&path);
                files::push_recent(&mut self.recents, &path);
                self.path = Some(path);
                self.selection = None;
                self.status = "Opened".into();
            }
            Err(e) => self.status = format!("Open failed: {e}"),
        }
    }

    fn save(&mut self) {
        let path = match &self.path {
            Some(p) => p.clone(),
            None => match files::pick_save() {
                Some(p) => p,
                None => return,
            },
        };
        match files::save_document(&mut self.doc, &path) {
            Ok(()) => {
                if self.notes.path.is_none() {
                    self.notes.path = Some(NotesPane::sidecar_path(&path));
                }
                self.notes.save();
                files::push_recent(&mut self.recents, &path);
                self.path = Some(path);
                self.status = "Saved".into();
            }
            Err(e) => self.status = format!("Save failed: {e}"),
        }
    }

    fn apply_op(&mut self, edits: Vec<gridflow_core::ops::TextEdit>, intent: Intent) {
        let seq = self.doc.next_seq();
        self.doc.apply(Op { edits, intent, actor: 0, seq });
    }

    fn fit_view(&mut self, viewport: egui::Rect, selection_only: bool) {
        let mut min = [f32::INFINITY; 2];
        let mut max = [f32::NEG_INFINITY; 2];
        let mut include = |pos: [f32; 2], size: [f32; 2]| {
            min[0] = min[0].min(pos[0] - size[0] / 2.0);
            min[1] = min[1].min(pos[1] - size[1] / 2.0);
            max[0] = max[0].max(pos[0] + size[0] / 2.0);
            max[1] = max[1].max(pos[1] + size[1] / 2.0);
        };
        match (&self.selection, selection_only) {
            (Some(id), true) => {
                if let (Some(&p), Some(&s)) =
                    (self.layout.positions.get(id), self.layout.sizes.get(id))
                {
                    include(p, [s[0] + 200.0, s[1] + 200.0]);
                }
            }
            _ => {
                for (id, &p) in &self.layout.positions {
                    if let Some(&s) = self.layout.sizes.get(id) {
                        include(p, s);
                    }
                }
            }
        }
        if min[0].is_finite() {
            self.camera.fit(min, max, viewport);
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        let cmd = Modifiers::COMMAND;
        let consume = |m: Modifiers, k: Key| {
            ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)))
        };
        // Undo/redo are intercepted here so TextEdit's built-in undoer never
        // sees them — the document's unified undo covers both panes.
        if consume(cmd.plus(Modifiers::SHIFT), Key::Z) {
            self.doc.redo();
        } else if consume(cmd, Key::Z) {
            self.doc.undo();
        }
        if consume(cmd, Key::S) {
            self.save();
        }
        if consume(cmd, Key::O) {
            if let Some(p) = files::pick_open() {
                self.load(p);
            }
        }
        if consume(cmd, Key::Num1) {
            self.show_editor = !self.show_editor;
        }
        if consume(cmd, Key::Num2) {
            self.show_notes = !self.show_notes;
        }
        if consume(cmd, Key::Num3) {
            self.show_library = !self.show_library;
        }
        if consume(cmd, Key::Slash) {
            self.help.open = !self.help.open;
        }
        if consume(cmd, Key::J) {
            // Jump camera to the node under the text cursor.
            if let Some(byte) = self.editor.cursor_byte {
                if let Some(id) = self.doc.node_at_byte(byte).cloned() {
                    self.selection = Some(id.clone());
                    if let Some(&p) = self.layout.positions.get(&id) {
                        self.camera.center = p;
                    }
                }
            }
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.selection = None;
        }
    }

    /// Replace the whole document text with generated GFD — one undoable op.
    fn convert_to_gfd(&mut self) {
        if self.doc.language() != Language::Mermaid {
            return;
        }
        let gfd = gridflow_core::mermaid::to_gfd(self.doc.model());
        let len = self.doc.text().len();
        self.apply_op(
            vec![gridflow_core::ops::TextEdit::new(0, len, gfd)],
            Intent::Paste,
        );
        self.status = "Converted to GFD (⌘Z reverts)".into();
    }

    fn menu_bar(&mut self, root: &mut egui::Ui, theme: &Theme) {
        let ctx = root.ctx().clone();
        let ctx = &ctx;
        egui::Panel::top("menu").show(root, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.doc = Document::new(String::new(), files::new_library(None));
                        self.path = None;
                        self.notes = NotesPane::new();
                        self.selection = None;
                        ui.close();
                    }
                    if ui.button("Open…").clicked() {
                        if let Some(p) = files::pick_open() {
                            self.load(p);
                        }
                        ui.close();
                    }
                    ui.menu_button("Open Recent", |ui| {
                        let recents = self.recents.clone();
                        for p in recents {
                            let name = p.file_name().map(|f| f.to_string_lossy().into_owned());
                            if ui.button(name.unwrap_or_default()).clicked() {
                                self.load(p);
                                ui.close();
                            }
                        }
                    });
                    ui.separator();
                    if ui.button("Save").clicked() {
                        self.save();
                        ui.close();
                    }
                    if ui.button("Save As…").clicked() {
                        if let Some(p) = files::pick_save() {
                            self.path = Some(p);
                            self.save();
                        }
                        ui.close();
                    }
                    ui.separator();
                    let convertible = self.doc.language() == Language::Mermaid;
                    if ui
                        .add_enabled(convertible, egui::Button::new("Convert Mermaid → GFD"))
                        .on_hover_text(
                            "Rewrites the document as equivalent GFD. Unlocks drag-to-pin, \
                             variables and classes; one undo step reverts.",
                        )
                        .clicked()
                    {
                        self.convert_to_gfd();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Export SVG…").clicked() {
                        if let Some(p) = crate::export::pick_export("svg") {
                            let m = EguiMeasurer(ctx);
                            match crate::export::export_svg(
                                self.doc.model(),
                                &self.layout,
                                &m,
                                &p,
                                theme.scene_style(),
                            ) {
                                Ok(()) => self.status = "Exported SVG".into(),
                                Err(e) => self.status = format!("Export failed: {e}"),
                            }
                        }
                        ui.close();
                    }
                    if ui.button("Export PNG (2x)…").clicked() {
                        if let Some(p) = crate::export::pick_export("png") {
                            let m = EguiMeasurer(ctx);
                            match crate::export::export_png(
                                self.doc.model(),
                                &self.layout,
                                &m,
                                &p,
                                2.0,
                                theme.scene_style(),
                            ) {
                                Ok(()) => self.status = "Exported PNG".into(),
                                Err(e) => self.status = format!("Export failed: {e}"),
                            }
                        }
                        ui.close();
                    }
                });
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_editor, "Editor pane   ⌘1");
                    ui.checkbox(&mut self.show_notes, "Notes pane   ⌘2");
                    ui.checkbox(&mut self.show_library, "Library pane   ⌘3");
                    ui.separator();
                    ui.menu_button(format!("Theme: {}", self.theme_choice.label()), |ui| {
                        for (choice, label) in ThemeChoice::ALL {
                            if ui
                                .radio_value(&mut self.theme_choice, *choice, *label)
                                .clicked()
                            {
                                ui.close();
                            }
                        }
                    });
                    ui.separator();
                    if ui.button("Zoom 100%").clicked() {
                        self.camera.zoom = 1.0;
                        ui.close();
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("Documentation   ⌘/").clicked() {
                        self.help.open = true;
                        ui.close();
                    }
                    ui.separator();
                    ui.label("F: fit selection · Shift+F: fit all");
                    ui.label("⌘J: jump to node under text cursor");
                    ui.label("Double-click node: jump to its source line");
                    ui.label("Drag node: pins it — writes @ (x, y) into the text");
                    ui.label("Scroll: pan · Pinch/⌘scroll: zoom");
                });
            });
        });
    }

    fn status_bar(&mut self, root: &mut egui::Ui) {
        egui::Panel::bottom("status").show(root, |ui| {
            ui.horizontal(|ui| {
                let name = self
                    .path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "untitled.gfd".into());
                ui.label(format!("{name}{}", if self.doc.is_dirty() { " •" } else { "" }));
                ui.separator();
                match self.doc.language() {
                    Language::Gfd => ui.weak("GFD"),
                    Language::Mermaid => ui
                        .weak("Mermaid")
                        .on_hover_text("Rendered natively. File → Convert Mermaid → GFD unlocks drag-to-pin."),
                };
                ui.separator();
                let errors = self
                    .doc
                    .model()
                    .diagnostics
                    .iter()
                    .filter(|d| d.severity == gridflow_core::ast::Severity::Error)
                    .count()
                    + self.layout.diagnostics.len();
                if errors > 0 {
                    ui.colored_label(Color32::from_rgb(0xd3, 0x2f, 0x2f), format!("{errors} problem(s)"));
                    if let Some(first) = self
                        .doc
                        .model()
                        .diagnostics
                        .iter()
                        .chain(self.layout.diagnostics.iter())
                        .next()
                    {
                        ui.weak(&first.message);
                    }
                } else {
                    ui.weak("no problems");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("{:.0}%", self.camera.zoom * 100.0));
                    ui.separator();
                    ui.weak(&self.status);
                });
            });
        });
    }
}

impl eframe::App for GridflowApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(
            storage,
            "gridflow",
            &Persisted {
                recents: self.recents.clone(),
                last_file: self.path.clone(),
                show_editor: self.show_editor,
                show_notes: self.show_notes,
                show_library: self.show_library,
                camera: Some(self.camera),
                theme: self.theme_choice,
            },
        );
    }

    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        let ctx = &ctx;
        // Confirm-close when dirty.
        if ctx.input(|i| i.viewport().close_requested()) && self.doc.is_dirty() && !self.confirm_close
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_close = true;
        }
        if self.confirm_close {
            egui::Window::new("Unsaved changes")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("The diagram has unsaved changes.");
                    ui.horizontal(|ui| {
                        if ui.button("Save and quit").clicked() {
                            self.save();
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("Quit without saving").clicked() {
                            self.doc.mark_saved();
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_close = false;
                        }
                    });
                });
        }

        let system_dark = ctx.theme() == egui::Theme::Dark;
        let theme = self.theme_choice.resolve(system_dark);
        ctx.set_visuals(theme.egui_visuals());

        self.shortcuts(ctx);
        self.menu_bar(root, &theme);
        self.status_bar(root);

        // Layout must be current before any pane draws.
        if self.doc.version() != self.last_layout_version {
            let measurer = EguiMeasurer(ctx);
            self.layout = self.engine.compute(self.doc.model(), &measurer);
            self.last_layout_version = self.doc.version();
        }

        if self.show_editor {
            egui::Panel::left("editor-pane")
                .resizable(true)
                .default_size(420.0)
                .min_size(240.0)
                .show(root, |ui| {
                    self.editor.sync(self.doc.text(), self.doc.version());
                    let mut diags = self.doc.model().diagnostics.clone();
                    diags.extend(self.layout.diagnostics.iter().cloned());
                    let language = self.doc.language();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        if let Some(edit) = self.editor.show(ui, &diags, &theme, language) {
                            self.apply_op(vec![edit], Intent::Typing);
                            self.editor.synced_version = self.doc.version();
                            self.editor.scratch = self.doc.text().to_string();
                        }
                    });
                });
        }

        if self.show_notes {
            egui::Panel::right("notes-pane")
                .resizable(true)
                .default_size(320.0)
                .min_size(200.0)
                .show(root, |ui| {
                    self.notes.show(ui);
                });
        }

        if self.show_library {
            egui::Panel::right("library-pane")
                .resizable(true)
                .default_size(260.0)
                .show(root, |ui| {
                    let lib = files::new_library(self.path.as_ref().and_then(|p| {
                        p.parent().map(|d| d.to_path_buf())
                    }));
                    if let Some(edit) = self.library_ui.show(ui, self.doc.text(), &lib) {
                        self.apply_op(vec![edit], Intent::Typing);
                    }
                });
        }

        egui::CentralPanel::default().show(root, |ui| {
            // Fit shortcuts want the viewport rect, so they live here.
            let viewport = ui.max_rect();
            if ui.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::F)) {
                self.selection = None;
                self.fit_view(viewport, false);
            } else if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F)) {
                self.fit_view(viewport, self.selection.is_some());
            }

            let events = canvas::show(
                ui,
                self.doc.model(),
                &self.layout,
                &mut self.camera,
                &mut self.canvas_state,
                self.selection.as_ref(),
                &theme,
            );
            for event in events {
                match event {
                    CanvasEvent::Select(id) => self.selection = id,
                    CanvasEvent::NodeDragged { id, world } => {
                        // Drag-to-pin writes `@ (x, y)` — GFD syntax. Mermaid
                        // has no placement clause, so the drag springs back
                        // and the status bar points at the converter.
                        if self.doc.language() == Language::Mermaid {
                            self.status =
                                "Mermaid has no pin syntax — File → Convert Mermaid → GFD".into();
                            self.selection = Some(id);
                            continue;
                        }
                        if let Some(node) = self.doc.model().nodes.get(&id).cloned() {
                            let edit =
                                rewrite::move_node_edit(self.doc.text(), &node, world[0], world[1]);
                            self.apply_op(vec![edit], Intent::MoveNode(id.clone()));
                            self.selection = Some(id);
                        }
                    }
                    CanvasEvent::GroupDragged { id, world } => {
                        if self.doc.language() == Language::Mermaid {
                            self.status =
                                "Mermaid has no pin syntax — File → Convert Mermaid → GFD".into();
                            continue;
                        }
                        if let Some(edit) =
                            rewrite::move_group_edit(self.doc.model(), &id, world[0], world[1])
                        {
                            self.apply_op(vec![edit], Intent::MoveGroup(id));
                        }
                    }
                    CanvasEvent::JumpToText(byte) => {
                        self.show_editor = true;
                        self.editor.pending_cursor = Some(byte);
                    }
                }
            }
        });

        self.help.show(ctx);
    }
}
