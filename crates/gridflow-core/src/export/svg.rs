//! Scene -> SVG string. No dependencies; the app rasterizes this same SVG for
//! PNG export so both formats always agree.

use super::{Scene, SceneItem};
use crate::ast::Shape;
use crate::model::Rgba;
use std::fmt::Write;

fn color(Rgba([r, g, b, a]): Rgba) -> String {
    if a == 255 {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("rgba({r},{g},{b},{:.3})", a as f32 / 255.0)
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

pub fn to_svg(scene: &Scene) -> String {
    let b = scene.bounds;
    let (w, h) = (b.max[0] - b.min[0], b.max[1] - b.min[1]);
    let mut out = String::new();
    let _ = writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{} {} {} {}" width="{w}" height="{h}" font-family="Helvetica, Arial, sans-serif">"#,
        b.min[0], b.min[1], w, h
    );
    let _ = writeln!(
        out,
        r#"<rect x="{}" y="{}" width="{w}" height="{h}" fill="{}"/>"#,
        b.min[0],
        b.min[1],
        color(scene.style.background)
    );

    for item in &scene.items {
        match item {
            SceneItem::GroupFrame { rect, title, stroke, title_color } => {
                let _ = writeln!(
                    out,
                    r#"<rect x="{}" y="{}" width="{}" height="{}" rx="8" fill="none" stroke="{}" stroke-width="1.2" stroke-dasharray="6 4"/>"#,
                    rect.min[0],
                    rect.min[1],
                    rect.max[0] - rect.min[0],
                    rect.max[1] - rect.min[1],
                    color(*stroke)
                );
                if let Some(t) = title {
                    let _ = writeln!(
                        out,
                        r#"<text x="{}" y="{}" font-size="12" font-weight="bold" fill="{}">{}</text>"#,
                        rect.min[0] + 10.0,
                        rect.min[1] + 16.0,
                        color(*title_color),
                        esc(t)
                    );
                }
            }
            SceneItem::Shape { shape, rect, fill, stroke, stroke_width, dashed } => {
                let dash = if *dashed { r#" stroke-dasharray="5 4""# } else { "" };
                let (x, y) = (rect.min[0], rect.min[1]);
                let (rw, rh) = (rect.max[0] - x, rect.max[1] - y);
                let (cx, cy) = ((rect.min[0] + rect.max[0]) / 2.0, (rect.min[1] + rect.max[1]) / 2.0);
                match shape {
                    Shape::Rect | Shape::Rounded => {
                        let rx = if *shape == Shape::Rounded { 10.0 } else { 0.0 };
                        let _ = writeln!(
                            out,
                            r#"<rect x="{x}" y="{y}" width="{rw}" height="{rh}" rx="{rx}" fill="{}" stroke="{}" stroke-width="{stroke_width}"{dash}/>"#,
                            color(*fill),
                            color(*stroke)
                        );
                    }
                    Shape::Circle | Shape::Ellipse => {
                        let _ = writeln!(
                            out,
                            r#"<ellipse cx="{cx}" cy="{cy}" rx="{}" ry="{}" fill="{}" stroke="{}" stroke-width="{stroke_width}"{dash}/>"#,
                            rw / 2.0,
                            rh / 2.0,
                            color(*fill),
                            color(*stroke)
                        );
                    }
                    // Everything else shares the canvas's outline geometry.
                    other => {
                        let outline =
                            crate::geometry::shape_outline(*other, [cx, cy], [rw, rh]);
                        let pts: Vec<String> = outline
                            .iter()
                            .map(|p| format!("{},{}", p[0], p[1]))
                            .collect();
                        let _ = writeln!(
                            out,
                            r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{stroke_width}"{dash} stroke-linejoin="round"/>"#,
                            pts.join(" "),
                            color(*fill),
                            color(*stroke)
                        );
                        if *other == Shape::Cylinder {
                            let rim = crate::geometry::cylinder_rim([cx, cy], [rw, rh]);
                            let pts: Vec<String> =
                                rim.iter().map(|p| format!("{},{}", p[0], p[1])).collect();
                            let _ = writeln!(
                                out,
                                r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="{stroke_width}"/>"#,
                                pts.join(" "),
                                color(*stroke)
                            );
                        }
                    }
                }
            }
            SceneItem::Line { points, stroke, width, dashed } => {
                let dash = if *dashed { r#" stroke-dasharray="6 5""# } else { "" };
                let pts: Vec<String> = points.iter().map(|p| format!("{},{}", p[0], p[1])).collect();
                let _ = writeln!(
                    out,
                    r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="{width}"{dash}/>"#,
                    pts.join(" "),
                    color(*stroke)
                );
            }
            SceneItem::Triangle { points, fill } => {
                let _ = writeln!(
                    out,
                    r#"<polygon points="{},{} {},{} {},{}" fill="{}"/>"#,
                    points[0][0], points[0][1], points[1][0], points[1][1], points[2][0], points[2][1],
                    color(*fill)
                );
            }
            SceneItem::Text { center, text, size, color: c, bold, plate } => {
                if let Some(bg) = plate {
                    let pw = text.chars().count() as f32 * size * 0.62 + 8.0;
                    let _ = writeln!(
                        out,
                        r#"<rect x="{}" y="{}" width="{pw}" height="{}" fill="{}"/>"#,
                        center[0] - pw / 2.0,
                        center[1] - size * 0.75,
                        size * 1.5,
                        color(*bg)
                    );
                }
                let weight = if *bold { r#" font-weight="bold""# } else { "" };
                let _ = writeln!(
                    out,
                    r#"<text x="{}" y="{}" font-size="{size}" text-anchor="middle" dominant-baseline="central" fill="{}"{weight}>{}</text>"#,
                    center[0],
                    center[1],
                    color(*c),
                    esc(text)
                );
            }
        }
    }
    out.push_str("</svg>\n");
    out
}
