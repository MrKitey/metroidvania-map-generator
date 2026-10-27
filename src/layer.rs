//! Independent map layers. Each layer owns its own grid and rooms.

use crate::color::{parse_hex, to_hex};
use crate::world::{Door, MapError, Room, World};
use egui::Color32;
use serde::{Deserialize, Serialize};

/// Default palette cycled for new layers.
const PALETTE: &[[u8; 3]] = &[
    [74, 144, 217],
    [217, 74, 74],
    [74, 217, 120],
    [217, 168, 74],
    [168, 74, 217],
    [74, 200, 200],
    [217, 74, 168],
    [120, 120, 217],
];

const MIN_OPACITY: f32 = 0.05;
const MAX_OPACITY: f32 = 1.0;

/// One map layer with its own rooms and doors.
#[derive(Clone, PartialEq)]
pub struct Layer {
    pub name: String,
    pub color: [u8; 3],
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32,
    pub world: World,
}

impl Layer {
    pub fn new(name: String, color: [u8; 3]) -> Layer {
        Layer {
            name,
            color,
            visible: true,
            locked: false,
            opacity: 1.0,
            world: World::default(),
        }
    }

    pub fn color32(&self) -> Color32 {
        Color32::from_rgb(self.color[0], self.color[1], self.color[2])
    }

    pub fn clamp_opacity(&mut self) {
        self.opacity = self.opacity.clamp(MIN_OPACITY, MAX_OPACITY);
    }

    /// Fill for the group background (low alpha of the border color).
    pub fn group_fill(&self) -> Color32 {
        let a = (self.opacity * 0.15 * 255.0).round() as u8;
        Color32::from_rgba_unmultiplied(self.color[0], self.color[1], self.color[2], a)
    }

    /// Stroke for the group border.
    pub fn group_stroke(&self) -> Color32 {
        let a = (self.opacity * 255.0).round() as u8;
        Color32::from_rgba_unmultiplied(self.color[0], self.color[1], self.color[2], a)
    }
}

/// Stack of layers. List order is draw order (bottom to top).
#[derive(Clone, PartialEq)]
pub struct LayerStack {
    layers: Vec<Layer>,
    active: usize,
    /// Next index used in default names (`Layer N`).
    next_name: usize,
}

impl LayerStack {
    /// One layer containing `world`, named Layer 0.
    pub fn from_world(world: World) -> LayerStack {
        LayerStack {
            layers: vec![Layer::new("Layer 0".into(), PALETTE[0])],
            active: 0,
            next_name: 1,
        }
        .with_world(0, world)
    }

    fn with_world(mut self, idx: usize, world: World) -> LayerStack {
        self.layers[idx].world = world;
        self
    }

    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    pub fn layers_mut(&mut self) -> &mut [Layer] {
        &mut self.layers
    }

    pub fn len(&self) -> usize {
        self.layers.len()
    }

    pub fn active(&self) -> usize {
        self.active
    }

    pub fn set_active(&mut self, idx: usize) {
        if idx < self.layers.len() {
            self.active = idx;
        }
    }

    pub fn active_world(&self) -> &World {
        &self.layers[self.active].world
    }

    pub fn active_world_mut(&mut self) -> &mut World {
        &mut self.layers[self.active].world
    }

    pub fn is_active_locked(&self) -> bool {
        self.layers[self.active].locked
    }

    /// Total room count across all layers.
    pub fn room_count(&self) -> usize {
        self.layers.iter().map(|l| l.world.rooms.len()).sum()
    }

    /// Union of visible non-empty layer bounds, in cells.
    pub fn visible_bounds(&self) -> Option<(i32, i32, i32, i32)> {
        let mut out: Option<(i32, i32, i32, i32)> = None;
        for layer in &self.layers {
            if !layer.visible {
                continue;
            }
            let Some(b) = layer.world.bounds_opt() else {
                continue;
            };
            out = Some(match out {
                None => b,
                Some(a) => (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)),
            });
        }
        out
    }

    /// Room under cell `(x, y)`, top visible layer first.
    pub fn room_at(&self, x: i32, y: i32) -> Option<(usize, usize)> {
        for (li, layer) in self.layers.iter().enumerate().rev() {
            if !layer.visible {
                continue;
            }
            if let Some(ri) = layer.world.room_at(x, y) {
                return Some((li, ri));
            }
        }
        None
    }

    pub fn add_empty(&mut self) -> usize {
        let name = format!("Layer {}", self.next_name);
        let color = PALETTE[self.next_name % PALETTE.len()];
        self.next_name += 1;
        self.layers.push(Layer::new(name, color));
        let idx = self.layers.len() - 1;
        self.active = idx;
        idx
    }

    /// Duplicate a layer. Returns the new index, or None if `idx` is invalid.
    pub fn duplicate(&mut self, idx: usize) -> Option<usize> {
        let src = self.layers.get(idx)?;
        let name = format!("{} copy", src.name);
        let color = PALETTE[self.next_name % PALETTE.len()];
        self.next_name += 1;
        let mut layer = Layer::new(name, color);
        layer.visible = src.visible;
        layer.locked = false;
        layer.opacity = src.opacity;
        layer.color = src.color;
        layer.world = src.world.clone();
        let insert_at = idx + 1;
        self.layers.insert(insert_at, layer);
        self.active = insert_at;
        Some(insert_at)
    }

    /// Remove a layer. Layer 0 cannot be removed. Returns false if refused.
    pub fn remove(&mut self, idx: usize) -> bool {
        if idx == 0 || idx >= self.layers.len() || self.layers.len() <= 1 {
            return false;
        }
        self.layers.remove(idx);
        if self.active == idx {
            self.active = idx.saturating_sub(1).min(self.layers.len() - 1);
        } else if self.active > idx {
            self.active -= 1;
        }
        true
    }

    /// Move layer from `from` to `to` (list indices).
    pub fn reorder(&mut self, from: usize, to: usize) {
        if from >= self.layers.len() || to >= self.layers.len() || from == to {
            return;
        }
        let layer = self.layers.remove(from);
        self.layers.insert(to, layer);
        if self.active == from {
            self.active = to;
        } else if from < self.active && to >= self.active {
            self.active -= 1;
        } else if from > self.active && to <= self.active {
            self.active += 1;
        }
    }

    /// Rebuild from saved layer data.
    pub fn from_saved(
        active: usize,
        layers: Vec<SavedLayer>,
        biome_count: usize,
    ) -> Result<LayerStack, MapError> {
        if layers.is_empty() {
            return Err(MapError::Empty);
        }
        let mut next_name = layers.len();
        let mut built = Vec::with_capacity(layers.len());
        for saved in layers {
            let world = if saved.rooms.is_empty() {
                World::default()
            } else {
                World::from_parts(saved.rooms, saved.doors, saved.used_biomes, biome_count)?
            };
            let color = parse_hex(&saved.color)
                .map(|c| [c.r(), c.g(), c.b()])
                .unwrap_or(PALETTE[0]);
            let mut layer = Layer::new(saved.name, color);
            layer.visible = saved.visible;
            layer.locked = saved.locked;
            layer.opacity = saved.opacity.clamp(MIN_OPACITY, MAX_OPACITY);
            layer.world = world;
            if let Some(n) = layer
                .name
                .strip_prefix("Layer ")
                .and_then(|s| s.parse::<usize>().ok())
            {
                next_name = next_name.max(n + 1);
            }
            built.push(layer);
        }
        let active = active.min(built.len() - 1);
        Ok(LayerStack {
            layers: built,
            active,
            next_name,
        })
    }

    pub fn to_saved(&self) -> Vec<SavedLayer> {
        self.layers.iter().map(SavedLayer::from_layer).collect()
    }
}

impl SavedLayer {
    fn from_layer(l: &Layer) -> SavedLayer {
        SavedLayer {
            name: l.name.clone(),
            color: to_hex(l.color32()),
            visible: l.visible,
            locked: l.locked,
            opacity: l.opacity,
            rooms: l.world.rooms.clone(),
            doors: l.world.doors.clone(),
            used_biomes: l.world.biomes.clone(),
        }
    }

    /// Same as `*self == SavedLayer::from_layer(l)`, without copying the rooms and doors.
    pub fn matches(&self, l: &Layer) -> bool {
        self.name == l.name
            && self.visible == l.visible
            && self.locked == l.locked
            && self.opacity == l.opacity
            && self.rooms == l.world.rooms
            && self.doors == l.world.doors
            && self.used_biomes == l.world.biomes
            && self.color == to_hex(l.color32())
    }
}

/// One layer as stored in `layers.json`.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedLayer {
    pub name: String,
    pub color: String,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    pub rooms: Vec<Room>,
    pub doors: Vec<Door>,
    pub used_biomes: Vec<usize>,
}

fn default_true() -> bool {
    true
}

fn default_opacity() -> f32 {
    1.0
}
