use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Lock action type for the lock command.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HyprlandLockType {
    #[default]
    Lock,
    Unlock,
    ToggleLock,
}

impl TypedMessage for HyprlandLockType {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::HyprlandLockType");
}
