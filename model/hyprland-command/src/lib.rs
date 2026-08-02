pub mod kill;
pub mod notify;
pub mod output_create;
pub mod output_remove;
pub mod plugin_load;
pub mod plugin_unload;
pub mod reload;
mod set_cursor;
pub mod set_error;
pub mod set_prop;
pub mod switch_xkb_layout;

pub use kill::KillCommandMessage;
pub use notify::NotifyCommandMessage;
pub use output_create::OutputCreateCommandMessage;
pub use output_remove::OutputRemoveCommandMessage;
pub use plugin_load::PluginLoadCommandMessage;
pub use plugin_unload::PluginUnloadCommandMessage;
pub use reload::ReloadCommandMessage;
pub use set_cursor::SetCursorCommandMessage;
pub use set_error::SetErrorCommandMessage;
pub use set_prop::SetPropCommandMessage;
pub use switch_xkb_layout::SwitchXkbLayoutCommandMessage;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::impl_json_convertible;

impl_json_convertible!(KillCommandMessageConverter, KillCommandMessage, |json: serde_json::Value| serde_json::from_value(json)
    .unwrap_or_default());
impl_json_convertible!(ReloadCommandMessageConverter, ReloadCommandMessage, |json: serde_json::Value| serde_json::from_value(json)
    .unwrap_or_default());
impl_json_convertible!(NotifyCommandMessageConverter, NotifyCommandMessage, |json: serde_json::Value| serde_json::from_value(json)
    .unwrap_or_default());
impl_json_convertible!(OutputCreateCommandMessageConverter, OutputCreateCommandMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});
impl_json_convertible!(OutputRemoveCommandMessageConverter, OutputRemoveCommandMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});
impl_json_convertible!(PluginLoadCommandMessageConverter, PluginLoadCommandMessage, |json: serde_json::Value| serde_json::from_value(
    json
)
.unwrap_or_default());
impl_json_convertible!(PluginUnloadCommandMessageConverter, PluginUnloadCommandMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});
impl_json_convertible!(SetCursorCommandMessageConverter, SetCursorCommandMessage, |json: serde_json::Value| serde_json::from_value(
    json
)
.unwrap_or_default());
impl_json_convertible!(SetErrorCommandMessageConverter, SetErrorCommandMessage, |json: serde_json::Value| serde_json::from_value(
    json
)
.unwrap_or_default());
impl_json_convertible!(SetPropCommandMessageConverter, SetPropCommandMessage, |json: serde_json::Value| serde_json::from_value(json)
    .unwrap_or_default());
impl_json_convertible!(SwitchXkbLayoutCommandMessageConverter, SwitchXkbLayoutCommandMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

/// Register JSON converters for command messages.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    KillCommandMessageConverter::register_in_host(context);
    ReloadCommandMessageConverter::register_in_host(context);
    NotifyCommandMessageConverter::register_in_host(context);
    OutputCreateCommandMessageConverter::register_in_host(context);
    OutputRemoveCommandMessageConverter::register_in_host(context);
    PluginLoadCommandMessageConverter::register_in_host(context);
    PluginUnloadCommandMessageConverter::register_in_host(context);
    SetCursorCommandMessageConverter::register_in_host(context);
    SetErrorCommandMessageConverter::register_in_host(context);
    SetPropCommandMessageConverter::register_in_host(context);
    SwitchXkbLayoutCommandMessageConverter::register_in_host(context);
}
