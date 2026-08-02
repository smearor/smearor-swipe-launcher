use serde::Deserialize;
use smearor_power_model::PowerAction;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DispatchableBinding;
use smearor_swipe_launcher_plugin_api::WidgetDimensions;
use smearor_swipe_launcher_plugin_api::WidgetIcon;
use smearor_swipe_launcher_plugin_api::WidgetLayout;
use smearor_swipe_launcher_plugin_api::WidgetMode;
use smearor_swipe_launcher_plugin_api::WidgetTextColors;
use typed_builder::TypedBuilder;

/// Configuration for the power menu widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct PowerWidgetConfig {
    /// Widget dimensions (width, height) for GTK layout.
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) dimensions: WidgetDimensions,
    /// Widget layout (spacing) for GTK container.
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) layout: WidgetLayout,
    /// Whether to show the shutdown button.
    #[builder(default = true)]
    pub(crate) show_shutdown: bool,
    /// Whether to show the reboot button.
    #[builder(default = true)]
    pub(crate) show_reboot: bool,
    /// Whether to show the suspend button.
    #[builder(default = true)]
    pub(crate) show_suspend: bool,
    /// Whether to show the hibernate button.
    #[builder(default = true)]
    pub(crate) show_hibernate: bool,
    /// Whether to show the lock screen button.
    #[builder(default = true)]
    pub(crate) show_lock: bool,
    /// Whether to show the logout button.
    #[builder(default = true)]
    pub(crate) show_logout: bool,
    /// Whether to show the reboot-to-firmware button.
    #[builder(default = true)]
    pub(crate) show_reboot_to_firmware: bool,
    /// Whether to show inhibitor warnings.
    #[builder(default = true)]
    pub(crate) show_inhibitor_warnings: bool,
    /// Whether to show the countdown overlay.
    #[builder(default = true)]
    pub(crate) show_countdown_overlay: bool,
    /// Whether to show the scheduled action status.
    #[builder(default = true)]
    pub(crate) show_scheduled_status: bool,
    /// Widget icon configuration (icon_size, icon_only).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) icon_config: WidgetIcon,
    /// Text color configuration (main_text_color, info_text_color).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) text_colors: WidgetTextColors,
    /// Widget layout mode (compact or wide).
    #[serde(default)]
    pub(crate) mode: WidgetMode,
    /// Human-readable description of what the power widget does.
    #[serde(default)]
    pub description: Option<String>,
    /// Which power action to select on startup.
    /// One of: "shutdown", "reboot", "suspend", "hibernate", "lock", "logout", "reboot_to_firmware".
    /// Defaults to the first enabled action.
    #[serde(default)]
    pub default_action: Option<String>,
    /// Action bindings for all input triggers.
    #[serde(flatten)]
    #[builder(default)]
    pub actions: ActionBindings,
}

impl Default for PowerWidgetConfig {
    fn default() -> Self {
        Self {
            dimensions: WidgetDimensions::default(),
            layout: WidgetLayout::default(),
            show_shutdown: true,
            show_reboot: true,
            show_suspend: true,
            show_hibernate: true,
            show_lock: true,
            show_logout: true,
            show_reboot_to_firmware: true,
            show_inhibitor_warnings: true,
            show_countdown_overlay: true,
            show_scheduled_status: true,
            icon_config: WidgetIcon::default(),
            text_colors: WidgetTextColors::default(),
            mode: WidgetMode::default(),
            description: None,
            default_action: None,
            actions: ActionBindings::default(),
        }
    }
}

impl PowerWidgetConfig {
    /// Returns the binding for the given action kind as a `&dyn DispatchableBinding`.
    pub fn binding_for_kind(&self, kind: ActionKind) -> &dyn DispatchableBinding {
        self.actions.binding_for_kind(kind)
    }

    /// Builds the list of enabled power actions based on the config flags.
    pub fn enabled_actions(&self) -> Vec<PowerAction> {
        let mut actions = Vec::new();
        if self.show_shutdown {
            actions.push(PowerAction::Shutdown);
        }
        if self.show_reboot {
            actions.push(PowerAction::Reboot);
        }
        if self.show_suspend {
            actions.push(PowerAction::Suspend);
        }
        if self.show_hibernate {
            actions.push(PowerAction::Hibernate);
        }
        if self.show_lock {
            actions.push(PowerAction::Lock);
        }
        if self.show_logout {
            actions.push(PowerAction::Logout);
        }
        if self.show_reboot_to_firmware {
            actions.push(PowerAction::RebootToFirmware);
        }
        actions
    }
}
