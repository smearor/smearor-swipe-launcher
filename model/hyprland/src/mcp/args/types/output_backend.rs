use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandOutputBackend;

/// Output backend type for the output-create command.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpOutputBackend {
    /// Wayland backend.
    #[default]
    Wayland,
    /// X11 backend.
    X11,
    /// Headless backend, no physical output.
    Headless,
    /// Automatically select the backend.
    Auto,
}

impl From<McpOutputBackend> for HyprlandOutputBackend {
    fn from(value: McpOutputBackend) -> Self {
        match value {
            McpOutputBackend::Wayland => HyprlandOutputBackend::Wayland,
            McpOutputBackend::X11 => HyprlandOutputBackend::X11,
            McpOutputBackend::Headless => HyprlandOutputBackend::Headless,
            McpOutputBackend::Auto => HyprlandOutputBackend::Auto,
        }
    }
}
