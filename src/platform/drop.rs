use std::path::PathBuf;

#[cfg(target_os = "linux")]
use super::linux::LinuxDrop;

/// Paths dropped since the previous poll. Empty when Wayland is not the window system.
pub struct WaylandDrop {
    #[cfg(target_os = "linux")]
    inner: LinuxDrop,
}

impl WaylandDrop {
    /// Starts listening when the window is a Wayland surface. Returns None otherwise.
    pub fn start(ctx: &egui::Context, cc: &eframe::CreationContext<'_>) -> Option<WaylandDrop> {
        #[cfg(target_os = "linux")]
        {
            LinuxDrop::start(ctx.clone(), cc).map(|inner| WaylandDrop { inner })
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (ctx, cc);
            None
        }
    }

    /// Files whose contents have arrived since the last call.
    pub fn poll(&mut self) -> Vec<PathBuf> {
        #[cfg(target_os = "linux")]
        {
            self.inner.poll()
        }
        #[cfg(not(target_os = "linux"))]
        {
            Vec::new()
        }
    }
}

/// Turns one `text/uri-list` line into a local path.
pub(super) fn file_uri(line: &str) -> Option<PathBuf> {
    let rest = line.trim().strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    let path = if rest.starts_with('/') {
        rest
    } else {
        rest.find('/').map(|i| &rest[i..])?
    };
    let decoded = percent_decode(path)?;
    Some(PathBuf::from(decoded))
}

fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_uris_become_local_paths() {
        assert_eq!(
            file_uri("file:///home/kitey/Castle.mmg\r"),
            Some(PathBuf::from("/home/kitey/Castle.mmg"))
        );
        assert_eq!(
            file_uri("file://localhost/tmp/a%20b.mmg"),
            Some(PathBuf::from("/tmp/a b.mmg"))
        );
        assert!(file_uri("https://example.com/a.mmg").is_none());
    }
}
