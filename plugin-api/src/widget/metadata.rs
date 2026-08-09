use serde::Deserialize;
use serde::Serialize;
use typed_builder::TypedBuilder;

/// Widget metadata for MCP tool registration.
///
/// When flattened into a widget config via `#[serde(flatten)]`, the TOML
/// fields `description` and `title` map directly to this struct.
#[derive(Debug, Clone, Deserialize, Serialize, TypedBuilder)]
#[serde(default)]
pub struct WidgetMetadata {
    /// Human-readable description of what the widget does.
    #[builder(default, setter(into))]
    pub description: Option<String>,
    /// Human-readable title for UI display.
    /// Maps to the MCP tool's `title` field.
    #[builder(default, setter(into))]
    pub title: Option<String>,
}

impl Default for WidgetMetadata {
    fn default() -> Self {
        Self {
            description: None,
            title: None,
        }
    }
}

impl WidgetMetadata {
    /// Returns the description if set, or `None` if not configured.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns the title if set, or `None` if not configured.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
}
