use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandWindowIdentifier;

/// Identifies a window by address, class, title, or process ID.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpWindowIdentifier {
    /// The currently active window.
    #[default]
    Active,
    /// A window identified by its process id.
    ProcessId(u32),
    /// A window identified by its address.
    Address(String),
    /// A window identified by a class name regular expression.
    ClassRegularExpression(String),
    /// A window identified by its title.
    Title(String),
}

impl From<McpWindowIdentifier> for HyprlandWindowIdentifier {
    fn from(value: McpWindowIdentifier) -> Self {
        match value {
            McpWindowIdentifier::Active => HyprlandWindowIdentifier::ProcessId(0),
            McpWindowIdentifier::ProcessId(pid) => HyprlandWindowIdentifier::ProcessId(pid),
            McpWindowIdentifier::Address(addr) => HyprlandWindowIdentifier::Address(addr.into()),
            McpWindowIdentifier::ClassRegularExpression(regex) => HyprlandWindowIdentifier::ClassRegularExpression(regex.into()),
            McpWindowIdentifier::Title(title) => HyprlandWindowIdentifier::Title(title.into()),
        }
    }
}
