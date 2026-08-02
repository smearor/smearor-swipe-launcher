use serde::Deserialize;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DispatchableBinding;

use crate::config::PercentageWidgetConfig;

/// Configuration for the memory widget.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct MemoryWidgetConfig {
    /// Shared percentage widget configuration.
    #[serde(flatten)]
    pub percentage: PercentageWidgetConfig,
    /// Whether to display the absolute used memory in bytes.
    pub show_used_bytes: bool,
    /// Whether to display the available memory in bytes.
    pub show_available_bytes: bool,
    /// Action bindings for all input triggers.
    #[serde(flatten)]
    pub actions: ActionBindings,
}

impl Default for MemoryWidgetConfig {
    fn default() -> Self {
        Self {
            percentage: PercentageWidgetConfig {
                icon: Some(String::from("nf-md-memory")),
                value_format: String::from("{memory_usage:.0}%"),
                ..Default::default()
            },
            show_used_bytes: false,
            show_available_bytes: false,
            actions: ActionBindings::default(),
        }
    }
}

impl MemoryWidgetConfig {
    /// Returns the binding for the given action kind as a `&dyn DispatchableBinding`.
    pub fn binding_for_kind(&self, kind: ActionKind) -> &dyn DispatchableBinding {
        self.actions.binding_for_kind(kind)
    }
}
