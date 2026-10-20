//! `#RRGGBB` text conversion for colors.

use egui::Color32;

/// Formats a color as `#RRGGBB`.
pub fn to_hex(color: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())
}

/// Parses `#RRGGBB`. Returns None if the text does not match.
pub fn parse_hex(text: &str) -> Option<Color32> {
    let hex = text.trim().strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    Some(Color32::from_rgb(channel(0)?, channel(2)?, channel(4)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        let color = Color32::from_rgb(70, 80, 120);
        assert_eq!(to_hex(color), "#465078");
        assert_eq!(to_hex(parse_hex("#465078").unwrap()), "#465078");
    }

    #[test]
    fn invalid_hex_is_rejected() {
        for text in ["", "465078", "#46507", "#4650789", "#GGGGGG", "#46é078"] {
            assert!(parse_hex(text).is_none(), "{text}");
        }
    }
}
