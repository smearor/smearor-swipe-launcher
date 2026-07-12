use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::messages::shared::color::HyprlandColor;

/// The kind of window property, matching `hyprland::ctl::set_prop::PropType` variants.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HyprlandPropTypeKind {
    #[default]
    AnimationStyle,
    Rounding,
    ForceNoBlur,
    ForceOpaque,
    ForceOpaqueOverriden,
    ForceAllowsInput,
    ForceNoAnims,
    ForceNoBorder,
    ForceNoShadow,
    WindowDanceCompat,
    NoMaxSize,
    DimAround,
    AlphaOverride,
    Alpha,
    AlphaInactiveOverride,
    AlphaInactive,
    ActiveBorderColor,
    InactiveBorderColor,
}

/// A window property to set, matching `hyprland::ctl::set_prop::PropType`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct HyprlandPropType {
    /// The kind of property.
    pub kind: HyprlandPropTypeKind,
    /// Animation style string for the AnimationStyle variant.
    pub animation_style: stabby::option::Option<stabby::string::String>,
    /// Rounding value for the Rounding variant.
    pub rounding: i64,
    /// Boolean value for boolean property variants.
    pub value_bool: bool,
    /// Float value for Alpha/AlphaInactive variants.
    pub value_float: f32,
    /// Color for ActiveBorderColor/InactiveBorderColor variants.
    pub color: HyprlandColor,
    /// Whether the property is locked (second parameter in most variants).
    pub locked: bool,
}

impl TypedMessage for HyprlandPropTypeKind {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandPropTypeKind");
}

impl TypedMessage for HyprlandPropType {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandPropType");
}
