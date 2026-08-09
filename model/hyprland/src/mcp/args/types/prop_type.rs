use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandPropType;

use crate::mcp::args::types::color::McpColor;
use crate::mcp::args::types::prop_type_kind::McpPropTypeKind;

/// A window property to set.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct McpPropType {
    /// The kind of property.
    pub kind: McpPropTypeKind,
    /// Animation style string for the AnimationStyle variant.
    pub animation_style: Option<String>,
    /// Rounding value for the Rounding variant.
    pub rounding: i64,
    /// Boolean value for boolean property variants.
    pub value_bool: bool,
    /// Float value for Alpha/AlphaInactive variants.
    pub value_float: f32,
    /// Color for ActiveBorderColor/InactiveBorderColor variants.
    pub color: McpColor,
    /// Whether the property is locked (second parameter in most variants).
    pub locked: bool,
}

impl From<McpPropType> for HyprlandPropType {
    fn from(value: McpPropType) -> Self {
        HyprlandPropType {
            kind: value.kind.into(),
            animation_style: value.animation_style.map(stabby::string::String::from).into(),
            rounding: value.rounding,
            value_bool: value.value_bool,
            value_float: value.value_float,
            color: value.color.into(),
            locked: value.locked,
        }
    }
}
