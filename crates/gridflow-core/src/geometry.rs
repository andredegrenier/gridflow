//! Node sizing and anchor-point math, shared by layout, canvas and export.
//! Text measurement is abstracted so core stays independent of egui; the app
//! implements `TextMeasurer` with real font metrics, tests use a stub.

use crate::ast::{Shape, Side};
use crate::model::{Node, Port};

pub const BASE_FONT_SIZE: f32 = 14.0;
pub const PAD_X: f32 = 14.0;
pub const PAD_Y: f32 = 10.0;
pub const MIN_W: f32 = 60.0;
pub const MIN_H: f32 = 34.0;

pub trait TextMeasurer {
    /// Width/height of a single line of text at the given font size.
    fn measure(&self, text: &str, size: f32) -> [f32; 2];
}

/// Fixed-metric measurer for tests and headless use.
pub struct MonoMeasurer;

impl TextMeasurer for MonoMeasurer {
    fn measure(&self, text: &str, size: f32) -> [f32; 2] {
        [text.chars().count() as f32 * size * 0.6, size * 1.3]
    }
}

pub fn node_size(node: &Node, measurer: &dyn TextMeasurer) -> [f32; 2] {
    let label = node.display_label();
    let mut w: f32 = 0.0;
    let mut h: f32 = 0.0;
    for line in label.lines() {
        let [lw, lh] = measurer.measure(line, BASE_FONT_SIZE);
        w = w.max(lw);
        h += lh;
    }
    let (mut w, mut h) = ((w + 2.0 * PAD_X).max(MIN_W), (h + 2.0 * PAD_Y).max(MIN_H));
    // Each shape grows enough that the label rectangle fits inside it.
    match node.shape {
        Shape::Rect | Shape::Rounded | Shape::Card | Shape::Note => {}
        Shape::Stadium => w += h, // one cap radius each side
        Shape::Circle => {
            let d = (w * w + h * h).sqrt() * 0.85 + PAD_X;
            w = d.max(h * 1.2);
            h = w;
        }
        Shape::DblCircle => {
            let d = (w * w + h * h).sqrt() * 0.85 + PAD_X + 2.0 * DBLCIRCLE_GAP;
            w = d.max(h * 1.2);
            h = w;
        }
        Shape::Ellipse => {
            w *= 1.35;
            h *= 1.35;
        }
        Shape::Diamond => {
            w *= 1.7;
            h *= 1.9;
        }
        Shape::Hexagon => w *= 1.45,
        Shape::Parallelogram => w += h * 0.9,
        Shape::Trapezoid => w *= 1.4,
        Shape::Cylinder => h += cylinder_cap(w) * 3.0,
        Shape::Subroutine => w += 2.0 * SUBROUTINE_INSET + PAD_X,
        Shape::Octagon => {
            w *= 1.25;
            h *= 1.35;
        }
        // Widest at the base: the label sits at center height, where the
        // triangle is only half its base width.
        Shape::Triangle => {
            w *= 2.1;
            h *= 1.9;
        }
        Shape::Tag => w += h * 0.5,
    }
    if let Some(fw) = node.style.width {
        w = fw;
    }
    if let Some(fh) = node.style.height {
        h = fh;
    }
    [w, h]
}

/// Vertical radius of a cylinder's elliptical caps.
pub fn cylinder_cap(width: f32) -> f32 {
    (width * 0.14).clamp(6.0, 22.0)
}

/// Distance of a subroutine's inner rails from its side edges.
pub const SUBROUTINE_INSET: f32 = 8.0;
/// Ring gap between a double circle's outer and inner circles.
pub const DBLCIRCLE_GAP: f32 = 5.0;

/// Fold size of a note's turned-down corner.
fn note_fold(size: [f32; 2]) -> f32 {
    (size[0] * 0.25).min(size[1] * 0.4).min(14.0)
}

/// Closed outline polygon for a shape, in world coordinates. Every consumer
/// (canvas fill/stroke/hit-test, SVG export, edge anchoring) uses this, so all
/// representations of a shape agree. Curved shapes are approximated with
/// enough segments to look smooth at high zoom.
pub fn shape_outline(shape: Shape, center: [f32; 2], size: [f32; 2]) -> Vec<[f32; 2]> {
    let [cx, cy] = center;
    let (hw, hh) = (size[0] / 2.0, size[1] / 2.0);
    let arc = |cx: f32, cy: f32, rx: f32, ry: f32, from: f32, to: f32, n: usize| {
        (0..=n).map(move |i| {
            let t = from + (to - from) * i as f32 / n as f32;
            [cx + rx * t.cos(), cy + ry * t.sin()]
        })
    };
    use std::f32::consts::PI;
    match shape {
        Shape::Rect | Shape::Rounded | Shape::Subroutine => {
            // Rounded corners matter visually but not for anchoring/hit tests.
            vec![
                [cx - hw, cy - hh],
                [cx + hw, cy - hh],
                [cx + hw, cy + hh],
                [cx - hw, cy + hh],
            ]
        }
        Shape::Circle | Shape::Ellipse | Shape::DblCircle => {
            arc(cx, cy, hw, hh, 0.0, 2.0 * PI, 48).collect()
        }
        Shape::Stadium => {
            let r = hh.min(hw);
            let mut pts: Vec<[f32; 2]> =
                arc(cx + hw - r, cy, r, hh, -PI / 2.0, PI / 2.0, 16).collect();
            pts.extend(arc(cx - hw + r, cy, r, hh, PI / 2.0, 3.0 * PI / 2.0, 16));
            pts
        }
        Shape::Diamond => vec![
            [cx, cy - hh],
            [cx + hw, cy],
            [cx, cy + hh],
            [cx - hw, cy],
        ],
        Shape::Hexagon => {
            let inset = (hw * 0.35).min(hh * 0.9);
            vec![
                [cx - hw + inset, cy - hh],
                [cx + hw - inset, cy - hh],
                [cx + hw, cy],
                [cx + hw - inset, cy + hh],
                [cx - hw + inset, cy + hh],
                [cx - hw, cy],
            ]
        }
        Shape::Parallelogram => {
            let s = (hw * 0.6).min(hh * 0.9);
            vec![
                [cx - hw + s, cy - hh],
                [cx + hw, cy - hh],
                [cx + hw - s, cy + hh],
                [cx - hw, cy + hh],
            ]
        }
        Shape::Trapezoid => {
            let s = (hw * 0.45).min(hh * 1.2);
            vec![
                [cx - hw + s, cy - hh],
                [cx + hw - s, cy - hh],
                [cx + hw, cy + hh],
                [cx - hw, cy + hh],
            ]
        }
        Shape::Cylinder => {
            let ry = cylinder_cap(size[0]).min(hh * 0.45);
            // Silhouette: straight sides, elliptical top bulging up and bottom
            // bulging down. The rim arc is drawn separately (see cylinder_rim).
            let mut pts: Vec<[f32; 2]> =
                arc(cx, cy - hh + ry, hw, ry, PI, 2.0 * PI, 20).collect();
            pts.extend(arc(cx, cy + hh - ry, hw, ry, 0.0, PI, 20));
            pts
        }
        Shape::Card => {
            let f = (hw * 0.5).min(hh * 0.5).min(16.0);
            vec![
                [cx - hw + f, cy - hh],
                [cx + hw, cy - hh],
                [cx + hw, cy + hh],
                [cx - hw, cy + hh],
                [cx - hw, cy - hh + f],
            ]
        }
        Shape::Octagon => {
            let c = (hw * 0.4).min(hh * 0.55);
            vec![
                [cx - hw + c, cy - hh],
                [cx + hw - c, cy - hh],
                [cx + hw, cy - hh + c],
                [cx + hw, cy + hh - c],
                [cx + hw - c, cy + hh],
                [cx - hw + c, cy + hh],
                [cx - hw, cy + hh - c],
                [cx - hw, cy - hh + c],
            ]
        }
        Shape::Triangle => vec![
            [cx, cy - hh],
            [cx + hw, cy + hh],
            [cx - hw, cy + hh],
        ],
        Shape::Note => {
            let f = note_fold(size);
            vec![
                [cx - hw, cy - hh],
                [cx + hw - f, cy - hh],
                [cx + hw, cy - hh + f],
                [cx + hw, cy + hh],
                [cx - hw, cy + hh],
            ]
        }
        Shape::Tag => {
            let s = (hw * 0.5).min(hh);
            vec![
                [cx - hw, cy - hh],
                [cx + hw - s, cy - hh],
                [cx + hw, cy],
                [cx + hw - s, cy + hh],
                [cx - hw, cy + hh],
            ]
        }
    }
}

/// Extra open polylines drawn on top of a shape's fill with the same stroke:
/// the cylinder rim, subroutine rails, double-circle inner ring, note fold.
/// One shared source keeps canvas and SVG pixel-identical.
pub fn shape_decorations(shape: Shape, center: [f32; 2], size: [f32; 2]) -> Vec<Vec<[f32; 2]>> {
    let [cx, cy] = center;
    let (hw, hh) = (size[0] / 2.0, size[1] / 2.0);
    match shape {
        Shape::Cylinder => vec![cylinder_rim(center, size)],
        Shape::Subroutine => {
            let x = hw - SUBROUTINE_INSET;
            vec![
                vec![[cx - x, cy - hh], [cx - x, cy + hh]],
                vec![[cx + x, cy - hh], [cx + x, cy + hh]],
            ]
        }
        Shape::DblCircle => {
            let (rx, ry) = ((hw - DBLCIRCLE_GAP).max(2.0), (hh - DBLCIRCLE_GAP).max(2.0));
            let ring = (0..=40)
                .map(|i| {
                    let t = 2.0 * std::f32::consts::PI * i as f32 / 40.0;
                    [cx + rx * t.cos(), cy + ry * t.sin()]
                })
                .collect();
            vec![ring]
        }
        Shape::Note => {
            let f = note_fold(size);
            vec![vec![
                [cx + hw - f, cy - hh],
                [cx + hw - f, cy - hh + f],
                [cx + hw, cy - hh + f],
            ]]
        }
        _ => Vec::new(),
    }
}

/// The visible inner rim arc of a cylinder (lower half of the top ellipse),
/// as an open polyline.
pub fn cylinder_rim(center: [f32; 2], size: [f32; 2]) -> Vec<[f32; 2]> {
    let (hw, hh) = (size[0] / 2.0, size[1] / 2.0);
    let ry = cylinder_cap(size[0]).min(hh * 0.45);
    let (cx, cy) = (center[0], center[1] - hh + ry);
    (0..=20)
        .map(|i| {
            let t = std::f32::consts::PI * i as f32 / 20.0;
            [cx + hw * t.cos(), cy + ry * t.sin()]
        })
        .collect()
}

/// Point on the node's boundary along the ray from its center toward `toward`.
pub fn boundary_anchor(shape: Shape, center: [f32; 2], size: [f32; 2], toward: [f32; 2]) -> [f32; 2] {
    let dx = toward[0] - center[0];
    let dy = toward[1] - center[1];
    if dx == 0.0 && dy == 0.0 {
        return center;
    }
    let (hw, hh) = (size[0] / 2.0, size[1] / 2.0);
    match shape {
        // Analytic fast paths.
        Shape::Rect | Shape::Rounded | Shape::Card | Shape::Subroutine | Shape::Note => {
            let tx = if dx != 0.0 { hw / dx.abs() } else { f32::INFINITY };
            let ty = if dy != 0.0 { hh / dy.abs() } else { f32::INFINITY };
            let t = tx.min(ty);
            [center[0] + dx * t, center[1] + dy * t]
        }
        Shape::Circle | Shape::Ellipse | Shape::DblCircle => {
            let k = (dx / hw).powi(2) + (dy / hh).powi(2);
            let t = 1.0 / k.sqrt();
            [center[0] + dx * t, center[1] + dy * t]
        }
        Shape::Diamond => {
            let k = dx.abs() / hw + dy.abs() / hh;
            let t = if k == 0.0 { 0.0 } else { 1.0 / k };
            [center[0] + dx * t, center[1] + dy * t]
        }
        _ => polygon_ray_anchor(&shape_outline(shape, center, size), center, toward),
    }
}

/// Intersection of the ray center→toward with a closed polygon outline.
fn polygon_ray_anchor(outline: &[[f32; 2]], center: [f32; 2], toward: [f32; 2]) -> [f32; 2] {
    let d = [toward[0] - center[0], toward[1] - center[1]];
    let mut best_t = f32::INFINITY;
    let n = outline.len();
    for i in 0..n {
        let a = outline[i];
        let b = outline[(i + 1) % n];
        let e = [b[0] - a[0], b[1] - a[1]];
        let denom = d[0] * e[1] - d[1] * e[0];
        if denom.abs() < 1e-9 {
            continue;
        }
        let ac = [a[0] - center[0], a[1] - center[1]];
        let t = (ac[0] * e[1] - ac[1] * e[0]) / denom; // along the ray
        let u = (ac[0] * d[1] - ac[1] * d[0]) / denom; // along the segment
        if t > 0.0 && (0.0..=1.0).contains(&u) && t < best_t {
            best_t = t;
        }
    }
    if best_t.is_finite() {
        [center[0] + d[0] * best_t, center[1] + d[1] * best_t]
    } else {
        center
    }
}

/// Is `p` inside the shape? Used for canvas hit-testing.
pub fn hit_test(shape: Shape, center: [f32; 2], size: [f32; 2], p: [f32; 2]) -> bool {
    let dx = (p[0] - center[0]).abs();
    let dy = (p[1] - center[1]).abs();
    let (hw, hh) = (size[0] / 2.0, size[1] / 2.0);
    match shape {
        Shape::Rect | Shape::Rounded | Shape::Card | Shape::Subroutine | Shape::Note => {
            dx <= hw && dy <= hh
        }
        Shape::Circle | Shape::Ellipse | Shape::DblCircle => {
            (dx / hw).powi(2) + (dy / hh).powi(2) <= 1.0
        }
        Shape::Diamond => dx / hw + dy / hh <= 1.0,
        _ => point_in_polygon(&shape_outline(shape, center, size), p),
    }
}

fn point_in_polygon(outline: &[[f32; 2]], p: [f32; 2]) -> bool {
    let mut inside = false;
    let n = outline.len();
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (outline[i], outline[j]);
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Anchor for a named port: ports on the same side are spaced evenly along it.
pub fn port_anchor(ports: &[Port], name: &str, center: [f32; 2], size: [f32; 2]) -> Option<[f32; 2]> {
    let port = ports.iter().find(|p| p.name == name)?;
    let same_side: Vec<&Port> = ports.iter().filter(|p| p.side == port.side).collect();
    let index = same_side.iter().position(|p| p.name == name)? as f32;
    let count = same_side.len() as f32;
    let frac = (index + 1.0) / (count + 1.0);
    let (hw, hh) = (size[0] / 2.0, size[1] / 2.0);
    Some(match port.side {
        Side::Top => [center[0] - hw + size[0] * frac, center[1] - hh],
        Side::Bottom => [center[0] - hw + size[0] * frac, center[1] + hh],
        Side::Left => [center[0] - hw, center[1] - hh + size[1] * frac],
        Side::Right => [center[0] + hw, center[1] - hh + size[1] * frac],
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

impl Rect {
    pub const NOTHING: Rect = Rect { min: [f32::INFINITY; 2], max: [f32::NEG_INFINITY; 2] };

    pub fn from_center_size(center: [f32; 2], size: [f32; 2]) -> Self {
        Rect {
            min: [center[0] - size[0] / 2.0, center[1] - size[1] / 2.0],
            max: [center[0] + size[0] / 2.0, center[1] + size[1] / 2.0],
        }
    }
    pub fn union(self, other: Rect) -> Rect {
        Rect {
            min: [self.min[0].min(other.min[0]), self.min[1].min(other.min[1])],
            max: [self.max[0].max(other.max[0]), self.max[1].max(other.max[1])],
        }
    }
    pub fn expand(self, by: f32) -> Rect {
        Rect {
            min: [self.min[0] - by, self.min[1] - by],
            max: [self.max[0] + by, self.max[1] + by],
        }
    }
    pub fn center(&self) -> [f32; 2] {
        [(self.min[0] + self.max[0]) / 2.0, (self.min[1] + self.max[1]) / 2.0]
    }
    pub fn size(&self) -> [f32; 2] {
        [self.max[0] - self.min[0], self.max[1] - self.min[1]]
    }
    pub fn is_finite(&self) -> bool {
        self.min[0].is_finite() && self.max[0].is_finite()
    }
}
