pub(crate) mod config;
pub(crate) mod widget;

use crate::widget::WorkspaceSwitcherWidget;
use smearor_swipe_launcher_plugin_api::widget_plugin;

widget_plugin!(WorkspaceSwitcherWidget);
