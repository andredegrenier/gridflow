//! SVG and PNG export. PNG rasterizes our own SVG via resvg so both formats
//! always agree.

use gridflow_core::export::{build_scene, svg::to_svg, SceneStyle};
use gridflow_core::geometry::TextMeasurer;
use gridflow_core::layout::LayoutResult;
use gridflow_core::model::DiagramModel;
use std::path::Path;

pub fn export_svg(
    model: &DiagramModel,
    layout: &LayoutResult,
    measurer: &dyn TextMeasurer,
    path: &Path,
    style: SceneStyle,
) -> Result<(), String> {
    let scene = build_scene(model, layout, style, measurer);
    std::fs::write(path, to_svg(&scene)).map_err(|e| e.to_string())
}

pub fn export_png(
    model: &DiagramModel,
    layout: &LayoutResult,
    measurer: &dyn TextMeasurer,
    path: &Path,
    scale: f32,
    style: SceneStyle,
) -> Result<(), String> {
    let scene = build_scene(model, layout, style, measurer);
    let svg = to_svg(&scene);
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default())
        .map_err(|e| e.to_string())?;
    let size = tree.size();
    let (w, h) = (
        (size.width() * scale).ceil() as u32,
        (size.height() * scale).ceil() as u32,
    );
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w.max(1), h.max(1))
        .ok_or_else(|| "pixmap allocation failed".to_string())?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.save_png(path).map_err(|e| e.to_string())
}

pub fn pick_export(extension: &str) -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .add_filter(extension, &[extension])
        .set_file_name(format!("diagram.{extension}"))
        .save_file()
}
