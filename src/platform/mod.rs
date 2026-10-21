//! File drops on Wayland.
//!
//! winit 0.30 reports a dropped file on X11, macOS and Windows, but not on Wayland.
//! This listens on the same display connection and reads `text/uri-list` offers.

mod drop;

#[cfg(target_os = "linux")]
mod linux;

pub use drop::WaylandDrop;
