use crate::config::NetworkWidgetConfig;
use crate::labels::NetworkLabel;
use crate::personalization::PersonalizationOverride;
use gtk4::Align;
use gtk4::DrawingArea;
use gtk4::Image;
use gtk4::Label;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::prelude::BoxExt;
use gtk4::prelude::WidgetExt;
use gtk4::prelude::*;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_widget::WidgetUpdateMessage;
use smearor_network_model::ConnectionStateLevel;
use smearor_network_model::NetworkCommandMessage;
use smearor_network_model::NetworkConnectionState;
use smearor_network_model::NetworkInterfaceType;
use smearor_network_model::NetworkStatusMessage;
use smearor_network_model::NetworkView;
use smearor_network_model::ScanResultsMessage;
use smearor_network_model::TOPIC_SCAN_RESULTS;
use smearor_network_model::TOPIC_STATUS;
use smearor_network_model::TOPIC_VPN_PROFILES;
use smearor_network_model::VpnProfilesMessage;
use smearor_network_model::WifiSignalLevel;
use smearor_personalization_model::PersonalizationCommandMessage;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_personalization_model::TOPIC_STATUS as TOPIC_PERSONALIZATION_STATUS;
use smearor_swipe_launcher_plugin_api::AcceptTopic;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DefaultFallback;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::GestureHandler;
use smearor_swipe_launcher_plugin_api::GestureHandlersConfiguration;
use smearor_swipe_launcher_plugin_api::Locale;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::ViewData;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;
use smearor_swipe_launcher_plugin_api::WidgetPlugin;
use smearor_swipe_launcher_plugin_api::apply_icon_color;
use smearor_swipe_launcher_plugin_api::apply_widget_css_classes;
use smearor_swipe_launcher_plugin_api::build_content_box;
use smearor_swipe_launcher_plugin_api::build_info_label;
use smearor_swipe_launcher_plugin_api::build_main_label;
use smearor_swipe_launcher_plugin_api::build_spacer;
use smearor_swipe_launcher_plugin_api::build_widget_icon;
use smearor_swipe_launcher_plugin_api::resolve_gtk_nerd_icon;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use tracing::debug;
use tracing::trace;

type SharedImage = Rc<RefCell<Option<Image>>>;
type SharedLabel = Rc<RefCell<Option<Label>>>;
type SharedDrawingArea = Rc<RefCell<Option<DrawingArea>>>;
type SharedString = Rc<RefCell<String>>;

pub struct NetworkWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: NetworkWidgetConfig,
    pub status_sender: tokio::sync::mpsc::UnboundedSender<NetworkStatusMessage>,
    pub status_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<NetworkStatusMessage>>,
    pub scan_sender: tokio::sync::mpsc::UnboundedSender<ScanResultsMessage>,
    pub scan_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<ScanResultsMessage>>,
    pub vpn_sender: tokio::sync::mpsc::UnboundedSender<VpnProfilesMessage>,
    pub vpn_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<VpnProfilesMessage>>,
    pub icon_image: SharedImage,
    pub value_label: SharedLabel,
    pub info_label: SharedLabel,
    pub qr_drawing_area: SharedDrawingArea,
    pub spacer_label: SharedLabel,
    pub current_view: Rc<RefCell<usize>>,
    pub latest_status: Rc<RefCell<Option<NetworkStatusMessage>>>,
    pub latest_scan: Rc<RefCell<Option<ScanResultsMessage>>>,
    pub latest_vpn: Rc<RefCell<Option<VpnProfilesMessage>>>,
    pub latest_ssid: SharedString,
    pub latest_password: SharedString,
    pub personalization: Rc<RefCell<PersonalizationOverride>>,
}

impl NetworkWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: NetworkWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let (status_sender, status_receiver) = tokio::sync::mpsc::unbounded_channel::<NetworkStatusMessage>();
        let (scan_sender, scan_receiver) = tokio::sync::mpsc::unbounded_channel::<ScanResultsMessage>();
        let (vpn_sender, vpn_receiver) = tokio::sync::mpsc::unbounded_channel::<VpnProfilesMessage>();

        let widget = NetworkWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            status_sender,
            status_receiver: Some(status_receiver),
            scan_sender,
            scan_receiver: Some(scan_receiver),
            vpn_sender,
            vpn_receiver: Some(vpn_receiver),
            icon_image: Rc::new(RefCell::new(None)),
            value_label: Rc::new(RefCell::new(None)),
            info_label: Rc::new(RefCell::new(None)),
            qr_drawing_area: Rc::new(RefCell::new(None)),
            spacer_label: Rc::new(RefCell::new(None)),
            current_view: Rc::new(RefCell::new(0)),
            latest_status: Rc::new(RefCell::new(None)),
            latest_scan: Rc::new(RefCell::new(None)),
            latest_vpn: Rc::new(RefCell::new(None)),
            latest_ssid: Rc::new(RefCell::new(String::new())),
            latest_password: Rc::new(RefCell::new(String::new())),
            personalization: Rc::new(RefCell::new(PersonalizationOverride::default())),
        };
        widget.register_mcp_capabilities();
        widget.request_personalization_status();
        Ok(widget)
    }

    fn request_personalization_status(&self) {
        MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(PersonalizationCommandMessage::request_status());
    }

    /// Broadcast a WidgetUpdateMessage so headless/Web instances re-render this widget.
    fn broadcast_widget_update(&self) {
        let plugin_id = self.meta.id.to_string();
        let msg = WidgetUpdateMessage::new(&plugin_id, "");
        let broadcaster = self.get_broadcaster();
        broadcaster.broadcast_message_to_topic(msg);
    }

    fn update_ui(&self, status: &NetworkStatusMessage) {
        let view_index = *self.current_view.borrow();
        let view = self.config.views.get(view_index).copied().unwrap_or(NetworkView::WifiStatus);

        if view == NetworkView::QrCode {
            show_qr_view(&self.qr_drawing_area, &self.icon_image, &self.value_label, &self.info_label, &self.spacer_label);
        } else {
            show_normal_view(&self.qr_drawing_area, &self.icon_image, &self.value_label, &self.info_label, &self.spacer_label);

            let scan = self.latest_scan.borrow().clone();
            let vpn = self.latest_vpn.borrow().clone();
            let override_data = self.personalization.borrow().clone();
            let view_data = render_view(status, scan.as_ref(), vpn.as_ref(), &self.config, view, &override_data);

            if let Some(ref img) = *self.icon_image.borrow() {
                update_icon_display(img, &view_data, self.config.icon_config.icon_size(), self.config.icon_config.icon_color());
            }
            if let Some(ref label) = *self.value_label.borrow() {
                label.set_text(&view_data.main_text);
            }
            if let Some(ref label) = *self.info_label.borrow() {
                label.set_text(&view_data.info_text);
            }
        }
        self.broadcast_widget_update();
    }

    fn next_view(&self) {
        if self.config.views.is_empty() {
            return;
        }
        let mut idx = self.current_view.borrow_mut();
        *idx = (*idx + 1) % self.config.views.len();
        let view = self.config.views[*idx];
        drop(idx);

        if let Some(ref status) = *self.latest_status.borrow() {
            if view == NetworkView::QrCode {
                show_qr_view(&self.qr_drawing_area, &self.icon_image, &self.value_label, &self.info_label, &self.spacer_label);
            } else {
                show_normal_view(&self.qr_drawing_area, &self.icon_image, &self.value_label, &self.info_label, &self.spacer_label);

                let scan = self.latest_scan.borrow().clone();
                let vpn = self.latest_vpn.borrow().clone();
                let override_data = self.personalization.borrow().clone();
                let view_data = render_view(status, scan.as_ref(), vpn.as_ref(), &self.config, view, &override_data);
                if let Some(ref img) = *self.icon_image.borrow() {
                    update_icon_display(img, &view_data, self.config.icon_config.icon_size(), self.config.icon_config.icon_color());
                }
                if let Some(ref label) = *self.value_label.borrow() {
                    label.set_text(&view_data.main_text);
                }
                if let Some(ref label) = *self.info_label.borrow() {
                    label.set_text(&view_data.info_text);
                }
            }
        }
        self.broadcast_widget_update();
    }

    fn prev_view(&self) {
        if self.config.views.is_empty() {
            return;
        }
        let mut idx = self.current_view.borrow_mut();
        if *idx == 0 {
            *idx = self.config.views.len() - 1;
        } else {
            *idx -= 1;
        }
        let view = self.config.views[*idx];
        drop(idx);

        if let Some(ref status) = *self.latest_status.borrow() {
            if view == NetworkView::QrCode {
                show_qr_view(&self.qr_drawing_area, &self.icon_image, &self.value_label, &self.info_label, &self.spacer_label);
            } else {
                show_normal_view(&self.qr_drawing_area, &self.icon_image, &self.value_label, &self.info_label, &self.spacer_label);

                let scan = self.latest_scan.borrow().clone();
                let vpn = self.latest_vpn.borrow().clone();
                let override_data = self.personalization.borrow().clone();
                let view_data = render_view(status, scan.as_ref(), vpn.as_ref(), &self.config, view, &override_data);
                if let Some(ref img) = *self.icon_image.borrow() {
                    update_icon_display(img, &view_data, self.config.icon_config.icon_size(), self.config.icon_config.icon_color());
                }
                if let Some(ref label) = *self.value_label.borrow() {
                    label.set_text(&view_data.main_text);
                }
                if let Some(ref label) = *self.info_label.borrow() {
                    label.set_text(&view_data.info_text);
                }
            }
        }
        self.broadcast_widget_update();
    }

    fn start_listeners(&mut self) {
        if let Some(mut receiver) = self.status_receiver.take() {
            let latest_status = self.latest_status.clone();
            let latest_ssid = self.latest_ssid.clone();
            let latest_password = self.latest_password.clone();

            MainContext::default().spawn_local(async move {
                while let Some(status) = receiver.recv().await {
                    *latest_status.borrow_mut() = Some(status.clone());

                    if let Some(wifi) = status.interfaces.iter().find(|i| i.interface_type == NetworkInterfaceType::Wifi) {
                        if let Some(ssid) = wifi.ssid.as_ref() {
                            *latest_ssid.borrow_mut() = ssid.to_string();
                        }
                        if let Some(pw) = wifi.wifi_password.as_ref() {
                            *latest_password.borrow_mut() = pw.to_string();
                        } else {
                            *latest_password.borrow_mut() = String::new();
                        }
                    }
                }
            });
        }

        if let Some(mut receiver) = self.scan_receiver.take() {
            let latest_scan = self.latest_scan.clone();

            MainContext::default().spawn_local(async move {
                while let Some(scan) = receiver.recv().await {
                    *latest_scan.borrow_mut() = Some(scan);
                }
            });
        }

        if let Some(mut receiver) = self.vpn_receiver.take() {
            let latest_vpn = self.latest_vpn.clone();

            MainContext::default().spawn_local(async move {
                while let Some(vpn) = receiver.recv().await {
                    *latest_vpn.borrow_mut() = Some(vpn);
                }
            });
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<NetworkStatusMessage>> for NetworkWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<NetworkStatusMessage>, _sender_id: &str) {
        if let Err(e) = self.status_sender.send(message.0.clone()) {
            debug!("Network Widget: failed to forward status to UI thread: {e}");
        }
        *self.latest_status.borrow_mut() = Some(message.0.clone());
        self.update_ui(&message.0);
    }
}

impl MessageHandler<FfiEnvelopePayload<ScanResultsMessage>> for NetworkWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<ScanResultsMessage>, _sender_id: &str) {
        if let Err(e) = self.scan_sender.send(message.0) {
            debug!("Network Widget: failed to forward scan results to UI thread: {e}");
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<VpnProfilesMessage>> for NetworkWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<VpnProfilesMessage>, _sender_id: &str) {
        if let Err(e) = self.vpn_sender.send(message.0) {
            debug!("Network Widget: failed to forward VPN profiles to UI thread: {e}");
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for NetworkWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("Network Widget: received personalization status");
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

impl AcceptTopic<FfiEnvelope> for NetworkWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_STATUS || topic == TOPIC_SCAN_RESULTS || topic == TOPIC_VPN_PROFILES || topic == TOPIC_PERSONALIZATION_STATUS
    }
}

impl MessageBroadcaster for NetworkWidget {}

impl PluginMetaGetter for NetworkWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for NetworkWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl WidgetPlugin for NetworkWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                if envelope.type_id == NetworkStatusMessage::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<NetworkStatusMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == ScanResultsMessage::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<ScanResultsMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == VpnProfilesMessage::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<VpnProfilesMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<PersonalizationStatusMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<PersonalizationStatusMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for NetworkWidget {
    fn build_widget(&mut self) -> Widget {
        let config = self.config.clone();
        let broadcaster = self.get_broadcaster();
        let show_labels = !config.icon_config.icon_only();

        let content_box = build_content_box(config.layout.spacing_or_default(), &["menu_button_inner"]);

        // Line 0: Icon
        let icon_image = build_widget_icon(config.icon_config.icon_size(), config.icon_config.icon_color(), |_| {});
        content_box.append(&icon_image);
        *self.icon_image.borrow_mut() = Some(icon_image);

        // Line 1: Main label (value text)
        let value_label = build_main_label(if show_labels { "Loading..." } else { "" }, config.text_colors.main_text_color(), false, None);
        content_box.append(&value_label);
        *self.value_label.borrow_mut() = Some(value_label);

        // Line 2: Info label
        let info_label = build_info_label(if show_labels { "" } else { "" }, config.text_colors.info_text_color(), false, None);
        content_box.append(&info_label);
        *self.info_label.borrow_mut() = Some(info_label);

        // Line 3: Spacer (hidden in QR view) or QR area (square, sized to widget height)
        let qr_size = config.dimensions.height_or_default() - 10;
        let qr_area = DrawingArea::builder()
            .css_classes(["network-qr"])
            .width_request(qr_size)
            .height_request(qr_size)
            .halign(Align::Center)
            .valign(Align::Center)
            .vexpand(false)
            .hexpand(false)
            .visible(false)
            .build();

        let spacer = build_spacer(16);
        content_box.append(&spacer);
        *self.spacer_label.borrow_mut() = Some(spacer);

        let latest_ssid_for_qr = self.latest_ssid.clone();
        let latest_password_for_qr = self.latest_password.clone();
        qr_area.set_draw_func(move |_, cr, w, h| {
            let ssid = latest_ssid_for_qr.borrow().clone();
            if ssid.is_empty() {
                cr.set_source_rgba(0.5, 0.5, 0.5, 1.0);
                cr.select_font_face("Sans", gtk4::cairo::FontSlant::Normal, gtk4::cairo::FontWeight::Normal);
                cr.set_font_size(14.0);
                let text = "No WiFi";
                if let Ok(extents) = cr.text_extents(text) {
                    let tx = (w as f64 - extents.width()) / 2.0 - extents.x_bearing();
                    let ty = (h as f64) / 2.0;
                    let _ = cr.move_to(tx, ty);
                    let _ = cr.show_text(text);
                }
                return;
            }
            let password = latest_password_for_qr.borrow().clone();
            let qr_string = generate_wifi_qr_string(&ssid, &password, "WPA");
            if let Ok(qr_code) = qrcode::QrCode::new(qr_string.as_bytes()) {
                draw_qr_code(cr, w, h, &qr_code);
            }
        });
        content_box.append(&qr_area);
        *self.qr_drawing_area.borrow_mut() = Some(qr_area);

        let button = config.dimensions.build_button(config.mode, &content_box, "max-width-");

        let widget_self = Rc::new(Self {
            meta: self.meta.clone(),
            core_context: self.core_context,
            config: self.config.clone(),
            status_sender: self.status_sender.clone(),
            status_receiver: None,
            scan_sender: self.scan_sender.clone(),
            scan_receiver: None,
            vpn_sender: self.vpn_sender.clone(),
            vpn_receiver: None,
            icon_image: self.icon_image.clone(),
            value_label: self.value_label.clone(),
            info_label: self.info_label.clone(),
            qr_drawing_area: self.qr_drawing_area.clone(),
            spacer_label: self.spacer_label.clone(),
            current_view: self.current_view.clone(),
            latest_status: self.latest_status.clone(),
            latest_scan: self.latest_scan.clone(),
            latest_vpn: self.latest_vpn.clone(),
            latest_ssid: self.latest_ssid.clone(),
            latest_password: self.latest_password.clone(),
            personalization: self.personalization.clone(),
        });

        let button_widget = button.upcast::<Widget>();
        apply_widget_css_classes(&button_widget, &self.meta.id, &self.config.layout.css_classes);
        widget_self.attach_gesture_handlers(&button_widget, &config.actions, &broadcaster, &GestureHandlersConfiguration::default());

        self.start_listeners();

        button_widget
    }
}

pub(crate) fn render_view(
    status: &NetworkStatusMessage,
    _scan: Option<&ScanResultsMessage>,
    _vpn: Option<&VpnProfilesMessage>,
    config: &NetworkWidgetConfig,
    view: NetworkView,
    override_data: &PersonalizationOverride,
) -> ViewData {
    let locale = override_data.effective_locale();
    match view {
        NetworkView::WifiStatus => {
            let wifi = find_interface(status, NetworkInterfaceType::Wifi);
            if let Some(iface) = wifi {
                let ssid = iface
                    .ssid
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| NetworkLabel::Unknown.localized_label(locale).to_string());
                let signal = iface.signal.as_ref().map(|s| *s).unwrap_or(0);
                let ip = iface.ipv4_address.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "No IP".to_string());
                let icon_name = wifi_signal_icon_name(signal, config);
                let color = WifiSignalLevel::from_percent(signal).get_icon_color();
                ViewData::with_color(icon_name, ssid, format!("{signal}%  {ip}"), color)
            } else {
                let color = ConnectionStateLevel::from_state(NetworkConnectionState::Disconnected).get_icon_color();
                ViewData::with_color(
                    config.icon_wifi_strength_off.clone(),
                    NetworkLabel::NoWiFi.localized_label(locale).to_string(),
                    String::new(),
                    color,
                )
            }
        }
        NetworkView::EthernetStatus => {
            let eth = find_interface(status, NetworkInterfaceType::Ethernet);
            if let Some(iface) = eth {
                let ip = iface.ipv4_address.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "No IP".to_string());
                let is_connected = iface.state == NetworkConnectionState::Connected;
                let icon_name = if is_connected {
                    config.icon_ethernet_on.clone()
                } else {
                    config.icon_ethernet_off.clone()
                };
                let state_text = if is_connected {
                    NetworkLabel::Connected.localized_label(locale)
                } else {
                    NetworkLabel::Disconnected.localized_label(locale)
                };
                let color = ConnectionStateLevel::from_state(iface.state).get_icon_color();
                ViewData::with_color(icon_name, state_text.to_string(), format!("{}  {ip}", iface.interface_name), color)
            } else {
                let color = ConnectionStateLevel::from_state(NetworkConnectionState::Disconnected).get_icon_color();
                ViewData::with_color(
                    config.icon_ethernet_off.clone(),
                    NetworkLabel::NoEthernet.localized_label(locale).to_string(),
                    String::new(),
                    color,
                )
            }
        }
        NetworkView::Throughput => {
            let rx = override_data.format_bandwidth(status.received_bytes_per_second);
            let tx = override_data.format_bandwidth(status.transmitted_bytes_per_second);
            ViewData::new(config.icon_throughput.clone(), rx, tx)
        }
        NetworkView::WifiScan => {
            let count = _scan.map(|s| s.access_points.len()).unwrap_or(0);
            let strongest = _scan
                .and_then(|s| s.access_points.iter().max_by_key(|ap| ap.signal))
                .map(|ap| (ap.ssid.to_string(), ap.signal))
                .unwrap_or_else(|| (NetworkLabel::Unknown.localized_label(locale).to_string(), 0u8));
            ViewData::new(
                config.icon_wifi_scan.clone(),
                format!("{count} {}", NetworkLabel::Networks.localized_label(locale)),
                format!("{}  {}%", strongest.0, strongest.1),
            )
        }
        NetworkView::Vpn => {
            let active_profile = _vpn.and_then(|v| v.profiles.iter().find(|p| p.is_active)).map(|p| p.name.to_string());
            let inactive_profile = _vpn.and_then(|v| v.profiles.iter().find(|p| !p.is_active)).map(|p| p.name.to_string());
            if let Some(name) = active_profile {
                let color = ConnectionStateLevel::from_state(NetworkConnectionState::Connected).get_icon_color();
                ViewData::with_color(config.icon_vpn_on.clone(), name, NetworkLabel::Active.localized_label(locale).to_string(), color)
            } else if let Some(name) = inactive_profile {
                let color = ConnectionStateLevel::from_state(NetworkConnectionState::Disconnected).get_icon_color();
                ViewData::with_color(config.icon_vpn_off.clone(), name, NetworkLabel::Inactive.localized_label(locale).to_string(), color)
            } else {
                let color = ConnectionStateLevel::from_state(NetworkConnectionState::Unavailable).get_icon_color();
                ViewData::with_color(config.icon_vpn_off.clone(), NetworkLabel::NoVpn.localized_label(locale).to_string(), String::new(), color)
            }
        }
        NetworkView::Airplane => {
            if status.airplane_mode {
                let color = ConnectionStateLevel::from_state(NetworkConnectionState::Unavailable).get_icon_color();
                ViewData::with_color(
                    config.icon_airplane_on.clone(),
                    "ON".to_string(),
                    NetworkLabel::AirplaneMode.localized_label(locale).to_string(),
                    color,
                )
            } else {
                let color = ConnectionStateLevel::from_state(NetworkConnectionState::Connected).get_icon_color();
                ViewData::with_color(
                    config.icon_airplane_off.clone(),
                    "OFF".to_string(),
                    NetworkLabel::AirplaneMode.localized_label(locale).to_string(),
                    color,
                )
            }
        }
        NetworkView::QrCode => ViewData::new(config.icon_qr_code.clone(), NetworkLabel::QrCode.localized_label(locale).to_string(), String::new()),
    }
}

fn wifi_signal_icon_name(signal: u8, config: &NetworkWidgetConfig) -> String {
    if signal > 75 {
        config.icon_wifi_strength_4.clone()
    } else if signal > 50 {
        config.icon_wifi_strength_3.clone()
    } else if signal > 25 {
        config.icon_wifi_strength_2.clone()
    } else if signal > 0 {
        config.icon_wifi_strength_1.clone()
    } else {
        config.icon_wifi_strength_off.clone()
    }
}

fn find_interface<'a>(status: &'a NetworkStatusMessage, iface_type: NetworkInterfaceType) -> Option<&'a smearor_network_model::InterfaceStatus> {
    status.interfaces.iter().find(|iface| iface.interface_type == iface_type)
}

fn show_qr_view(qr_area: &SharedDrawingArea, icon_image: &SharedImage, value_label: &SharedLabel, info_label: &SharedLabel, spacer_label: &SharedLabel) {
    if let Some(ref area) = *qr_area.borrow() {
        area.set_visible(true);
        area.queue_draw();
    }
    if let Some(ref img) = *icon_image.borrow() {
        img.set_visible(false);
    }
    if let Some(ref label) = *value_label.borrow() {
        label.set_visible(false);
    }
    if let Some(ref label) = *info_label.borrow() {
        label.set_visible(false);
    }
    if let Some(ref label) = *spacer_label.borrow() {
        label.set_visible(false);
    }
}

fn show_normal_view(qr_area: &SharedDrawingArea, icon_image: &SharedImage, value_label: &SharedLabel, info_label: &SharedLabel, spacer_label: &SharedLabel) {
    if let Some(ref area) = *qr_area.borrow() {
        area.set_visible(false);
    }
    if let Some(ref img) = *icon_image.borrow() {
        img.set_visible(true);
    }
    if let Some(ref label) = *value_label.borrow() {
        label.set_visible(true);
    }
    if let Some(ref label) = *info_label.borrow() {
        label.set_visible(true);
    }
    if let Some(ref label) = *spacer_label.borrow() {
        label.set_visible(true);
    }
}

fn set_icon_image(img: &Image, icon_name: &str, icon_size: i32) {
    if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(icon_name) {
        img.set_icon_name(Some(&gtk_icon_name));
    }
    img.set_pixel_size(icon_size);
}

fn update_icon_display(img: &Image, view_data: &ViewData, icon_size: i32, configured_color: Option<smearor_swipe_launcher_plugin_api::Color>) {
    set_icon_image(img, &view_data.icon_name, icon_size);
    if let Some(c) = configured_color {
        apply_icon_color(img, c);
    }
}

impl DefaultFallback for NetworkWidget {
    fn default_fallback(&self, kind: &ActionKind, broadcaster: &MessageBroadcasterInner) {
        match kind {
            ActionKind::Click | ActionKind::DoublePress => {
                self.next_view();
            }
            ActionKind::Longpress | ActionKind::RightClick => {
                broadcaster.broadcast_message_to_topic(NetworkCommandMessage::refresh());
            }
            ActionKind::SwipeUp | ActionKind::ScrollUp | ActionKind::MiddleClick => {
                self.next_view();
            }
            ActionKind::SwipeDown | ActionKind::ScrollDown => {
                self.prev_view();
            }
            ActionKind::Hold | ActionKind::CompoundLongpress | ActionKind::Init | ActionKind::Expand | ActionKind::Collapse | ActionKind::ToggleView => {}
        }
    }
}

fn escape_wifi_qr_field(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\\' | ';' | ',' | ':' | '"' => format!("\\{c}"),
            _ => c.to_string(),
        })
        .collect()
}

fn generate_wifi_qr_string(ssid: &str, password: &str, security: &str) -> String {
    let escaped_ssid = escape_wifi_qr_field(ssid);
    let escaped_password = escape_wifi_qr_field(password);
    format!("WIFI:T:{security};S:{escaped_ssid};P:{escaped_password};;")
}

fn draw_qr_code(cr: &gtk4::cairo::Context, width: i32, height: i32, qr_data: &qrcode::QrCode) {
    let modules = qr_data.width() as i32;
    let quiet_zone = 2;
    let total_modules = modules + 2 * quiet_zone;
    let size = width.min(height);
    let cell_size = size as f64 / total_modules as f64;
    let offset_x = (width as f64 - cell_size * total_modules as f64) / 2.0;
    let offset_y = (height as f64 - cell_size * total_modules as f64) / 2.0;

    cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
    cr.rectangle(offset_x, offset_y, cell_size * total_modules as f64, cell_size * total_modules as f64);
    let _ = cr.fill();

    cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
    for y in 0..modules {
        for x in 0..modules {
            if qr_data[(x as usize, y as usize)] == qrcode::Color::Dark {
                let px = offset_x + (x + quiet_zone) as f64 * cell_size;
                let py = offset_y + (y + quiet_zone) as f64 * cell_size;
                cr.rectangle(px, py, cell_size, cell_size);
                let _ = cr.fill();
            }
        }
    }
}
