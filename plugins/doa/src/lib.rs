pub(crate) mod config;
pub(crate) mod graphic;
pub(crate) mod html;
pub(crate) mod labels;
pub(crate) mod personalization;
pub(crate) mod widget;

use crate::widget::DoaWidget;
use smearor_swipe_launcher_plugin_api::widget_plugin_graphic;

widget_plugin_graphic!(DoaWidget);
