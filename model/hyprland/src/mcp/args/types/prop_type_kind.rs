use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandPropTypeKind;

/// The kind of window property to set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpPropTypeKind {
    /// Animation style for the window.
    #[default]
    AnimationStyle,
    /// Corner rounding radius.
    Rounding,
    /// Force disable blur for the window.
    ForceNoBlur,
    /// Force the window to be opaque.
    ForceOpaque,
    /// Force opaque overridden state.
    ForceOpaqueOverriden,
    /// Force the window to allow input.
    ForceAllowsInput,
    /// Force disable animations for the window.
    ForceNoAnims,
    /// Force disable the window border.
    ForceNoBorder,
    /// Force disable the window shadow.
    ForceNoShadow,
    /// Window dance compatibility mode.
    WindowDanceCompat,
    /// Remove maximum size constraints.
    NoMaxSize,
    /// Dim the area around the window.
    DimAround,
    /// Override the active window alpha.
    AlphaOverride,
    /// Active window alpha value.
    Alpha,
    /// Override the inactive window alpha.
    AlphaInactiveOverride,
    /// Inactive window alpha value.
    AlphaInactive,
    /// Active window border color.
    ActiveBorderColor,
    /// Inactive window border color.
    InactiveBorderColor,
}

impl From<McpPropTypeKind> for HyprlandPropTypeKind {
    fn from(value: McpPropTypeKind) -> Self {
        match value {
            McpPropTypeKind::AnimationStyle => HyprlandPropTypeKind::AnimationStyle,
            McpPropTypeKind::Rounding => HyprlandPropTypeKind::Rounding,
            McpPropTypeKind::ForceNoBlur => HyprlandPropTypeKind::ForceNoBlur,
            McpPropTypeKind::ForceOpaque => HyprlandPropTypeKind::ForceOpaque,
            McpPropTypeKind::ForceOpaqueOverriden => HyprlandPropTypeKind::ForceOpaqueOverriden,
            McpPropTypeKind::ForceAllowsInput => HyprlandPropTypeKind::ForceAllowsInput,
            McpPropTypeKind::ForceNoAnims => HyprlandPropTypeKind::ForceNoAnims,
            McpPropTypeKind::ForceNoBorder => HyprlandPropTypeKind::ForceNoBorder,
            McpPropTypeKind::ForceNoShadow => HyprlandPropTypeKind::ForceNoShadow,
            McpPropTypeKind::WindowDanceCompat => HyprlandPropTypeKind::WindowDanceCompat,
            McpPropTypeKind::NoMaxSize => HyprlandPropTypeKind::NoMaxSize,
            McpPropTypeKind::DimAround => HyprlandPropTypeKind::DimAround,
            McpPropTypeKind::AlphaOverride => HyprlandPropTypeKind::AlphaOverride,
            McpPropTypeKind::Alpha => HyprlandPropTypeKind::Alpha,
            McpPropTypeKind::AlphaInactiveOverride => HyprlandPropTypeKind::AlphaInactiveOverride,
            McpPropTypeKind::AlphaInactive => HyprlandPropTypeKind::AlphaInactive,
            McpPropTypeKind::ActiveBorderColor => HyprlandPropTypeKind::ActiveBorderColor,
            McpPropTypeKind::InactiveBorderColor => HyprlandPropTypeKind::InactiveBorderColor,
        }
    }
}
