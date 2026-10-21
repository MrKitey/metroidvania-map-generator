//! Registers the `.mmg` type with the desktop.

use crate::storage;
use crate::text::files;
use std::fs;

/// Registers the `.mmg` type with the desktop, once. Failures are ignored.
/// A missing `xdg-mime` is retried on a later launch.
pub fn register_mime() {
    #[cfg(target_os = "linux")]
    linux_mime();
}

#[cfg(target_os = "linux")]
fn linux_mime() {
    let dir = storage::app_dir();
    let stamp = dir.join(files::MIME_STAMP);
    let legacy = dir.join(files::MIME_XML_OLD);
    if stamp.exists() && !legacy.exists() {
        return;
    }
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let xml = dir.join(files::MIME_XML);
    let png = dir.join("icon.png");
    if fs::write(
        &xml,
        include_str!("../../packaging/linux/metroidvania-mmg.xml"),
    )
    .is_err()
    {
        return;
    }
    let bytes = include_bytes!("../../assets/icon.png");
    if fs::write(&png, bytes).is_err() {
        return;
    }
    let Ok(status) = std::process::Command::new("xdg-mime")
        .args(["install", "--mode", "user"])
        .arg(&xml)
        .status()
    else {
        return;
    };
    if !status.success() {
        return;
    }
    if legacy.exists() {
        let _ = std::process::Command::new("xdg-mime")
            .args(["uninstall", "--mode", "user"])
            .arg(&legacy)
            .status();
        let _ = fs::remove_file(&legacy);
    }
    if let Some(side) = png_side(bytes) {
        let _ = std::process::Command::new("xdg-icon-resource")
            .args([
                "install",
                "--mode",
                "user",
                "--noupdate",
                "--context",
                "apps",
                "--size",
                &side.to_string(),
            ])
            .arg(&png)
            .arg("metroidvania-map-generator")
            .status();
        let _ = std::process::Command::new("xdg-icon-resource")
            .arg("forceupdate")
            .status();
    }
    let _ = fs::write(&stamp, "");
}

/// Width of a square PNG, read from the IHDR chunk. None when the file is not square.
#[cfg(target_os = "linux")]
fn png_side(bytes: &[u8]) -> Option<u32> {
    if bytes.len() < 24 || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    (width == height && width > 0).then_some(width)
}
