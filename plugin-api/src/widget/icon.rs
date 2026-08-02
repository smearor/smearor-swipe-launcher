/// RGBA color with components in the range [0.0, 1.0].
///
/// Used by `WidgetIconRendering` implementations to provide semantic
/// coloring for widget icons (e.g. green = safe, red = dangerous).
/// Also used by `WidgetIcon::icon_color` for user-configured icon colors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    /// Red component [0.0, 1.0].
    pub r: f64,
    /// Green component [0.0, 1.0].
    pub g: f64,
    /// Blue component [0.0, 1.0].
    pub b: f64,
    /// Alpha component [0.0, 1.0], defaults to 1.0 (opaque).
    pub a: f64,
}

impl Color {
    /// Creates a new opaque color from RGB components (alpha = 1.0).
    pub const fn new(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Creates a new color from RGBA components.
    pub const fn new_rgba(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    /// Black.
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0);

    /// White.
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0);

    /// Green — safe / low danger.
    pub const GREEN: Self = Self::new(0.0, 0.8, 0.2);

    /// Light green — low danger.
    pub const LIGHT_GREEN: Self = Self::new(0.5, 0.9, 0.3);

    /// Yellow — moderate / caution.
    pub const YELLOW: Self = Self::new(1.0, 0.85, 0.0);

    /// Orange — high danger.
    pub const ORANGE: Self = Self::new(1.0, 0.5, 0.0);

    /// Red — dangerous.
    pub const RED: Self = Self::new(0.9, 0.1, 0.1);

    /// Dark red — extreme danger.
    pub const DARK_RED: Self = Self::new(0.6, 0.0, 0.0);

    /// Dark blue — freezing.
    pub const DARK_BLUE: Self = Self::new(0.0, 0.0, 0.5);

    /// Blue — cold.
    pub const BLUE: Self = Self::new(0.1, 0.3, 0.9);

    /// Light blue — cool.
    pub const LIGHT_BLUE: Self = Self::new(0.4, 0.7, 1.0);

    /// Returns the CSS class name for this color.
    pub fn css_class(&self) -> &'static str {
        if *self == Self::GREEN {
            "icon-color-green"
        } else if *self == Self::LIGHT_GREEN {
            "icon-color-light-green"
        } else if *self == Self::YELLOW {
            "icon-color-yellow"
        } else if *self == Self::ORANGE {
            "icon-color-orange"
        } else if *self == Self::RED {
            "icon-color-red"
        } else if *self == Self::DARK_RED {
            "icon-color-dark-red"
        } else if *self == Self::DARK_BLUE {
            "icon-color-dark-blue"
        } else if *self == Self::BLUE {
            "icon-color-blue"
        } else if *self == Self::LIGHT_BLUE {
            "icon-color-light-blue"
        } else if *self == Self::BLACK {
            "icon-color-black"
        } else if *self == Self::WHITE {
            "icon-color-white"
        } else {
            "icon-color-default"
        }
    }

    /// Converts to RGBA bytes `[r, g, b, a]` for pixel-buffer rendering.
    pub fn to_rgba(&self) -> [u8; 4] {
        [
            (self.r.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.g.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.b.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.a.clamp(0.0, 1.0) * 255.0).round() as u8,
        ]
    }

    /// Converts to a canonical hex string representation `#rrggbbaa`.
    pub fn to_hex_string(&self) -> String {
        let r = (self.r.clamp(0.0, 1.0) * 255.0).round() as u8;
        let g = (self.g.clamp(0.0, 1.0) * 255.0).round() as u8;
        let b = (self.b.clamp(0.0, 1.0) * 255.0).round() as u8;
        let a = (self.a.clamp(0.0, 1.0) * 255.0).round() as u8;
        format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a)
    }
}

/// Trait for types that can provide icon rendering hints.
///
/// Implementations return an optional color (for semantic coloring based on
/// danger level or category) and an optional icon name (for data-driven icon
/// selection, e.g. wind direction icons).
pub trait WidgetIconRendering {
    /// Returns a semantic color for the icon, if applicable.
    fn get_icon_color(&self) -> Option<Color>;

    /// Returns a data-driven icon name, if applicable.
    fn get_icon_name(&self) -> Option<String>;

    fn get_icon_name_or_default(&self, default_value: &str) -> String {
        self.get_icon_name().unwrap_or(default_value.to_string())
    }
}

/// Error returned when parsing a hex color string fails.
#[derive(Debug, thiserror::Error)]
pub enum ColorParseError {
    /// The input does not match any supported format (`#rgb`, `#rrggbb`, `#rrggbbaa`).
    #[error("invalid hex color: expected #rgb, #rrggbb, or #rrggbbaa, got '{0}'")]
    InvalidFormat(String),
    /// The input contains non-hexadecimal characters.
    #[error("invalid hex digit in color '{0}'")]
    InvalidHexDigit(String),
}

impl std::str::FromStr for Color {
    type Err = ColorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix('#').unwrap_or(s);
        match s.len() {
            3 => {
                let r = u8::from_str_radix(&s[0..1], 16).map_err(|_| ColorParseError::InvalidHexDigit(s.to_string()))?;
                let g = u8::from_str_radix(&s[1..2], 16).map_err(|_| ColorParseError::InvalidHexDigit(s.to_string()))?;
                let b = u8::from_str_radix(&s[2..3], 16).map_err(|_| ColorParseError::InvalidHexDigit(s.to_string()))?;
                Ok(Self::new_rgba((r as f64) * 17.0 / 255.0, (g as f64) * 17.0 / 255.0, (b as f64) * 17.0 / 255.0, 1.0))
            }
            6 => {
                let r = u8::from_str_radix(&s[0..2], 16).map_err(|_| ColorParseError::InvalidHexDigit(s.to_string()))?;
                let g = u8::from_str_radix(&s[2..4], 16).map_err(|_| ColorParseError::InvalidHexDigit(s.to_string()))?;
                let b = u8::from_str_radix(&s[4..6], 16).map_err(|_| ColorParseError::InvalidHexDigit(s.to_string()))?;
                Ok(Self::new_rgba(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0, 1.0))
            }
            8 => {
                let r = u8::from_str_radix(&s[0..2], 16).map_err(|_| ColorParseError::InvalidHexDigit(s.to_string()))?;
                let g = u8::from_str_radix(&s[2..4], 16).map_err(|_| ColorParseError::InvalidHexDigit(s.to_string()))?;
                let b = u8::from_str_radix(&s[4..6], 16).map_err(|_| ColorParseError::InvalidHexDigit(s.to_string()))?;
                let a = u8::from_str_radix(&s[6..8], 16).map_err(|_| ColorParseError::InvalidHexDigit(s.to_string()))?;
                Ok(Self::new_rgba(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0, a as f64 / 255.0))
            }
            _ => Err(ColorParseError::InvalidFormat(format!("#{}", s))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rgb_short() {
        let color = "#f60".parse::<Color>().unwrap();
        assert_eq!(color, Color::new_rgba(1.0, 102.0 / 255.0, 0.0, 1.0));
    }

    #[test]
    fn parse_rrggbb() {
        let color = "#ff6600".parse::<Color>().unwrap();
        assert_eq!(color, Color::new_rgba(1.0, 102.0 / 255.0, 0.0, 1.0));
    }

    #[test]
    fn parse_rrggbbaa() {
        let color = "#ff660080".parse::<Color>().unwrap();
        assert_eq!(color, Color::new_rgba(1.0, 102.0 / 255.0, 0.0, 128.0 / 255.0));
    }

    #[test]
    fn parse_without_hash() {
        let color = "ff6600".parse::<Color>().unwrap();
        assert_eq!(color, Color::new_rgba(1.0, 102.0 / 255.0, 0.0, 1.0));
    }

    #[test]
    fn parse_invalid_format() {
        assert!("#ff".parse::<Color>().is_err());
        assert!("#ff660".parse::<Color>().is_err());
        assert!("#ff6600000".parse::<Color>().is_err());
    }

    #[test]
    fn parse_invalid_hex_digit() {
        assert!("#ff660g".parse::<Color>().is_err());
        assert!("#zzzzzz".parse::<Color>().is_err());
    }

    #[test]
    fn to_hex_string_roundtrip() {
        let color = Color::new_rgba(1.0, 102.0 / 255.0, 0.0, 128.0 / 255.0);
        assert_eq!(color.to_hex_string(), "#ff660080");
    }

    #[test]
    fn to_rgba_uses_alpha() {
        let color = Color::new_rgba(1.0, 0.0, 0.0, 0.5);
        let rgba = color.to_rgba();
        assert_eq!(rgba, [255, 0, 0, 128]);
    }

    #[test]
    fn existing_constants_are_opaque() {
        assert_eq!(Color::GREEN.a, 1.0);
        assert_eq!(Color::RED.a, 1.0);
        assert_eq!(Color::BLACK.a, 1.0);
    }
}
