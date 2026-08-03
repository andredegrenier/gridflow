//! World<->screen transform for the canvas. Zoom is anchored at the pointer so
//! the world point under the cursor stays put.

use eframe::egui::{Pos2, Rect, Vec2};

pub const MIN_ZOOM: f32 = 0.05;
pub const MAX_ZOOM: f32 = 16.0;

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Camera {
    /// World coordinates shown at the viewport center.
    pub center: [f32; 2],
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Camera { center: [300.0, 260.0], zoom: 1.0 }
    }
}

impl Camera {
    pub fn world_to_screen(&self, world: [f32; 2], viewport: Rect) -> Pos2 {
        let c = viewport.center();
        Pos2::new(
            c.x + (world[0] - self.center[0]) * self.zoom,
            c.y + (world[1] - self.center[1]) * self.zoom,
        )
    }

    pub fn screen_to_world(&self, screen: Pos2, viewport: Rect) -> [f32; 2] {
        let c = viewport.center();
        [
            self.center[0] + (screen.x - c.x) / self.zoom,
            self.center[1] + (screen.y - c.y) / self.zoom,
        ]
    }

    pub fn pan_screen(&mut self, delta: Vec2) {
        self.center[0] -= delta.x / self.zoom;
        self.center[1] -= delta.y / self.zoom;
    }

    /// Multiply zoom by `factor`, keeping the world point under `anchor` fixed.
    pub fn zoom_at(&mut self, factor: f32, anchor: Pos2, viewport: Rect) {
        let before = self.screen_to_world(anchor, viewport);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let after = self.screen_to_world(anchor, viewport);
        self.center[0] += before[0] - after[0];
        self.center[1] += before[1] - after[1];
    }

    /// Frame a world rect with 10% margin.
    pub fn fit(&mut self, min: [f32; 2], max: [f32; 2], viewport: Rect) {
        let w = (max[0] - min[0]).max(1.0);
        let h = (max[1] - min[1]).max(1.0);
        self.zoom = ((viewport.width() / w).min(viewport.height() / h) * 0.9)
            .clamp(MIN_ZOOM, MAX_ZOOM);
        self.center = [(min[0] + max[0]) / 2.0, (min[1] + max[1]) / 2.0];
    }
}
