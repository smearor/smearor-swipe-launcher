pub mod toggle_dpms;
pub mod toggle_fake_fullscreen;
pub mod toggle_floating;
pub mod toggle_fullscreen;
pub mod toggle_group;
pub mod toggle_opaque;
pub mod toggle_pin;
pub mod toggle_pseudo;
pub mod toggle_split;

mod kind;

pub use kind::ToggleDispatchKind;
pub use kind::ToggleDispatchOps;
pub use toggle_dpms::ToggleDpmsDispatchMessage;
pub use toggle_dpms::ToggleDpmsDispatchMessageStabby;
pub use toggle_fake_fullscreen::ToggleFakeFullscreenDispatchMessage;
pub use toggle_floating::ToggleFloatingDispatchMessage;
pub use toggle_fullscreen::ToggleFullscreenDispatchMessage;
pub use toggle_group::ToggleGroupDispatchMessage;
pub use toggle_opaque::ToggleOpaqueDispatchMessage;
pub use toggle_pin::TogglePinDispatchMessage;
pub use toggle_pseudo::TogglePseudoDispatchMessage;
pub use toggle_split::ToggleSplitDispatchMessage;
