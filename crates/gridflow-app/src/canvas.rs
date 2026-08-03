//! The diagram canvas: pan/zoom camera, node/edge/group painting with text
//! that stays crisp at every zoom (font size scales with zoom — glyphs are
//! rasterized natively, never bitmap-scaled), hit testing, and drag.

use crate::camera::Camera;
use crate::theme::{to_color32, Theme};
use eframe::egui::{
    Align2, Color32, CursorIcon, FontId, Pos2, Rect, Sense, Shape as EShape, Stroke, Ui, Vec2,
};
use eframe::epaint::{PathShape, PathStroke, StrokeKind};
use gridflow_core::ast::{ArrowKind, Shape};
use gridflow_core::export::{edge_anchor, ARROW_HALF_W, ARROW_LEN};
use gridflow_core::geometry::BASE_FONT_SIZE;
use gridflow_core::layout::{LayoutResult, GROUP_TITLE_H};
use gridflow_core::model::{DiagramModel, NodeId};
use std::collections::HashMap;

/// Below this on-screen font size we draw a placeholder bar instead of text —
/// unreadable anyway, and it keeps far-out zoom fast.
const TEXT_CUTOFF_PX: f32 = 5.0;

pub enum CanvasEvent {
    Select(Option<NodeId>),
    NodeDragged { id: NodeId, world: [f32; 2] },
    GroupDragged { id: NodeId, world: [f32; 2] },
    JumpToText(usize),
}

#[derive(Default)]
pub struct CanvasState {
    pub drag: Option<DragState>,
    /// Live preview positions during a drag (world coords).
    pub preview: HashMap<NodeId, [f32; 2]>,
}

pub struct DragState {
    pub id: NodeId,
    pub is_group: bool,
    pub start_world: [f32; 2],
    pub accum: Vec2,
}

/// Quantize the on-screen font size so continuous zoom doesn't thrash the
/// glyph atlas with hundreds of unique sizes.
fn zoomed_font(base: f32, zoom: f32) -> FontId {
    let px = (base * zoom * 4.0).round() / 4.0;
    FontId::proportional(px.max(0.1))
}

pub fn show(
    ui: &mut Ui,
    model: &DiagramModel,
    layout: &LayoutResult,
    camera: &mut Camera,
    state: &mut CanvasState,
    selection: Option<&NodeId>,
    theme: &Theme,
) -> Vec<CanvasEvent> {
    let mut events = Vec::new();
    let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme.canvas_bg);

    // ---- input: zoom & pan ------------------------------------------------
    if let Some(hover) = response.hover_pos() {
        let zoom_delta = ui.input(|i| i.zoom_delta());
        if zoom_delta != 1.0 {
            camera.zoom_at(zoom_delta, hover, rect);
        }
        let scroll = ui.input(|i| i.smooth_scroll_delta);
        if scroll != Vec2::ZERO && zoom_delta == 1.0 {
            camera.pan_screen(scroll);
        }
    }

    let world_at = |pos: Pos2| camera.screen_to_world(pos, rect);

    // ---- hit testing ------------------------------------------------------
    let hovered_node = response.hover_pos().and_then(|p| {
        let w = world_at(p);
        // Reverse order so later-declared (painted on top) wins.
        model.nodes.values().rev().find_map(|n| {
            let pos = pv(&state.preview, layout, &n.id)?;
            let size = *layout.sizes.get(&n.id)?;
            gridflow_core::geometry::hit_test(n.shape, pos, size, w).then(|| n.id.clone())
        })
    });
    let hovered_group_title = response.hover_pos().and_then(|p| {
        let w = world_at(p);
        layout.group_rects.iter().find_map(|(gid, r)| {
            (w[0] >= r.min[0]
                && w[0] <= r.max[0]
                && w[1] >= r.min[1]
                && w[1] <= r.min[1] + GROUP_TITLE_H)
                .then(|| gid.clone())
        })
    });

    if hovered_node.is_some() || hovered_group_title.is_some() {
        ui.output_mut(|o| o.cursor_icon = CursorIcon::Grab);
    }

    // ---- drag -------------------------------------------------------------
    if response.drag_started() {
        if let Some(id) = hovered_node.clone() {
            let start = pv(&state.preview, layout, &id).unwrap_or_default();
            state.drag = Some(DragState { id, is_group: false, start_world: start, accum: Vec2::ZERO });
        } else if let Some(gid) = hovered_group_title.clone() {
            let start = layout.group_rects[&gid].center();
            state.drag =
                Some(DragState { id: gid, is_group: true, start_world: start, accum: Vec2::ZERO });
        }
    }
    if response.dragged() {
        if let Some(drag) = &mut state.drag {
            drag.accum += response.drag_delta();
            let world = [
                drag.start_world[0] + drag.accum.x / camera.zoom,
                drag.start_world[1] + drag.accum.y / camera.zoom,
            ];
            if drag.is_group {
                // Preview every member.
                if let Some(group) = model.groups.get(&drag.id) {
                    let delta = [world[0] - drag.start_world[0], world[1] - drag.start_world[1]];
                    for m in &group.members {
                        if let Some(p) = layout.positions.get(m) {
                            state.preview.insert(m.clone(), [p[0] + delta[0], p[1] + delta[1]]);
                        }
                    }
                }
            } else {
                state.preview.insert(drag.id.clone(), world);
            }
        } else {
            // Background drag pans.
            camera.pan_screen(response.drag_delta());
        }
    }
    if response.drag_stopped() {
        if let Some(drag) = state.drag.take() {
            let world = [
                drag.start_world[0] + drag.accum.x / camera.zoom,
                drag.start_world[1] + drag.accum.y / camera.zoom,
            ];
            if drag.accum.length() > 1.0 {
                events.push(if drag.is_group {
                    CanvasEvent::GroupDragged { id: drag.id, world }
                } else {
                    CanvasEvent::NodeDragged { id: drag.id, world }
                });
            }
            state.preview.clear();
        }
    }

    // ---- click / double-click --------------------------------------------
    if response.double_clicked() {
        if let Some(id) = &hovered_node {
            if let Some(node) = model.nodes.get(id) {
                events.push(CanvasEvent::JumpToText(node.stmt_span.start));
            }
        }
    } else if response.clicked() {
        events.push(CanvasEvent::Select(hovered_node.clone()));
    }

    // ---- paint ------------------------------------------------------------
    let to_screen = |w: [f32; 2]| camera.world_to_screen(w, rect);
    draw_grid(&painter, rect, camera, theme);

    for (gid, grect) in &layout.group_rects {
        // Recompute the frame from preview positions during a group drag.
        let mut r = *grect;
        if state.drag.as_ref().is_some_and(|d| d.is_group && &d.id == gid) {
            if let Some(group) = model.groups.get(gid) {
                if let Some(m0) = group.members.first().and_then(|m| state.preview.get(m)) {
                    let orig = layout.positions[&group.members[0]];
                    let dx = m0[0] - orig[0];
                    let dy = m0[1] - orig[1];
                    r.min[0] += dx;
                    r.max[0] += dx;
                    r.min[1] += dy;
                    r.max[1] += dy;
                }
            }
        }
        let sr = Rect::from_two_pos(to_screen(r.min), to_screen(r.max));
        painter.rect_stroke(
            sr,
            8.0 * camera.zoom,
            Stroke::new((1.2 * camera.zoom).max(0.5), theme.group),
            StrokeKind::Middle,
        );
        let title = model
            .groups
            .get(gid)
            .and_then(|g| g.label.clone())
            .unwrap_or_else(|| gid.to_string());
        let font = zoomed_font(12.0, camera.zoom);
        if font.size >= TEXT_CUTOFF_PX {
            painter.text(
                sr.min + Vec2::new(10.0 * camera.zoom, 4.0 * camera.zoom),
                Align2::LEFT_TOP,
                title,
                font,
                theme.group,
            );
        }
    }

    for e in &model.edges {
        let (Some(fc), Some(tc)) = (
            pv(&state.preview, layout, &e.from),
            pv(&state.preview, layout, &e.to),
        ) else {
            continue;
        };
        // Anchor on shape boundaries / ports, honoring drag previews.
        let a = anchored(model, layout, &e.from, e.from_port.as_deref(), fc, tc);
        let b = anchored(model, layout, &e.to, e.to_port.as_deref(), tc, fc);
        let color = e.stroke.map(to_color32).unwrap_or(theme.edge);
        let width = (if e.bold { 3.0 } else { 1.6 }) * camera.zoom;
        let (sa, sb) = (to_screen(a), to_screen(b));
        if e.dashed {
            draw_dashed(&painter, sa, sb, Stroke::new(width.max(0.5), color), camera.zoom);
        } else {
            painter.line_segment([sa, sb], Stroke::new(width.max(0.5), color));
        }
        if matches!(e.arrow, ArrowKind::Directed | ArrowKind::Dotted | ArrowKind::Bidirectional) {
            draw_arrowhead(&painter, sa, sb, color, camera.zoom);
        }
        if e.arrow == ArrowKind::Bidirectional {
            draw_arrowhead(&painter, sb, sa, color, camera.zoom);
        }
        if let Some(label) = &e.label {
            let font = zoomed_font(BASE_FONT_SIZE - 2.0, camera.zoom);
            if font.size >= TEXT_CUTOFF_PX {
                let mid = Pos2::new((sa.x + sb.x) / 2.0, (sa.y + sb.y) / 2.0);
                let galley = painter.layout_no_wrap(label.clone(), font, theme.text);
                let bg = Rect::from_center_size(mid, galley.size() + Vec2::splat(4.0));
                painter.rect_filled(bg, 3.0, theme.canvas_bg.gamma_multiply(0.9));
                painter.galley(bg.min + Vec2::splat(2.0), galley, theme.text);
            }
        }
    }

    for node in model.nodes.values() {
        let Some(pos) = pv(&state.preview, layout, &node.id) else { continue };
        let Some(&size) = layout.sizes.get(&node.id) else { continue };
        let center = to_screen(pos);
        let ssize = Vec2::new(size[0], size[1]) * camera.zoom;
        let nrect = Rect::from_center_size(center, ssize);
        if !rect.intersects(nrect.expand(40.0)) {
            continue; // cull off-screen nodes
        }

        let fill = if node.phantom {
            theme.canvas_bg
        } else {
            node.style.fill.map(to_color32).unwrap_or(theme.node_fill)
        };
        let stroke_color = if node.phantom {
            theme.phantom
        } else {
            node.style.stroke.map(to_color32).unwrap_or(theme.node_stroke)
        };
        let sw = (if node.style.bold { 2.5 } else { 1.4 } * camera.zoom).max(0.5);
        let dashed = node.style.dashed || node.phantom;
        draw_shape(&painter, node.shape, nrect, fill, Stroke::new(sw, stroke_color), dashed, camera.zoom);

        if selection == Some(&node.id) {
            let sel = Stroke::new((2.0 * camera.zoom).max(1.0), theme.selection);
            draw_shape_outline(&painter, node.shape, nrect.expand(3.0 * camera.zoom), sel, camera.zoom);
        }

        // Ports: small dots when selected or hovered.
        if selection == Some(&node.id) || hovered_node.as_ref() == Some(&node.id) {
            for p in &node.ports {
                if let Some(a) =
                    gridflow_core::geometry::port_anchor(&node.ports, &p.name, pos, size)
                {
                    painter.circle_filled(to_screen(a), (3.0 * camera.zoom).max(1.5), theme.selection);
                }
            }
        }

        let label = node.label.clone().unwrap_or_else(|| node.id.to_string());
        let font = zoomed_font(BASE_FONT_SIZE, camera.zoom);
        if font.size >= TEXT_CUTOFF_PX {
            let color = if node.phantom {
                theme.phantom
            } else {
                node.style.text.map(to_color32).unwrap_or(theme.text)
            };
            painter.text(center, Align2::CENTER_CENTER, label, font, color);
        } else {
            // Placeholder bar: cheap and still signals "there's text here".
            let bar = Rect::from_center_size(center, Vec2::new(ssize.x * 0.6, ssize.y * 0.12));
            painter.rect_filled(bar, 1.0, theme.text.gamma_multiply(0.35));
        }
    }

    events
}

/// Preview position during a drag, else the laid-out position.
fn pv(
    preview: &HashMap<NodeId, [f32; 2]>,
    layout: &LayoutResult,
    id: &NodeId,
) -> Option<[f32; 2]> {
    preview
        .get(id)
        .copied()
        .or_else(|| layout.positions.get(id).copied())
}

fn anchored(
    model: &DiagramModel,
    layout: &LayoutResult,
    id: &NodeId,
    port: Option<&str>,
    center: [f32; 2],
    toward: [f32; 2],
) -> [f32; 2] {
    // edge_anchor uses layout positions; during drags we shift its result by
    // the preview delta baked into `center`.
    let base = layout.positions.get(id).copied().unwrap_or(center);
    let a = edge_anchor(model, layout, id, port, [toward[0] - center[0] + base[0], toward[1] - center[1] + base[1]])
        .unwrap_or(base);
    [a[0] - base[0] + center[0], a[1] - base[1] + center[1]]
}

fn draw_grid(painter: &eframe::egui::Painter, rect: Rect, camera: &Camera, theme: &Theme) {
    let step = 40.0 * camera.zoom;
    if step < 8.0 {
        return;
    }
    let stroke = Stroke::new(1.0, theme.grid);
    let origin = camera.world_to_screen([0.0, 0.0], rect);
    let mut x = origin.x % step;
    while x < rect.width() {
        let sx = rect.min.x + x;
        painter.line_segment([Pos2::new(sx, rect.min.y), Pos2::new(sx, rect.max.y)], stroke);
        x += step;
    }
    let mut y = origin.y % step;
    while y < rect.height() {
        let sy = rect.min.y + y;
        painter.line_segment([Pos2::new(rect.min.x, sy), Pos2::new(rect.max.x, sy)], stroke);
        y += step;
    }
}

/// Screen-space outline points via the shared core geometry, so canvas,
/// export, anchoring and hit-testing always agree on a shape's silhouette.
fn screen_outline(shape: Shape, rect: Rect) -> Vec<Pos2> {
    let c = rect.center();
    gridflow_core::geometry::shape_outline(
        shape,
        [c.x, c.y],
        [rect.width(), rect.height()],
    )
    .into_iter()
    .map(|p| Pos2::new(p[0], p[1]))
    .collect()
}

fn draw_shape(
    painter: &eframe::egui::Painter,
    shape: Shape,
    rect: Rect,
    fill: Color32,
    stroke: Stroke,
    dashed: bool,
    zoom: f32,
) {
    match shape {
        Shape::Rect | Shape::Rounded => {
            let r = if shape == Shape::Rounded { 10.0 * zoom } else { 0.0 };
            painter.rect_filled(rect, r, fill);
            if dashed {
                draw_dashed_rect(painter, rect, stroke, zoom);
            } else {
                painter.rect_stroke(rect, r, stroke, StrokeKind::Middle);
            }
        }
        _ => {
            // All remaining shapes have convex outlines by construction.
            let pts = screen_outline(shape, rect);
            painter.add(EShape::convex_polygon(pts.clone(), fill, Stroke::NONE));
            draw_poly_outline(painter, &pts, stroke, dashed, zoom);
            if shape == Shape::Cylinder {
                let c = rect.center();
                let rim: Vec<Pos2> = gridflow_core::geometry::cylinder_rim(
                    [c.x, c.y],
                    [rect.width(), rect.height()],
                )
                .into_iter()
                .map(|p| Pos2::new(p[0], p[1]))
                .collect();
                painter.add(EShape::line(rim, stroke));
            }
        }
    }
}

fn draw_shape_outline(
    painter: &eframe::egui::Painter,
    shape: Shape,
    rect: Rect,
    stroke: Stroke,
    zoom: f32,
) {
    match shape {
        Shape::Rect | Shape::Rounded => {
            let r = if shape == Shape::Rounded { 12.0 * zoom } else { 0.0 };
            painter.rect_stroke(rect, r, stroke, StrokeKind::Middle);
        }
        _ => {
            let pts = screen_outline(shape, rect);
            draw_poly_outline(painter, &pts, stroke, false, zoom);
        }
    }
}

fn draw_poly_outline(
    painter: &eframe::egui::Painter,
    pts: &[Pos2],
    stroke: Stroke,
    dashed: bool,
    zoom: f32,
) {
    if dashed {
        let mut prev = pts[pts.len() - 1];
        for &p in pts {
            draw_dashed(painter, prev, p, stroke, zoom);
            prev = p;
        }
    } else {
        painter.add(EShape::Path(PathShape::closed_line(
            pts.to_vec(),
            PathStroke::new(stroke.width, stroke.color),
        )));
    }
}

fn draw_dashed_rect(painter: &eframe::egui::Painter, rect: Rect, stroke: Stroke, zoom: f32) {
    let c = [
        rect.min,
        Pos2::new(rect.max.x, rect.min.y),
        rect.max,
        Pos2::new(rect.min.x, rect.max.y),
    ];
    for i in 0..4 {
        draw_dashed(painter, c[i], c[(i + 1) % 4], stroke, zoom);
    }
}

fn draw_dashed(painter: &eframe::egui::Painter, a: Pos2, b: Pos2, stroke: Stroke, zoom: f32) {
    let dash = 6.0 * zoom;
    let gap = 5.0 * zoom;
    painter.extend(EShape::dashed_line(&[a, b], stroke, dash, gap));
}

fn draw_arrowhead(painter: &eframe::egui::Painter, from: Pos2, tip: Pos2, color: Color32, zoom: f32) {
    let d = tip - from;
    let len = d.length().max(1e-3);
    let u = d / len;
    let p = Vec2::new(-u.y, u.x);
    let base = tip - u * ARROW_LEN * zoom;
    painter.add(EShape::convex_polygon(
        vec![
            tip,
            base + p * ARROW_HALF_W * zoom,
            base - p * ARROW_HALF_W * zoom,
        ],
        color,
        Stroke::NONE,
    ));
}
