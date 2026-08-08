pub(crate) mod atomic;
pub(crate) mod config;
pub(crate) mod graphic;
pub(crate) mod html;
pub(crate) mod mcp;
pub(crate) mod nav_target;
pub(crate) mod personalization;
pub(crate) mod widget;

use crate::atomic::WorkspaceAtomicWidget;
use crate::widget::WorkspaceSwitcherWidget;
use smearor_swipe_launcher_plugin_api::widget_factory_plugin_graphic;

widget_factory_plugin_graphic! {
    "workspace_switcher" => workspace_switcher_widget => WorkspaceSwitcherWidget => html,
    "workspace_next" => workspace_next_widget => WorkspaceAtomicWidget,
    "workspace_previous" => workspace_previous_widget => WorkspaceAtomicWidget,
    "workspace_name" => workspace_name_widget => WorkspaceAtomicWidget,
    "workspace_select" => workspace_select_widget => WorkspaceAtomicWidget,
}
