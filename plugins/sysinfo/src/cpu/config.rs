use serde::Deserialize;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DispatchableBinding;

use crate::config::PercentageWidgetConfig;

/// Configuration for the CPU widget.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct CpuWidgetConfig {
    /// Shared percentage widget configuration.
    #[serde(flatten)]
    pub percentage: PercentageWidgetConfig,
    /// Whether to display the CPU temperature.
    pub show_temperature: bool,
    /// Format string for the temperature label.
    pub temperature_format: String,
    /// Optional filter to select which temperature component to display.
    ///
    /// Matched against component label and id (case-insensitive substring match).
    /// If `None`, the primary `cpu_temperature` from the service is used.
    /// Example values: `"k10temp"`, `"Tctl"`, `"thermal_zone0"`, `"hwmon0_1"`.
    pub temperature_component: Option<String>,
    /// Action bindings for all input triggers.
    #[serde(flatten)]
    pub actions: ActionBindings,
}

impl Default for CpuWidgetConfig {
    fn default() -> Self {
        Self {
            percentage: PercentageWidgetConfig {
                icon: Some(String::from("nf-fae-chip")),
                value_format: String::from("{cpu_usage:.0}%"),
                ..Default::default()
            },
            show_temperature: true,
            temperature_format: String::from("{cpu_temperature:.0}°C"),
            temperature_component: None,
            actions: ActionBindings::default(),
        }
    }
}

impl CpuWidgetConfig {
    /// Returns the binding for the given action kind as a `&dyn DispatchableBinding`.
    pub fn binding_for_kind(&self, kind: ActionKind) -> &dyn DispatchableBinding {
        self.actions.binding_for_kind(kind)
    }
}
