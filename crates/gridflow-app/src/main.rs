#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod camera;
mod canvas;
mod editor;
mod export;
mod files;
mod help;
mod library_ui;
mod notes;
mod theme;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title("gridflow"),
        ..Default::default()
    };
    eframe::run_native(
        "gridflow",
        options,
        Box::new(|cc| Ok(Box::new(app::GridflowApp::new(cc)))),
    )
}
