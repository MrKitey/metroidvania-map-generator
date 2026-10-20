//! Camera control: pan, zoom and fit-to-map.

use crate::layout::{Layout, MARGIN};
use crate::world::World;
use egui::{Pos2, Rect, Vec2, vec2};

/// Zoom factor applied per `POINTS_PER_STEP` of scroll.
const ZOOM_STEP: f32 = 1.1;
/// Scroll points that equal one 10% zoom step. Matches one mouse-wheel notch.
const POINTS_PER_STEP: f32 = 40.0;
/// Zoom limits, in screen points per world unit.
const MIN_SCALE: f32 = 2.0;
const MAX_SCALE: f32 = 200.0;

/// Pan and zoom state. `scale` is screen points per world unit.
pub struct View {
    /// World point shown at the center of the map area.
    center: Vec2,
    pub scale: f32,
}

impl View {
    pub fn new() -> View {
        View {
            center: Vec2::ZERO,
            scale: 20.0,
        }
    }

    /// Centers the whole map and picks the largest zoom that shows it entirely.
    pub fn fit(&mut self, world: &World, layout: &Layout, map: Rect) {
        let (x0, y0, x1, y1) = world.bounds();
        let w = (x1 - x0) as f32 * layout.cw;
        let h = (y1 - y0) as f32 * layout.ch;
        self.center = vec2(
            x0 as f32 * layout.cw + w / 2.0,
            y0 as f32 * layout.ch + h / 2.0,
        );
        let avail = (map.size() - vec2(2.0 * MARGIN, 2.0 * MARGIN)).max(Vec2::splat(1.0));
        self.scale = (avail.x / w).min(avail.y / h).clamp(MIN_SCALE, MAX_SCALE);
    }

    /// Moves the map by a screen-space drag.
    pub fn pan(&mut self, screen_delta: Vec2) {
        self.center -= screen_delta / self.scale;
    }

    /// Zooms around `cursor`, keeping that world point under it.
    /// `wheel` is the egui scroll delta in points. One mouse notch is about 40.
    pub fn zoom_at(&mut self, wheel: f32, cursor: Pos2, map: Rect) {
        let before = self.screen_to_world(cursor, map);
        let factor = ZOOM_STEP.powf(wheel / POINTS_PER_STEP);
        self.scale = (self.scale * factor).clamp(MIN_SCALE, MAX_SCALE);
        let after = self.screen_to_world(cursor, map);
        self.center += before - after;
    }

    /// Screen position of a world point, in the map area.
    pub fn world_to_screen(&self, world: Vec2, map: Rect) -> Pos2 {
        map.center() + (world - self.center) * self.scale
    }

    /// World point under a screen position, in the map area.
    pub fn screen_to_world(&self, screen: Pos2, map: Rect) -> Vec2 {
        self.center + (screen - map.center()) / self.scale
    }
}
