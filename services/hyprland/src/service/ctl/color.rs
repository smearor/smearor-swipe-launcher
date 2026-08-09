use hyprland::ctl::Color;
use smearor_hyprland_model::HyprlandColor;

pub(crate) fn convert_color(color: HyprlandColor) -> Color {
    Color::new(color.red, color.green, color.blue, color.alpha)
}
