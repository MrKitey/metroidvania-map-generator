//! Metroidvania map generator.
//!
//! Generates a random map of rooms grouped by biome, shows it with pan and zoom,
//! and lets the user mark boss, save, secret and portal rooms with the keyboard.

// No console window behind the app in Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod biome;
mod biomes;
mod color;
mod common;
mod export;
mod icon_lib;
mod icons;
mod layer;
mod layout;
mod platform;
mod project;
mod render;
mod settings;
mod settings_panel;
mod storage;
mod text;
mod view;
mod world;

use app::App;
use eframe::egui;

fn main() -> eframe::Result {
    eframe::run_native(
        text::app::WINDOW_TITLE,
        native_options(),
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

fn native_options() -> eframe::NativeOptions {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title(text::app::WINDOW_TITLE),
        ..Default::default()
    };
    #[cfg(target_os = "windows")]
    let options = dx12_unless_env(options);
    options
}

/// The Intel UHD 630 Vulkan driver crashes while wgpu creates its instance.
/// `WGPU_BACKEND` still wins when it is set.
#[cfg(target_os = "windows")]
fn dx12_unless_env(mut options: eframe::NativeOptions) -> eframe::NativeOptions {
    if std::env::var_os("WGPU_BACKEND").is_some() {
        return options;
    }
    if let eframe::egui_wgpu::WgpuSetup::CreateNew(setup) = &mut options.wgpu_options.wgpu_setup {
        setup.instance_descriptor.backends = eframe::wgpu::Backends::DX12;
    }
    options
}
