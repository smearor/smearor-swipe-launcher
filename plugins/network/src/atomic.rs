use crate::labels::NetworkLabel;
use crate::personalization::PersonalizationOverride;
use gtk4::Label;
use smearor_model_widget::AtomicWidgetConfig;
use smearor_network_model::ConnectionStateLevel;
use smearor_network_model::NetworkCommandMessage;
use smearor_network_model::NetworkConnectionState;
use smearor_network_model::NetworkInterfaceType;
use smearor_network_model::NetworkStatusMessage;
use smearor_network_model::TOPIC_STATUS;
use smearor_network_model::TOPIC_VPN_PROFILES;
use smearor_network_model::VpnProfilesMessage;
use smearor_network_model::WifiSignalLevel;
use smearor_personalization_model::PersonalizationCommandMessage;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_render_utils::resolve_icon_codepoint;
use smearor_swipe_launcher_plugin_api::AtomicGraphicData;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::Locale;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::ViewData;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;
use smearor_swipe_launcher_plugin_api::apply_text_color;
use smearor_swipe_launcher_plugin_api::atomic_widget_impl;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use tracing::trace;

/// Which network view an atomic widget renders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkAtomicView {
    /// WiFi status: SSID, signal strength.
    WiFiStatus,
    /// WiFi connect: triggers WiFi selection.
    WiFiConnect,
    /// Ethernet status: connection state.
    EthernetStatus,
    /// VPN toggle: active/inactive state.
    VpnToggle,
}

impl NetworkAtomicView {
    /// Returns the default nerd font icon name for this view.
    pub fn icon_name(&self) -> &'static str {
        match self {
            Self::WiFiStatus => "nf-md-wifi",
            Self::WiFiConnect => "nf-md-wifi_plus",
            Self::EthernetStatus => "nf-md-ethernet",
            Self::VpnToggle => "nf-md-shield_key",
        }
    }
}

impl FromStr for NetworkAtomicView {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "network_wifi_status" => Ok(Self::WiFiStatus),
            "network_wifi_connect" => Ok(Self::WiFiConnect),
            "network_ethernet_status" => Ok(Self::EthernetStatus),
            "network_vpn_toggle" => Ok(Self::VpnToggle),
            _ => Err(format!("Unknown network atomic view: {s}")),
        }
    }
}

impl NetworkAtomicView {
    /// Renders this view's display data from the current network status and personalization override.
    pub fn render(&self, status: &NetworkStatusMessage, vpn_profiles: Option<&VpnProfilesMessage>, override_data: &PersonalizationOverride) -> ViewData {
        let locale = override_data.effective_locale();
        match self {
            Self::WiFiStatus => {
                let wifi = status.interfaces.iter().find(|i| i.interface_type == NetworkInterfaceType::Wifi);
                if let Some(iface) = wifi {
                    if iface.state == NetworkConnectionState::Connected {
                        let ssid = iface
                            .ssid
                            .as_ref()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| NetworkLabel::Unknown.localized_label(locale).to_string());
                        let signal = iface.signal.as_ref().copied().unwrap_or(0);
                        let icon = wifi_signal_icon_name(signal);
                        let color = WifiSignalLevel::from_percent(signal).get_icon_color();
                        ViewData::with_color(icon.to_string(), ssid, format!("{}%", signal), color)
                    } else {
                        let color = ConnectionStateLevel::from_state(NetworkConnectionState::Disconnected).get_icon_color();
                        ViewData::with_color(
                            "nf-md-wifi_strength_off".to_string(),
                            NetworkLabel::Disconnected.localized_label(locale).to_string(),
                            String::new(),
                            color,
                        )
                    }
                } else {
                    let color = ConnectionStateLevel::from_state(NetworkConnectionState::Unavailable).get_icon_color();
                    ViewData::with_color(
                        "nf-md-wifi_strength_off".to_string(),
                        NetworkLabel::NoWiFi.localized_label(locale).to_string(),
                        String::new(),
                        color,
                    )
                }
            }
            Self::WiFiConnect => ViewData::new(self.icon_name().to_string(), "Connect".to_string(), NetworkLabel::WiFi.localized_label(locale).to_string()),
            Self::EthernetStatus => {
                let eth = status.interfaces.iter().find(|i| i.interface_type == NetworkInterfaceType::Ethernet);
                if let Some(iface) = eth {
                    let is_connected = iface.state == NetworkConnectionState::Connected;
                    let icon = if is_connected { "nf-md-ethernet" } else { "nf-md-network_off" };
                    let state_text = if is_connected {
                        NetworkLabel::Connected.localized_label(locale)
                    } else {
                        NetworkLabel::Disconnected.localized_label(locale)
                    };
                    let color = ConnectionStateLevel::from_state(iface.state).get_icon_color();
                    ViewData::with_color(icon.to_string(), state_text.to_string(), iface.interface_name.to_string(), color)
                } else {
                    let color = ConnectionStateLevel::from_state(NetworkConnectionState::Disconnected).get_icon_color();
                    ViewData::with_color(
                        "nf-md-network_off".to_string(),
                        NetworkLabel::NoEthernet.localized_label(locale).to_string(),
                        String::new(),
                        color,
                    )
                }
            }
            Self::VpnToggle => {
                let active_profile = vpn_profiles.and_then(|v| v.profiles.iter().find(|p| p.is_active)).map(|p| p.name.to_string());
                let inactive_profile = vpn_profiles.and_then(|v| v.profiles.iter().find(|p| !p.is_active)).map(|p| p.name.to_string());
                if let Some(name) = active_profile {
                    let color = ConnectionStateLevel::from_state(NetworkConnectionState::Connected).get_icon_color();
                    ViewData::with_color("nf-md-shield_key".to_string(), name, NetworkLabel::Active.localized_label(locale).to_string(), color)
                } else if let Some(name) = inactive_profile {
                    let color = ConnectionStateLevel::from_state(NetworkConnectionState::Disconnected).get_icon_color();
                    ViewData::with_color("nf-md-shield_off".to_string(), name, NetworkLabel::Inactive.localized_label(locale).to_string(), color)
                } else {
                    let color = ConnectionStateLevel::from_state(NetworkConnectionState::Unavailable).get_icon_color();
                    ViewData::with_color("nf-md-shield_off".to_string(), NetworkLabel::NoVpn.localized_label(locale).to_string(), String::new(), color)
                }
            }
        }
    }
}

/// Select the nerd font icon name for WiFi signal strength.
fn wifi_signal_icon_name(signal: u8) -> &'static str {
    if signal > 75 {
        "nf-md-wifi_strength_4"
    } else if signal > 50 {
        "nf-md-wifi_strength_3"
    } else if signal > 25 {
        "nf-md-wifi_strength_2"
    } else if signal > 0 {
        "nf-md-wifi_strength_1"
    } else {
        "nf-md-wifi_strength_off"
    }
}

/// Atomic network widget that renders a single network view.
///
/// Subscribes to `service.network.status` and renders only the view specified
/// at construction time. No view switching — each atomic widget is a
/// single-purpose display.
pub struct NetworkAtomicWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: AtomicWidgetConfig,
    pub view: NetworkAtomicView,
    pub icon_label: Rc<RefCell<Option<Label>>>,
    pub main_label: Rc<RefCell<Option<Label>>>,
    pub info_label: Rc<RefCell<Option<Label>>>,
    pub latest_status: Rc<RefCell<Option<NetworkStatusMessage>>>,
    pub latest_vpn_profiles: Rc<RefCell<Option<VpnProfilesMessage>>>,
    pub personalization: Rc<RefCell<PersonalizationOverride>>,
}

impl NetworkAtomicWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: AtomicWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let widget_name = config.config.get("widget").and_then(|v| v.as_str()).unwrap_or_default();

        let view = NetworkAtomicView::from_str(widget_name).unwrap_or(NetworkAtomicView::WiFiStatus);

        let widget = NetworkAtomicWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            view,
            icon_label: Rc::new(RefCell::new(None)),
            main_label: Rc::new(RefCell::new(None)),
            info_label: Rc::new(RefCell::new(None)),
            latest_status: Rc::new(RefCell::new(None)),
            latest_vpn_profiles: Rc::new(RefCell::new(None)),
            personalization: Rc::new(RefCell::new(PersonalizationOverride::default())),
        };
        widget.register_mcp_capabilities();
        widget.request_initial_status();
        widget.request_personalization_status();
        Ok(widget)
    }

    fn request_personalization_status(&self) {
        MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(PersonalizationCommandMessage::request_status());
    }

    fn update_ui(&self, status: &NetworkStatusMessage) {
        let override_data = self.personalization.borrow().clone();
        let vpn = self.latest_vpn_profiles.borrow().clone();
        let view_data = self
            .view
            .render(status, vpn.as_ref(), &override_data)
            .with_text_colors(&self.config.text_colors);
        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or('\u{f0928}');
        smearor_swipe_launcher_plugin_api::update_labels(
            &*self.icon_label.borrow(),
            &*self.main_label.borrow(),
            &*self.info_label.borrow(),
            &icon_char.to_string(),
            &view_data.main_text,
            &view_data.info_text,
        );
        if let Some(ref label) = *self.main_label.borrow() {
            apply_text_color(label, view_data.main_text_color);
        }
        if let Some(ref label) = *self.info_label.borrow() {
            apply_text_color(label, view_data.info_text_color);
        }
    }

    /// Extract graphic rendering data from the latest status.
    fn render_atomic_graphic_data(&self) -> AtomicGraphicData {
        let status = self.latest_status.borrow();
        let Some(status) = status.as_ref() else {
            return AtomicGraphicData::error('\u{f0928}', "Loading...".to_string());
        };

        let override_data = self.personalization.borrow().clone();
        let vpn = self.latest_vpn_profiles.borrow().clone();
        let view_data = self
            .view
            .render(status, vpn.as_ref(), &override_data)
            .with_text_colors(&self.config.text_colors);
        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or('\u{f0928}');
        let mut data = AtomicGraphicData::new(icon_char, view_data.main_text, view_data.info_text);
        data.icon_color = view_data.icon_color.map(|c| c.to_rgba());
        data.main_text_color = view_data.main_text_color.map(|c| c.to_rgba());
        data.info_text_color = view_data.info_text_color.map(|c| c.to_rgba());
        data
    }
}

atomic_widget_impl! {
    widget: NetworkAtomicWidget,
    status: NetworkStatusMessage,
    topic: TOPIC_STATUS,
    debug_tag: "network-atomic",
    mcp_description: "Network atomic widget",
    css_prefix: "network",
    default_icon: '\u{f0928}',
    default_main: "--",
    default_info: "Loading...",
    refresh_command: NetworkCommandMessage::refresh(),
    extra_message_types: [FfiEnvelopePayload<PersonalizationStatusMessage>, FfiEnvelopePayload<VpnProfilesMessage>]
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for NetworkAtomicWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("network atomic widget: received personalization status");
        let status = message.0;
        let locale = status.locale.as_ref().map(|l| Locale::from_str(l).unwrap_or_default()).unwrap_or_default();
        let override_data = PersonalizationOverride {
            measurement_system: Some(status.measurement_system),
            locale,
        };
        *self.personalization.borrow_mut() = override_data;
        if let Some(ref status) = *self.latest_status.borrow() {
            self.update_ui(status);
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<VpnProfilesMessage>> for NetworkAtomicWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<VpnProfilesMessage>, _sender_id: &str) {
        trace!("network atomic widget: received vpn profiles");
        *self.latest_vpn_profiles.borrow_mut() = Some(message.0);
        if let Some(ref status) = *self.latest_status.borrow() {
            self.update_ui(status);
        }
    }
}
