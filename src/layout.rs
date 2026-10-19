//! Size of a cell in world units and the screen areas around the map.

use macroquad::prelude::*;

/// Height of the header bar, in screen pixels.
pub const TOP: f32 = 40.0;
/// Free space kept around the map when fitting it, in screen pixels.
pub const MARGIN: f32 = 20.0;

/// Cell size in world units. Cells keep the viewport aspect ratio.
pub struct Layout {
    pub cw: f32,
    pub ch: f32,
    /// Size of one in-game screen in pixels. One cell stands for one screen.
    pub viewport: (u32, u32),
}

impl Layout {
    /// A cell is one unit tall and as wide as the viewport aspect ratio.
    pub fn new(viewport: (u32, u32)) -> Layout {
        Layout {
            cw: viewport.0 as f32 / viewport.1 as f32,
            ch: 1.0,
            viewport,
        }
    }

    /// Cell containing world point `p`. Uses floor so negative cells are correct.
    pub fn cell_at(&self, p: Vec2) -> (i32, i32) {
        (
            (p.x / self.cw).floor() as i32,
            (p.y / self.ch).floor() as i32,
        )
    }
}
