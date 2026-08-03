//! Colors for canvas and editor, resolved against egui's light/dark visuals.

use eframe::egui::Color32;
use gridflow_core::model::Rgba;

pub fn to_color32(Rgba([r, g, b, a]): Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

pub struct Theme {
    pub node_fill: Color32,
    pub node_stroke: Color32,
    pub text: Color32,
    pub edge: Color32,
    pub group: Color32,
    pub selection: Color32,
    pub phantom: Color32,
    pub canvas_bg: Color32,
    pub grid: Color32,
    // editor token colors
    pub tok_keyword: Color32,
    pub tok_string: Color32,
    pub tok_number: Color32,
    pub tok_color: Color32,
    pub tok_comment: Color32,
    pub tok_ident: Color32,
    pub tok_punct: Color32,
    pub diagnostic: Color32,
}

impl Theme {
    pub fn new(dark: bool) -> Self {
        if dark {
            Theme {
                node_fill: Color32::from_rgb(0x2a, 0x31, 0x3c),
                node_stroke: Color32::from_rgb(0x9a, 0xa7, 0xb8),
                text: Color32::from_rgb(0xe8, 0xec, 0xf2),
                edge: Color32::from_rgb(0x8a, 0x96, 0xa8),
                group: Color32::from_rgb(0x6a, 0x76, 0x88),
                selection: Color32::from_rgb(0x4f, 0xa3, 0xff),
                phantom: Color32::from_rgb(0x77, 0x80, 0x8c),
                canvas_bg: Color32::from_rgb(0x14, 0x17, 0x1c),
                grid: Color32::from_rgb(0x1e, 0x23, 0x2b),
                tok_keyword: Color32::from_rgb(0xc7, 0x92, 0xea),
                tok_string: Color32::from_rgb(0xa5, 0xd6, 0x8a),
                tok_number: Color32::from_rgb(0xf0, 0xc6, 0x74),
                tok_color: Color32::from_rgb(0x6c, 0xc7, 0xd8),
                tok_comment: Color32::from_rgb(0x6a, 0x73, 0x7d),
                tok_ident: Color32::from_rgb(0xe8, 0xec, 0xf2),
                tok_punct: Color32::from_rgb(0x9a, 0xa4, 0xb0),
                diagnostic: Color32::from_rgb(0xff, 0x6b, 0x6b),
            }
        } else {
            Theme {
                node_fill: Color32::from_rgb(0xf4, 0xf6, 0xfa),
                node_stroke: Color32::from_rgb(0x3a, 0x45, 0x55),
                text: Color32::from_rgb(0x1a, 0x20, 0x28),
                edge: Color32::from_rgb(0x55, 0x60, 0x70),
                group: Color32::from_rgb(0x8a, 0x94, 0xa6),
                selection: Color32::from_rgb(0x1f, 0x6f, 0xeb),
                phantom: Color32::from_rgb(0x9a, 0xa2, 0xac),
                canvas_bg: Color32::from_rgb(0xfd, 0xfd, 0xfe),
                grid: Color32::from_rgb(0xee, 0xf0, 0xf4),
                tok_keyword: Color32::from_rgb(0x8f, 0x3f, 0xc8),
                tok_string: Color32::from_rgb(0x2e, 0x7d, 0x32),
                tok_number: Color32::from_rgb(0xb2, 0x60, 0x00),
                tok_color: Color32::from_rgb(0x00, 0x77, 0x8b),
                tok_comment: Color32::from_rgb(0x8a, 0x91, 0x99),
                tok_ident: Color32::from_rgb(0x1a, 0x20, 0x28),
                tok_punct: Color32::from_rgb(0x5a, 0x64, 0x70),
                diagnostic: Color32::from_rgb(0xd3, 0x2f, 0x2f),
            }
        }
    }

}
