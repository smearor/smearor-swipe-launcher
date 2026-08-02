/// RGBA color type alias for pixel buffers.
pub type Color = [u8; 4];

/// Background color for inactive buttons (dark gray).
pub const COLOR_BACKGROUND: Color = [30, 30, 30, 255];

/// Background color for active buttons (lighter gray).
pub const COLOR_BACKGROUND_ACTIVE: Color = [60, 60, 60, 255];

/// Text color for inactive labels.
pub const COLOR_TEXT: Color = [240, 240, 240, 255];

/// Text color for active labels (accent blue).
pub const COLOR_TEXT_ACTIVE: Color = [100, 180, 255, 255];

/// Accent / highlight color.
pub const COLOR_ACCENT: Color = [15, 52, 96, 255];

/// State indicator color for "on" state (green).
pub const COLOR_STATE_ON: Color = [0, 255, 136, 255];

/// State indicator color for "off" state (dark gray).
pub const COLOR_STATE_OFF: Color = [68, 68, 68, 255];

/// Returns the appropriate background color based on active state.
pub fn background_color(is_active: bool) -> Color {
    if is_active { COLOR_BACKGROUND_ACTIVE } else { COLOR_BACKGROUND }
}

/// Returns the appropriate text color based on active state.
pub fn text_color(is_active: bool) -> Color {
    if is_active { COLOR_TEXT_ACTIVE } else { COLOR_TEXT }
}
