//! Configuration and action dispatch for atomic widgets.

use serde::Deserialize;

use crate::action::ClickBinding;
use crate::action::CompoundLongpressBinding;
use crate::action::DispatchableBinding;
use crate::action::DoublePressBinding;
use crate::action::HoldBinding;
use crate::action::LongpressBinding;
use crate::atomic::action::AtomicAction;
use crate::atomic::render_mode::AtomicRenderMode;
use crate::widget::WidgetMetadata;
use crate::widget::WidgetTextColors;

/// Configuration for an atomic widget.
///
/// Defines MacroPad action bindings and an optional MCP tool description.
/// All fields are optional and default to `None`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AtomicWidgetConfig {
    /// Single-click action binding.
    #[serde(flatten)]
    pub click: ClickBinding,
    /// Long-press action binding.
    #[serde(flatten)]
    pub longpress: LongpressBinding,
    /// Hold action binding (push-to-talk).
    #[serde(flatten)]
    pub hold: HoldBinding,
    /// Double press action binding.
    #[serde(flatten)]
    pub double_press: DoublePressBinding,
    /// Compound longpress action binding.
    #[serde(flatten)]
    pub compound_longpress: CompoundLongpressBinding,
    /// Widget metadata (description for MCP tool registration).
    #[serde(flatten)]
    pub metadata: WidgetMetadata,
    /// Render mode for headless graphic output. Defaults to `Icon`.
    pub render_mode: Option<AtomicRenderMode>,
    /// Whether to show `main_text` in headless rendering. Defaults to `true`.
    pub show_main_text: Option<bool>,
    /// Whether to show `info_text` in headless rendering. Defaults to `true`.
    pub show_info_text: Option<bool>,
    /// Opacity of the semi-transparent text backdrop (0.0 = transparent, 1.0 = opaque).
    /// Only used in `BackgroundOnly` and `Background` modes. Defaults to `0.5`.
    pub text_backdrop_opacity: Option<f32>,
    /// Size of the icon in pixels for headless graphic rendering.
    ///
    /// When `None`, the icon size is derived from the physical button dimensions
    /// (`(min(width, height) * 0.5).min(40)`) to ensure room for `main_text` and
    /// `info_text`. This is intentionally **not** based on `DEFAULT_ICON_SIZE`
    /// because atomic widgets must share button space with text labels.
    pub icon_size: Option<i32>,
    /// Text color configuration (main_text_color, info_text_color).
    #[serde(flatten)]
    pub text_colors: WidgetTextColors,
}

impl Default for AtomicWidgetConfig {
    fn default() -> Self {
        Self {
            click: ClickBinding::default(),
            longpress: LongpressBinding::default(),
            hold: HoldBinding::default(),
            double_press: DoublePressBinding::default(),
            compound_longpress: CompoundLongpressBinding::default(),
            metadata: WidgetMetadata::default(),
            render_mode: None,
            show_main_text: None,
            show_info_text: None,
            text_backdrop_opacity: None,
            icon_size: None,
            text_colors: WidgetTextColors::default(),
        }
    }
}

impl AtomicWidgetConfig {
    /// Dispatches a MacroPad action to the configured topic.
    ///
    /// Uses `DispatchableBinding` trait to look up the binding by action kind,
    /// then dispatches via the broadcaster if configured.
    pub fn dispatch_action(&self, broadcaster: &crate::MessageBroadcasterInner, action: AtomicAction) {
        let binding: &dyn DispatchableBinding = match action {
            AtomicAction::Click => &self.click,
            AtomicAction::Longpress => &self.longpress,
            AtomicAction::HoldStart | AtomicAction::HoldStop => &self.hold,
            AtomicAction::DoublePress => &self.double_press,
            AtomicAction::CompoundLongpress => &self.compound_longpress,
        };
        if binding.is_configured() {
            binding.dispatch(broadcaster);
        }
    }
}
