//! Embeds the app icon into the Windows executable.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    // build.rs runs on the host, so the target OS must be read at runtime.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.compile().expect("failed to embed the Windows icon");
    }
}
