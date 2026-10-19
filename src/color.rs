//! `#RRGGBB` text conversion for colors.

use macroquad::color::Color;

/// Formats a color as `#RRGGBB`.
pub fn to_hex(color: Color) -> String {
    let [r, g, b, _]: [u8; 4] = color.into();
    format!("#{r:02X}{g:02X}{b:02X}")
}

/// Parses `#RRGGBB`. Returns None if the text does not match.
pub fn parse_hex(text: &str) -> Option<Color> {
    let hex = text.trim().strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    Some(Color::from_rgba(channel(0)?, channel(2)?, channel(4)?, 255))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        let color = Color::from_rgba(70, 80, 120, 255);
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
