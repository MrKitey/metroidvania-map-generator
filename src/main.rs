//! Metroidvania map generator.
//!
//! Generates a random map of rooms grouped by biome, shows it with pan and zoom,
//! and lets the user mark boss, save, secret and portal rooms with the keyboard.

// No console window behind the app in Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod biome;
mod color;
mod editor;
mod export;
mod icons;
mod layout;
mod render;
mod settings;
mod storage;
mod view;
mod world;

use app::App;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Metroidvania Map Generator".to_owned(),
        window_width: 1280,
        window_height: 800,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut app = App::new();
    // One iteration per frame.
    while app.frame() {
        next_frame().await;
    }
}
