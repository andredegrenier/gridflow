//! Theme presets for canvas, editor and chrome, switchable at runtime from
//! View → Theme. `System` follows the OS light/dark setting; the named
//! presets pin a full palette. The canvas, syntax highlighting, egui chrome
//! and exports all draw from the same `Theme` so switching is total.

use eframe::egui::{self, Color32};
use gridflow_core::export::SceneStyle;
use gridflow_core::model::Rgba;

pub fn to_color32(Rgba([r, g, b, a]): Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

fn to_rgba(c: Color32) -> Rgba {
    Rgba([c.r(), c.g(), c.b(), c.a()])
}

/// What the user picked in View → Theme. Persisted across runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ThemeChoice {
    #[default]
    System,
    Light,
    Dark,
    Midnight,
    Nord,
    Solarized,
    Paper,
}

impl ThemeChoice {
    pub const ALL: &'static [(ThemeChoice, &'static str)] = &[
        (ThemeChoice::System, "System"),
        (ThemeChoice::Light, "Light"),
        (ThemeChoice::Dark, "Dark"),
        (ThemeChoice::Midnight, "Midnight"),
        (ThemeChoice::Nord, "Nord"),
        (ThemeChoice::Solarized, "Solarized Light"),
        (ThemeChoice::Paper, "Paper"),
    ];

    pub fn label(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(c, _)| *c == self)
            .map(|(_, l)| *l)
            .unwrap_or("System")
    }

    pub fn resolve(self, system_dark: bool) -> Theme {
        match self {
            ThemeChoice::System => {
                if system_dark {
                    Theme::dark()
                } else {
                    Theme::light()
                }
            }
            ThemeChoice::Light => Theme::light(),
            ThemeChoice::Dark => Theme::dark(),
            ThemeChoice::Midnight => Theme::midnight(),
            ThemeChoice::Nord => Theme::nord(),
            ThemeChoice::Solarized => Theme::solarized(),
            ThemeChoice::Paper => Theme::paper(),
        }
    }
}

pub struct Theme {
    pub dark: bool,
    pub node_fill: Color32,
    pub node_stroke: Color32,
    pub text: Color32,
    pub edge: Color32,
    pub group: Color32,
    pub selection: Color32,
    pub phantom: Color32,
    pub canvas_bg: Color32,
    pub grid: Color32,
    /// Panel/chrome background, applied to egui visuals.
    pub panel_bg: Color32,
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

const fn c(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

impl Theme {
    pub fn light() -> Self {
        Theme {
            dark: false,
            node_fill: c(0xf4, 0xf6, 0xfa),
            node_stroke: c(0x3a, 0x45, 0x55),
            text: c(0x1a, 0x20, 0x28),
            edge: c(0x55, 0x60, 0x70),
            group: c(0x8a, 0x94, 0xa6),
            selection: c(0x1f, 0x6f, 0xeb),
            phantom: c(0x9a, 0xa2, 0xac),
            canvas_bg: c(0xfd, 0xfd, 0xfe),
            grid: c(0xee, 0xf0, 0xf4),
            panel_bg: c(0xf7, 0xf8, 0xfa),
            tok_keyword: c(0x8f, 0x3f, 0xc8),
            tok_string: c(0x2e, 0x7d, 0x32),
            tok_number: c(0xb2, 0x60, 0x00),
            tok_color: c(0x00, 0x77, 0x8b),
            tok_comment: c(0x8a, 0x91, 0x99),
            tok_ident: c(0x1a, 0x20, 0x28),
            tok_punct: c(0x5a, 0x64, 0x70),
            diagnostic: c(0xd3, 0x2f, 0x2f),
        }
    }

    pub fn dark() -> Self {
        Theme {
            dark: true,
            node_fill: c(0x2a, 0x31, 0x3c),
            node_stroke: c(0x9a, 0xa7, 0xb8),
            text: c(0xe8, 0xec, 0xf2),
            edge: c(0x8a, 0x96, 0xa8),
            group: c(0x6a, 0x76, 0x88),
            selection: c(0x4f, 0xa3, 0xff),
            phantom: c(0x77, 0x80, 0x8c),
            canvas_bg: c(0x14, 0x17, 0x1c),
            grid: c(0x1e, 0x23, 0x2b),
            panel_bg: c(0x1b, 0x1f, 0x26),
            tok_keyword: c(0xc7, 0x92, 0xea),
            tok_string: c(0xa5, 0xd6, 0x8a),
            tok_number: c(0xf0, 0xc6, 0x74),
            tok_color: c(0x6c, 0xc7, 0xd8),
            tok_comment: c(0x6a, 0x73, 0x7d),
            tok_ident: c(0xe8, 0xec, 0xf2),
            tok_punct: c(0x9a, 0xa4, 0xb0),
            diagnostic: c(0xff, 0x6b, 0x6b),
        }
    }

    /// Near-black with electric accents; for OLED and late nights.
    pub fn midnight() -> Self {
        Theme {
            dark: true,
            node_fill: c(0x11, 0x18, 0x24),
            node_stroke: c(0x5c, 0x7a, 0x9e),
            text: c(0xd9, 0xe4, 0xf1),
            edge: c(0x5e, 0x74, 0x8e),
            group: c(0x3e, 0x54, 0x70),
            selection: c(0x36, 0xa3, 0xff),
            phantom: c(0x4e, 0x5c, 0x6e),
            canvas_bg: c(0x05, 0x08, 0x0d),
            grid: c(0x0e, 0x14, 0x1d),
            panel_bg: c(0x0a, 0x0e, 0x15),
            tok_keyword: c(0xff, 0xb4, 0x54),
            tok_string: c(0x7f, 0xd9, 0x62),
            tok_number: c(0xff, 0xd1, 0x73),
            tok_color: c(0x39, 0xd7, 0xe0),
            tok_comment: c(0x48, 0x55, 0x66),
            tok_ident: c(0xd9, 0xe4, 0xf1),
            tok_punct: c(0x7a, 0x8a, 0xa0),
            diagnostic: c(0xff, 0x57, 0x64),
        }
    }

    /// The arctic classic.
    pub fn nord() -> Self {
        Theme {
            dark: true,
            node_fill: c(0x3b, 0x42, 0x52),
            node_stroke: c(0x81, 0xa1, 0xc1),
            text: c(0xec, 0xef, 0xf4),
            edge: c(0x81, 0x8c, 0xa0),
            group: c(0x61, 0x6e, 0x88),
            selection: c(0x88, 0xc0, 0xd0),
            phantom: c(0x6a, 0x74, 0x89),
            canvas_bg: c(0x2e, 0x34, 0x40),
            grid: c(0x35, 0x3c, 0x4a),
            panel_bg: c(0x2b, 0x30, 0x3b),
            tok_keyword: c(0x81, 0xa1, 0xc1),
            tok_string: c(0xa3, 0xbe, 0x8c),
            tok_number: c(0xb4, 0x8e, 0xad),
            tok_color: c(0x88, 0xc0, 0xd0),
            tok_comment: c(0x61, 0x6e, 0x88),
            tok_ident: c(0xd8, 0xde, 0xe9),
            tok_punct: c(0x9a, 0xa5, 0xb8),
            diagnostic: c(0xbf, 0x61, 0x6a),
        }
    }

    /// Solarized light, precise base + accent values.
    pub fn solarized() -> Self {
        Theme {
            dark: false,
            node_fill: c(0xee, 0xe8, 0xd5),
            node_stroke: c(0x58, 0x6e, 0x75),
            text: c(0x07, 0x36, 0x42),
            edge: c(0x65, 0x7b, 0x83),
            group: c(0x93, 0xa1, 0xa1),
            selection: c(0x26, 0x8b, 0xd2),
            phantom: c(0x93, 0xa1, 0xa1),
            canvas_bg: c(0xfd, 0xf6, 0xe3),
            grid: c(0xf2, 0xea, 0xd7),
            panel_bg: c(0xf7, 0xf0, 0xdd),
            tok_keyword: c(0x6c, 0x71, 0xc4),
            tok_string: c(0x85, 0x99, 0x00),
            tok_number: c(0xcb, 0x4b, 0x16),
            tok_color: c(0x2a, 0xa1, 0x98),
            tok_comment: c(0x93, 0xa1, 0xa1),
            tok_ident: c(0x07, 0x36, 0x42),
            tok_punct: c(0x65, 0x7b, 0x83),
            diagnostic: c(0xdc, 0x32, 0x2f),
        }
    }

    /// Warm sepia light theme — easy on the eyes in bright rooms.
    pub fn paper() -> Self {
        Theme {
            dark: false,
            node_fill: c(0xf3, 0xed, 0xe2),
            node_stroke: c(0x5c, 0x50, 0x3e),
            text: c(0x3a, 0x32, 0x26),
            edge: c(0x7a, 0x6e, 0x5c),
            group: c(0xa0, 0x94, 0x80),
            selection: c(0xb0, 0x5f, 0x2c),
            phantom: c(0xa8, 0x9e, 0x8e),
            canvas_bg: c(0xfa, 0xf6, 0xf0),
            grid: c(0xef, 0xe9, 0xdf),
            panel_bg: c(0xf5, 0xf0, 0xe7),
            tok_keyword: c(0x8f, 0x3d, 0x2e),
            tok_string: c(0x4f, 0x72, 0x30),
            tok_number: c(0xa8, 0x62, 0x0e),
            tok_color: c(0x2e, 0x6f, 0x6b),
            tok_comment: c(0xa0, 0x96, 0x86),
            tok_ident: c(0x3a, 0x32, 0x26),
            tok_punct: c(0x6e, 0x62, 0x50),
            diagnostic: c(0xc0, 0x36, 0x2c),
        }
    }

    /// egui chrome (panels, widgets, selection) tinted to match the palette.
    pub fn egui_visuals(&self) -> egui::Visuals {
        let mut v = if self.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        v.panel_fill = self.panel_bg;
        v.window_fill = self.panel_bg;
        v.extreme_bg_color = if self.dark {
            self.canvas_bg
        } else {
            self.canvas_bg.gamma_multiply(0.99)
        };
        v.selection.bg_fill = self.selection.gamma_multiply(0.35);
        v.selection.stroke.color = self.selection;
        v.hyperlink_color = self.selection;
        v.override_text_color = Some(self.text);
        v
    }

    /// Export style matching this theme, so SVG/PNG look like the canvas.
    pub fn scene_style(&self) -> SceneStyle {
        SceneStyle {
            node_fill: to_rgba(self.node_fill),
            node_stroke: to_rgba(self.node_stroke),
            text: to_rgba(self.text),
            edge: to_rgba(self.edge),
            group: to_rgba(self.group),
            background: to_rgba(self.canvas_bg),
        }
    }
}
