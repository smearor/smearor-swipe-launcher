use super::color::convert_color;
use hyprland::ctl::set_prop::PropType;
use smearor_hyprland_model::HyprlandPropType;
use smearor_hyprland_model::HyprlandPropTypeKind;

pub(crate) fn convert_prop_type(prop: HyprlandPropType) -> PropType {
    let animation_style: Option<stabby::string::String> = prop.animation_style.into();
    let animation_style_string = animation_style.map(|s| s.to_string());
    match prop.kind {
        HyprlandPropTypeKind::AnimationStyle => PropType::AnimationStyle(animation_style_string.unwrap_or_default()),
        HyprlandPropTypeKind::Rounding => PropType::Rounding(prop.rounding, prop.locked),
        HyprlandPropTypeKind::ForceNoBlur => PropType::ForceNoBlur(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::ForceOpaque => PropType::ForceOpaque(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::ForceOpaqueOverriden => PropType::ForceOpaqueOverriden(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::ForceAllowsInput => PropType::ForceAllowsInput(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::ForceNoAnims => PropType::ForceNoAnims(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::ForceNoBorder => PropType::ForceNoBorder(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::ForceNoShadow => PropType::ForceNoShadow(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::WindowDanceCompat => PropType::WindowDanceCompat(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::NoMaxSize => PropType::NoMaxSize(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::DimAround => PropType::DimAround(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::AlphaOverride => PropType::AlphaOverride(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::Alpha => PropType::Alpha(prop.value_float, prop.locked),
        HyprlandPropTypeKind::AlphaInactiveOverride => PropType::AlphaInactiveOverride(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::AlphaInactive => PropType::AlphaInactive(prop.value_float, prop.locked),
        HyprlandPropTypeKind::ActiveBorderColor => PropType::ActiveBorderColor(convert_color(prop.color), prop.locked),
        HyprlandPropTypeKind::InactiveBorderColor => PropType::InactiveBorderColor(convert_color(prop.color), prop.locked),
    }
}
