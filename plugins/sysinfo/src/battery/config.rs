use serde::Deserialize;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DispatchableBinding;

use crate::config::PercentageWidgetConfig;

/// Configuration for the battery widget.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct BatteryWidgetConfig {
    /// Shared percentage widget configuration.
    #[serde(flatten)]
    pub percentage: PercentageWidgetConfig,
    /// Whether to display the charging status as text.
    pub show_status_text: bool,
    /// Whether to change the icon based on charging status.
    pub animate_icon: bool,
    /// Action bindings for all input triggers.
    #[serde(flatten)]
    pub actions: ActionBindings,
}

impl Default for BatteryWidgetConfig {
    fn default() -> Self {
        Self {
            percentage: PercentageWidgetConfig {
                icon: Some(String::from("nf-md-battery")),
                value_format: String::from("{level:.0}%"),
                ..Default::default()
            },
            show_status_text: true,
            animate_icon: false,
            actions: ActionBindings::default(),
        }
    }
}

impl BatteryWidgetConfig {
    /// Returns the binding for the given action kind as a `&dyn DispatchableBinding`.
    pub fn binding_for_kind(&self, kind: ActionKind) -> &dyn DispatchableBinding {
        self.actions.binding_for_kind(kind)
    }
}
