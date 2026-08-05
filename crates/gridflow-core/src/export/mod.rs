//! Render-target-agnostic scene: a flat list of styled primitives in world
//! coordinates, built from a model + layout. The SVG exporter serializes it;
//! keeping the builder here means exports always match what layout computed.

pub mod svg;

use crate::ast::{ArrowKind, Shape};
use crate::geometry::{boundary_anchor, port_anchor, Rect, TextMeasurer, BASE_FONT_SIZE};
use crate::layout::LayoutResult;
use crate::model::{DiagramModel, Rgba};

#[derive(Debug, Clone)]
pub enum SceneItem {
    Shape {
        shape: Shape,
        rect: Rect,
        fill: Rgba,
        stroke: Rgba,
        stroke_width: f32,
        dashed: bool,
    },
    Line {
        points: Vec<[f32; 2]>,
        stroke: Rgba,
        width: f32,
        dashed: bool,
    },
    /// Filled triangle (arrowheads).
    Triangle {
        points: [[f32; 2]; 3],
        fill: Rgba,
    },
    Text {
        center: [f32; 2],
        text: String,
        size: f32,
        color: Rgba,
        bold: bool,
        /// Background plate behind edge labels for readability.
        plate: Option<Rgba>,
    },
    GroupFrame {
        rect: Rect,
        title: Option<String>,
        stroke: Rgba,
        title_color: Rgba,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct SceneStyle {
    pub node_fill: Rgba,
    pub node_stroke: Rgba,
    pub text: Rgba,
    pub edge: Rgba,
    pub group: Rgba,
    pub background: Rgba,
}

impl Default for SceneStyle {
    fn default() -> Self {
        SceneStyle {
            node_fill: Rgba::rgb(0xf4, 0xf6, 0xfa),
            node_stroke: Rgba::rgb(0x3a, 0x45, 0x55),
            text: Rgba::rgb(0x1a, 0x20, 0x28),
            edge: Rgba::rgb(0x55, 0x60, 0x70),
            group: Rgba::rgb(0x8a, 0x94, 0xa6),
            background: Rgba::rgb(0xff, 0xff, 0xff),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Scene {
    pub items: Vec<SceneItem>,
    pub bounds: Rect,
    pub style: SceneStyle,
}

pub const ARROW_LEN: f32 = 10.0;
pub const ARROW_HALF_W: f32 = 4.5;

/// Anchor point on `id`'s boundary for an edge heading toward `toward`,
/// honoring a named port when given.
pub fn edge_anchor(
    model: &DiagramModel,
    layout: &LayoutResult,
    id: &str,
    port: Option<&str>,
    toward: [f32; 2],
) -> Option<[f32; 2]> {
    let node = model.nodes.get(id)?;
    let center = *layout.positions.get(id)?;
    let size = *layout.sizes.get(id)?;
    if let Some(p) = port {
        if let Some(a) = port_anchor(&node.ports, p, center, size) {
            return Some(a);
        }
    }
    Some(boundary_anchor(node.shape, center, size, toward))
}

pub fn arrow_head(tip: [f32; 2], from: [f32; 2], fill: Rgba) -> SceneItem {
    let dx = tip[0] - from[0];
    let dy = tip[1] - from[1];
    let len = (dx * dx + dy * dy).sqrt().max(1e-3);
    let (ux, uy) = (dx / len, dy / len);
    let base = [tip[0] - ux * ARROW_LEN, tip[1] - uy * ARROW_LEN];
    let (px, py) = (-uy, ux);
    SceneItem::Triangle {
        points: [
            tip,
            [base[0] + px * ARROW_HALF_W, base[1] + py * ARROW_HALF_W],
            [base[0] - px * ARROW_HALF_W, base[1] - py * ARROW_HALF_W],
        ],
        fill,
    }
}

pub fn build_scene(
    model: &DiagramModel,
    layout: &LayoutResult,
    style: SceneStyle,
    measurer: &dyn TextMeasurer,
) -> Scene {
    let mut items = Vec::new();
    let mut bounds = Rect::NOTHING;

    for (gid, rect) in &layout.group_rects {
        let group = &model.groups[gid];
        items.push(SceneItem::GroupFrame {
            rect: *rect,
            title: Some(group.label.clone().unwrap_or_else(|| gid.to_string())),
            stroke: style.group,
            title_color: style.group,
        });
        bounds = bounds.union(*rect);
    }

    for e in &model.edges {
        let (Some(&fc), Some(&tc)) = (
            layout.positions.get(&e.from),
            layout.positions.get(&e.to),
        ) else {
            continue;
        };
        let Some(a) = edge_anchor(model, layout, &e.from, e.from_port.as_deref(), tc) else {
            continue;
        };
        let Some(b) = edge_anchor(model, layout, &e.to, e.to_port.as_deref(), fc) else {
            continue;
        };
        let stroke = e.stroke.unwrap_or(style.edge);
        let width = if e.bold { 3.0 } else { 1.6 };
        items.push(SceneItem::Line {
            points: vec![a, b],
            stroke,
            width,
            dashed: e.dashed,
        });
        if matches!(e.arrow, ArrowKind::Directed | ArrowKind::Dotted | ArrowKind::Bidirectional) {
            items.push(arrow_head(b, a, stroke));
        }
        if e.arrow == ArrowKind::Bidirectional {
            items.push(arrow_head(a, b, stroke));
        }
        if let Some(label) = &e.label {
            items.push(SceneItem::Text {
                center: [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0],
                text: label.clone(),
                size: BASE_FONT_SIZE - 2.0,
                color: style.text,
                bold: false,
                plate: Some(style.background),
            });
        }
    }

    for node in model.nodes.values() {
        let (Some(&pos), Some(&size)) = (
            layout.positions.get(&node.id),
            layout.sizes.get(&node.id),
        ) else {
            continue;
        };
        let rect = Rect::from_center_size(pos, size);
        bounds = bounds.union(rect);
        let stroke = node.style.stroke.unwrap_or(style.node_stroke);
        let stroke_width = if node.style.bold { 2.5 } else { 1.4 };
        items.push(SceneItem::Shape {
            shape: node.shape,
            rect,
            fill: node.style.fill.unwrap_or(style.node_fill),
            stroke,
            stroke_width,
            dashed: node.style.dashed || node.phantom,
        });
        for deco in crate::geometry::shape_decorations(node.shape, pos, size) {
            items.push(SceneItem::Line {
                points: deco,
                stroke,
                width: stroke_width,
                dashed: false,
            });
        }
        let label = node.display_label();
        // Center multi-line labels as a block.
        let line_h = measurer.measure("Ay", BASE_FONT_SIZE)[1];
        let lines: Vec<&str> = label.lines().collect();
        let total = line_h * lines.len() as f32;
        for (i, line) in lines.iter().enumerate() {
            items.push(SceneItem::Text {
                center: [
                    pos[0],
                    pos[1] - total / 2.0 + line_h * (i as f32 + 0.5),
                ],
                text: line.to_string(),
                size: BASE_FONT_SIZE,
                color: node.style.text.unwrap_or(style.text),
                bold: node.style.bold,
                plate: None,
            });
        }
    }

    if !bounds.is_finite() {
        bounds = Rect { min: [0.0, 0.0], max: [100.0, 100.0] };
    }
    Scene { items, bounds: bounds.expand(24.0), style }
}
