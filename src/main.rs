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
mod editor;
mod export;
mod icon_lib;
mod icons;
mod layer;
mod layout;
mod platform;
mod project;
mod render;
mod settings;
mod storage;
mod text;
mod view;
mod world;

use app::App;
use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title(text::app::WINDOW_TITLE),
        ..Default::default()
    };
    eframe::run_native(
        text::app::WINDOW_TITLE,
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
