#![recursion_limit = "512"]
#![allow(long_running_const_eval)]

use smearor_swipe_launcher_plugin_api::FfiCoreContext;

pub use smearor_hyprland_command::*;
pub use smearor_hyprland_dispatch::*;
pub use smearor_hyprland_shared::*;
pub use smearor_hyprland_status::*;

/// Register all JSON converter implementations for Hyprland messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    smearor_hyprland_command::register_json_converters(context);
    smearor_hyprland_dispatch::register_json_converters(context);
    smearor_hyprland_status::register_json_converters(context);
}
