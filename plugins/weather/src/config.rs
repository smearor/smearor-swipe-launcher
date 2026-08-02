use serde::Deserialize;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DispatchableBinding;
use smearor_swipe_launcher_plugin_api::WidgetDimensions;
use smearor_swipe_launcher_plugin_api::WidgetIcon;
use smearor_swipe_launcher_plugin_api::WidgetLayout;
use smearor_swipe_launcher_plugin_api::WidgetTextColors;
use smearor_weather_model::WeatherView;
use typed_builder::TypedBuilder;

/// Configuration for the weather widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct WeatherWidgetConfig {
    /// Widget dimensions (width, height) for GTK layout.
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) dimensions: WidgetDimensions,

    /// Widget layout (spacing) for GTK container.
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) layout: WidgetLayout,

    /// The background color of the widget.
    #[builder(default, setter(into))]
    pub(crate) background_color: Option<String>,

    /// Views to cycle through on swipe up/down.
    #[builder(default)]
    pub(crate) views: Vec<WeatherView>,

    /// Widget icon configuration (icon_size, icon_only).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) icon_config: WidgetIcon,

    /// Text color configuration (main_text_color, info_text_color).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) text_colors: WidgetTextColors,

    /// Action bindings for all input triggers.
    #[serde(flatten)]
    #[builder(default)]
    pub actions: ActionBindings,
}

impl WeatherWidgetConfig {
    /// Returns the binding for the given action kind as a `&dyn DispatchableBinding`.
    pub fn binding_for_kind(&self, kind: ActionKind) -> &dyn DispatchableBinding {
        self.actions.binding_for_kind(kind)
    }
}

impl Default for WeatherWidgetConfig {
    fn default() -> Self {
        Self {
            dimensions: WidgetDimensions::default(),
            layout: WidgetLayout::default(),
            background_color: None,
            views: vec![
                WeatherView::Current,
                WeatherView::ForecastToday,
                WeatherView::Wind,
                WeatherView::Humidity,
                WeatherView::Sunshine,
                WeatherView::PrecipitationProbability,
            ],
            icon_config: WidgetIcon::default(),
            text_colors: WidgetTextColors::default(),
            actions: ActionBindings::default(),
        }
    }
}
