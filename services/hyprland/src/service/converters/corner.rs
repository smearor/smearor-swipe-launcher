use hyprland::dispatch::Corner;
use smearor_hyprland_model::HyprlandCorner;

pub(crate) fn convert_corner(corner: HyprlandCorner) -> Corner {
    match corner {
        HyprlandCorner::BottomLeft => Corner::BottomLeft,
        HyprlandCorner::BottomRight => Corner::BottomRight,
        HyprlandCorner::TopRight => Corner::TopRight,
        HyprlandCorner::TopLeft => Corner::TopLeft,
    }
}
