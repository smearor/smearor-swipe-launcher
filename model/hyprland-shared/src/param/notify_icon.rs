use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Icon type for the notify command.
#[repr(i8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HyprlandNotifyIcon {
    #[default]
    Warning,
    Info,
    Hint,
    Error,
    Confused,
    Ok,
    NoIcon,
}

impl TypedMessage for HyprlandNotifyIcon {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandNotifyIcon");
}
