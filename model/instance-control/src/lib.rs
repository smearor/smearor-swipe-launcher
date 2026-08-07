//! Shared message types for dynamic launcher instance lifecycle management.
//!
//! Plugins and services use these messages to request the creation, start,
//! stop, unload, or reload of launcher instances at runtime via the message broker.

mod instance_type;
mod lifecycle_event;
mod load_message;
mod reload_message;
mod start_message;
mod status_message;
mod stop_message;
mod topics;
mod unload_message;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::impl_json_convertible;

pub use instance_type::InstanceType;
pub use lifecycle_event::LauncherInstanceLifecycle;
pub use lifecycle_event::LauncherInstanceLifecycleTransitionError;
pub use load_message::InstanceLoadMessage;
pub use reload_message::InstanceReloadMessage;
pub use start_message::InstanceStartMessage;
pub use status_message::InstanceStatusMessage;
pub use stop_message::InstanceStopMessage;
pub use topics::TOPIC_CORE_INSTANCE_LOAD;
pub use topics::TOPIC_CORE_INSTANCE_RELOAD;
pub use topics::TOPIC_CORE_INSTANCE_START;
pub use topics::TOPIC_CORE_INSTANCE_STATUS;
pub use topics::TOPIC_CORE_INSTANCE_STOP;
pub use topics::TOPIC_CORE_INSTANCE_UNLOAD;
pub use unload_message::InstanceUnloadMessage;

impl_json_convertible!(InstanceLoadMessageConverter, InstanceLoadMessage, |json: serde_json::Value| serde_json::from_value(json)
    .unwrap_or_default());
impl_json_convertible!(InstanceStartMessageConverter, InstanceStartMessage, |json: serde_json::Value| serde_json::from_value(json)
    .unwrap_or_default());
impl_json_convertible!(InstanceStopMessageConverter, InstanceStopMessage, |json: serde_json::Value| serde_json::from_value(json)
    .unwrap_or_default());
impl_json_convertible!(InstanceUnloadMessageConverter, InstanceUnloadMessage, |json: serde_json::Value| serde_json::from_value(json)
    .unwrap_or_default());
impl_json_convertible!(InstanceReloadMessageConverter, InstanceReloadMessage, |json: serde_json::Value| serde_json::from_value(json)
    .unwrap_or_default());
impl_json_convertible!(InstanceStatusMessageConverter, InstanceStatusMessage, |json: serde_json::Value| serde_json::from_value(json)
    .unwrap_or_default());

/// Register all JSON converter implementations for instance-control messages.
///
/// Call this once during startup.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    InstanceLoadMessageConverter::register_in_host(context);
    InstanceStartMessageConverter::register_in_host(context);
    InstanceStopMessageConverter::register_in_host(context);
    InstanceUnloadMessageConverter::register_in_host(context);
    InstanceReloadMessageConverter::register_in_host(context);
    InstanceStatusMessageConverter::register_in_host(context);
}
