//! Camera control: pan, zoom and fit-to-map.

use crate::layout::{Layout, MARGIN, TOP};
use crate::world::World;
use macroquad::prelude::*;

/// Zoom factor applied per wheel notch.
const ZOOM_STEP: f32 = 1.1;
/// Zoom limits, in screen pixels per world unit.
const MIN_SCALE: f32 = 2.0;
const MAX_SCALE: f32 = 200.0;

/// Pan and zoom state. `scale` is screen pixels per world unit.
pub struct View {
    /// World point shown at the center of the map area.
    center: Vec2,
    pub scale: f32,
    /// Mouse position on the previous frame, used to compute the drag distance.
    last_mouse: Vec2,
}

impl View {
    pub fn new() -> View {
        View {
            center: Vec2::ZERO,
            scale: 20.0,
            last_mouse: Vec2::ZERO,
        }
    }

    /// Centers the whole map and picks the largest zoom that shows it entirely.
    pub fn fit(&mut self, world: &World, layout: &Layout, reserved: f32) {
        let (x0, y0, x1, y1) = world.bounds();
        let w = (x1 - x0) as f32 * layout.cw;
        let h = (y1 - y0) as f32 * layout.ch;
        self.center = vec2(
            x0 as f32 * layout.cw + w / 2.0,
            y0 as f32 * layout.ch + h / 2.0,
        );
        let avail_w = screen_width() - reserved - 2.0 * MARGIN;
        let avail_h = screen_height() - TOP - MARGIN;
        self.scale = (avail_w / w).min(avail_h / h).clamp(MIN_SCALE, MAX_SCALE);
    }

    /// Middle drag pans, wheel zooms around the cursor. Input is ignored when `active` is false.
    pub fn update(&mut self, reserved: f32, active: bool) {
        let mouse = Vec2::from(mouse_position());
        if active && is_mouse_button_down(MouseButton::Middle) {
            // Move the map by the same distance as the mouse, converted to world units.
            self.center -= (mouse - self.last_mouse) / self.scale;
        }
        self.last_mouse = mouse;

        let wheel = mouse_wheel().1;
        if active && wheel != 0.0 {
            // Only the sign is used: wheel magnitude differs between platforms.
            // The center is then shifted so the world point under the cursor stays put.
            let before = self.camera(reserved).screen_to_world(mouse);
            self.scale = (self.scale * ZOOM_STEP.powf(wheel.signum())).clamp(MIN_SCALE, MAX_SCALE);
            let after = self.camera(reserved).screen_to_world(mouse);
            self.center += before - after;
        }
    }

    /// Y points down. The view is centered in the area below the header and left of `reserved`.
    pub fn camera(&self, reserved: f32) -> Camera2D {
        let (sw, sh) = (screen_width(), screen_height());
        // Camera2D works in normalized device coordinates, where the screen spans -1 to 1.
        // `zoom` turns world units into that space, `offset` shifts the center there.
        Camera2D {
            target: self.center,
            zoom: vec2(2.0 * self.scale / sw, 2.0 * self.scale / sh),
            offset: vec2(-reserved / sw, -TOP / sh),
            ..Default::default()
        }
    }
}
