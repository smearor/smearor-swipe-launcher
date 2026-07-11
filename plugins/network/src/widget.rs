use crate::config::NetworkWidgetConfig;
use gtk4::Align;
use gtk4::Box as GtkBox;
use gtk4::DrawingArea;
use gtk4::EventSequenceState;
use gtk4::GestureClick;
use gtk4::GestureDrag;
use gtk4::GestureLongPress;
use gtk4::Image;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::PropagationPhase;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::prelude::BoxExt;
use gtk4::prelude::WidgetExt;
use gtk4::prelude::*;
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
use smearor_swipe_launcher_plugin_api::AcceptTopic;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::Plugin;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use smearor_swipe_launcher_plugin_api::resolve_gtk_nerd_icon;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::debug;

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
    pub current_view: Rc<RefCell<usize>>,
    pub latest_status: Rc<RefCell<Option<NetworkStatusMessage>>>,
    pub latest_scan: Rc<RefCell<Option<ScanResultsMessage>>>,
    pub latest_vpn: Rc<RefCell<Option<VpnProfilesMessage>>>,
    pub latest_ssid: SharedString,
    pub latest_password: SharedString,
}

impl NetworkWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: NetworkWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let (status_sender, status_receiver) = tokio::sync::mpsc::unbounded_channel::<NetworkStatusMessage>();
        let (scan_sender, scan_receiver) = tokio::sync::mpsc::unbounded_channel::<ScanResultsMessage>();
        let (vpn_sender, vpn_receiver) = tokio::sync::mpsc::unbounded_channel::<VpnProfilesMessage>();

        Ok(NetworkWidget {
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
            current_view: Rc::new(RefCell::new(0)),
            latest_status: Rc::new(RefCell::new(None)),
            latest_scan: Rc::new(RefCell::new(None)),
            latest_vpn: Rc::new(RefCell::new(None)),
            latest_ssid: Rc::new(RefCell::new(String::new())),
            latest_password: Rc::new(RefCell::new(String::new())),
        })
    }

    fn update_ui(&self, status: &NetworkStatusMessage) {
        let view_index = *self.current_view.borrow();
        let view = self.config.views.get(view_index).copied().unwrap_or(NetworkView::WifiStatus);

        let icon_image = self.icon_image.clone();
        let value_label = self.value_label.clone();
        let info_label = self.info_label.clone();
        let qr_area = self.qr_drawing_area.clone();
        let config = self.config.clone();
        let status = status.clone();
        let latest_scan = self.latest_scan.clone();
        let latest_vpn = self.latest_vpn.clone();

        MainContext::default().spawn_local(async move {
            if view == NetworkView::QrCode {
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
            } else {
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

                let scan = latest_scan.borrow().clone();
                let vpn = latest_vpn.borrow().clone();
                let (icon_name, value_text, info_text) = render_view(&status, scan.as_ref(), vpn.as_ref(), &config, view);

                if let Some(ref img) = *icon_image.borrow() {
                    set_icon_image(img, &icon_name, config.icon_size);
                }
                if let Some(ref label) = *value_label.borrow() {
                    label.set_text(&value_text);
                }
                if let Some(ref label) = *info_label.borrow() {
                    label.set_text(&info_text);
                }
            }
        });
    }

    fn next_view(&self) {
        let current_view = self.current_view.clone();
        let latest_status = self.latest_status.clone();
        let latest_scan = self.latest_scan.clone();
        let latest_vpn = self.latest_vpn.clone();
        let icon_image = self.icon_image.clone();
        let value_label = self.value_label.clone();
        let info_label = self.info_label.clone();
        let qr_area = self.qr_drawing_area.clone();
        let config = self.config.clone();

        MainContext::default().spawn_local(async move {
            if config.views.is_empty() {
                return;
            }
            let mut idx = current_view.borrow_mut();
            *idx = (*idx + 1) % config.views.len();
            let view = config.views[*idx];
            drop(idx);

            if let Some(ref status) = *latest_status.borrow() {
                if view == NetworkView::QrCode {
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
                } else {
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

                    let scan = latest_scan.borrow().clone();
                    let vpn = latest_vpn.borrow().clone();
                    let (icon_name, value_text, info_text) = render_view(status, scan.as_ref(), vpn.as_ref(), &config, view);
                    if let Some(ref img) = *icon_image.borrow() {
                        set_icon_image(img, &icon_name, config.icon_size);
                    }
                    if let Some(ref label) = *value_label.borrow() {
                        label.set_text(&value_text);
                    }
                    if let Some(ref label) = *info_label.borrow() {
                        label.set_text(&info_text);
                    }
                }
            }
        });
    }

    fn prev_view(&self) {
        let current_view = self.current_view.clone();
        let latest_status = self.latest_status.clone();
        let latest_scan = self.latest_scan.clone();
        let latest_vpn = self.latest_vpn.clone();
        let icon_image = self.icon_image.clone();
        let value_label = self.value_label.clone();
        let info_label = self.info_label.clone();
        let qr_area = self.qr_drawing_area.clone();
        let config = self.config.clone();

        MainContext::default().spawn_local(async move {
            if config.views.is_empty() {
                return;
            }
            let mut idx = current_view.borrow_mut();
            if *idx == 0 {
                *idx = config.views.len() - 1;
            } else {
                *idx -= 1;
            }
            let view = config.views[*idx];
            drop(idx);

            if let Some(ref status) = *latest_status.borrow() {
                if view == NetworkView::QrCode {
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
                } else {
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

                    let scan = latest_scan.borrow().clone();
                    let vpn = latest_vpn.borrow().clone();
                    let (icon_name, value_text, info_text) = render_view(status, scan.as_ref(), vpn.as_ref(), &config, view);
                    if let Some(ref img) = *icon_image.borrow() {
                        set_icon_image(img, &icon_name, config.icon_size);
                    }
                    if let Some(ref label) = *value_label.borrow() {
                        label.set_text(&value_text);
                    }
                    if let Some(ref label) = *info_label.borrow() {
                        label.set_text(&info_text);
                    }
                }
            }
        });
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

impl AcceptTopic<FfiEnvelope> for NetworkWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_STATUS || topic == TOPIC_SCAN_RESULTS || topic == TOPIC_VPN_PROFILES
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

impl Plugin for NetworkWidget {
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
                }
            }
        }
    }
}

impl WidgetBuilder for NetworkWidget {
    fn build_widget(&mut self) -> Widget {
        let config = self.config.clone();
        let broadcaster = self.get_broadcaster();

        let outer_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(config.spacing)
            .css_classes(["network-widget".to_string()])
            .halign(Align::Center)
            .valign(Align::Center)
            .build();

        outer_box.set_width_request(config.width);
        outer_box.set_height_request(config.height);

        let icon_image = Image::builder()
            .css_classes(["network-icon".to_string()])
            .halign(Align::Center)
            .pixel_size(config.icon_size)
            .build();

        let value_label = Label::builder().css_classes(["network-value".to_string()]).halign(Align::Center).build();
        value_label.set_text("Loading...");

        let info_label = Label::builder().css_classes(["network-info".to_string()]).halign(Align::Center).build();
        info_label.set_text("");

        let qr_area = DrawingArea::builder()
            .css_classes(["network-qr".to_string()])
            .width_request(128)
            .height_request(128)
            .halign(Align::Center)
            .valign(Align::Center)
            .visible(false)
            .build();

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

        outer_box.append(&icon_image);
        outer_box.append(&value_label);
        outer_box.append(&info_label);
        outer_box.append(&qr_area);

        *self.icon_image.borrow_mut() = Some(icon_image);
        *self.value_label.borrow_mut() = Some(value_label);
        *self.info_label.borrow_mut() = Some(info_label);
        *self.qr_drawing_area.borrow_mut() = Some(qr_area);

        let click_topic = config.click_topic.clone();
        let click_payload = config.click_payload.clone();
        let click_instance = config.click_instance.clone();
        let longpress_topic = config.longpress_topic.clone();
        let longpress_payload = config.longpress_payload.clone();
        let longpress_instance = config.longpress_instance.clone();
        let message_broadcaster = broadcaster.clone();

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
            current_view: self.current_view.clone(),
            latest_status: self.latest_status.clone(),
            latest_scan: self.latest_scan.clone(),
            latest_vpn: self.latest_vpn.clone(),
            latest_ssid: self.latest_ssid.clone(),
            latest_password: self.latest_password.clone(),
        });

        let click_gesture = GestureClick::builder().button(0).propagation_phase(PropagationPhase::Capture).build();
        let broadcaster_for_click = message_broadcaster.clone();
        let widget_for_click = widget_self.clone();
        click_gesture.connect_released(move |gesture, _n_press, _x, _y| {
            if let Some(seq) = gesture.current_sequence() {
                let state = gesture.sequence_state(&seq);
                if state == EventSequenceState::Claimed || state == EventSequenceState::Denied {
                    return;
                }
            }
            if let (Some(topic), Some(payload)) = (click_topic.clone(), click_payload.clone()) {
                let payload_str = payload.to_string();
                if let Some(instance) = click_instance.clone() {
                    broadcaster_for_click.broadcast_string_to_instance(&instance, &topic, &payload_str);
                } else {
                    broadcaster_for_click.broadcast_string(&topic, &payload_str);
                }
            }
            widget_for_click.handle_click();
            gesture.set_state(EventSequenceState::Claimed);
        });
        outer_box.add_controller(click_gesture);

        let drag_gesture = GestureDrag::new();
        drag_gesture.set_propagation_phase(PropagationPhase::Capture);
        let widget_for_drag = widget_self.clone();
        drag_gesture.connect_drag_end(move |gesture, offset_x, offset_y| {
            const SWIPE_THRESHOLD: f64 = 50.0;
            if offset_y.abs() > offset_x.abs() && offset_y.abs() > SWIPE_THRESHOLD {
                gesture.set_state(EventSequenceState::Claimed);
                if offset_y < 0.0 {
                    widget_for_drag.next_view();
                } else {
                    widget_for_drag.prev_view();
                }
            }
        });
        outer_box.add_controller(drag_gesture);

        let longpress_gesture = GestureLongPress::builder().button(0).propagation_phase(PropagationPhase::Capture).build();
        let broadcaster_for_longpress = message_broadcaster.clone();
        longpress_gesture.connect_pressed(move |gesture, _x, _y| {
            if let (Some(topic), Some(payload)) = (longpress_topic.clone(), longpress_payload.clone()) {
                let payload_str = payload.to_string();
                if let Some(instance) = longpress_instance.clone() {
                    broadcaster_for_longpress.broadcast_string_to_instance(&instance, &topic, &payload_str);
                } else {
                    broadcaster_for_longpress.broadcast_string(&topic, &payload_str);
                }
            }
            let command = NetworkCommandMessage::refresh();
            broadcaster_for_longpress.broadcast_message_to_topic(command);
            gesture.set_state(EventSequenceState::Claimed);
        });
        outer_box.add_controller(longpress_gesture);

        self.start_listeners();

        outer_box.upcast::<Widget>()
    }
}

fn render_view(
    status: &NetworkStatusMessage,
    _scan: Option<&ScanResultsMessage>,
    _vpn: Option<&VpnProfilesMessage>,
    config: &NetworkWidgetConfig,
    view: NetworkView,
) -> (String, String, String) {
    match view {
        NetworkView::WifiStatus => {
            let wifi = find_interface(status, NetworkInterfaceType::Wifi);
            if let Some(iface) = wifi {
                let ssid = iface.ssid.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "Unknown".to_string());
                let signal = iface.signal.as_ref().map(|s| *s).unwrap_or(0);
                let ip = iface.ipv4_address.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "No IP".to_string());
                let icon_name = wifi_signal_icon_name(signal, config);
                (icon_name, ssid, format!("{signal}%  {ip}"))
            } else {
                (config.icon_wifi_strength_off.clone(), "WiFi Off".to_string(), String::new())
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
                let state_text = if is_connected { "Connected" } else { "Disconnected" };
                (icon_name, state_text.to_string(), format!("{}  {ip}", iface.interface_name))
            } else {
                (config.icon_ethernet_off.clone(), "No Ethernet".to_string(), String::new())
            }
        }
        NetworkView::Throughput => {
            let rx = format_bytes(status.received_bytes_per_second);
            let tx = format_bytes(status.transmitted_bytes_per_second);
            (config.icon_throughput.clone(), format!("{rx}/s"), format!("{tx}/s"))
        }
        NetworkView::WifiScan => {
            let count = _scan.map(|s| s.access_points.len()).unwrap_or(0);
            let strongest = _scan
                .and_then(|s| s.access_points.iter().max_by_key(|ap| ap.signal))
                .map(|ap| (ap.ssid.to_string(), ap.signal))
                .unwrap_or_else(|| ("Unknown".to_string(), 0u8));
            (config.icon_wifi_scan.clone(), format!("{count} networks"), format!("{}  {}%", strongest.0, strongest.1))
        }
        NetworkView::Vpn => {
            let active_profile = _vpn.and_then(|v| v.profiles.iter().find(|p| p.is_active)).map(|p| p.name.to_string());
            let inactive_profile = _vpn.and_then(|v| v.profiles.iter().find(|p| !p.is_active)).map(|p| p.name.to_string());
            if let Some(name) = active_profile {
                (config.icon_vpn_on.clone(), name, "Active".to_string())
            } else if let Some(name) = inactive_profile {
                (config.icon_vpn_off.clone(), name, "Inactive".to_string())
            } else {
                (config.icon_vpn_off.clone(), "No VPN".to_string(), "Not configured".to_string())
            }
        }
        NetworkView::Airplane => {
            if status.airplane_mode {
                (config.icon_airplane_on.clone(), "ON".to_string(), "Airplane Mode".to_string())
            } else {
                (config.icon_airplane_off.clone(), "OFF".to_string(), "Airplane Mode".to_string())
            }
        }
        NetworkView::QrCode => (config.icon_qr_code.clone(), "QR Code".to_string(), String::new()),
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

fn set_icon_image(img: &Image, icon_name: &str, icon_size: i32) {
    if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(icon_name) {
        let resource_path = format!("/com/nerd/icons/{}.svg", gtk_icon_name);
        if gtk4::gio::resources_lookup_data(&resource_path, gtk4::gio::ResourceLookupFlags::NONE).is_ok() {
            img.set_resource(Some(&resource_path));
        } else {
            debug!("Network Widget: GResource not found for icon '{}': {}", icon_name, resource_path);
        }
        img.set_pixel_size(icon_size);
    }
}

impl NetworkWidget {
    fn handle_click(&self) {
        let view_index = *self.current_view.borrow();
        let view = self.config.views.get(view_index).copied().unwrap_or(NetworkView::WifiStatus);
        let broadcaster = self.get_broadcaster();
        debug!("Network Widget: handle_click view={:?}", view);

        match view {
            NetworkView::WifiStatus => {
                let status_ref = self.latest_status.borrow();
                let wifi = status_ref.as_ref().and_then(|s| find_interface(s, NetworkInterfaceType::Wifi));
                let is_connected = wifi.map(|i| i.state == NetworkConnectionState::Connected).unwrap_or(false);
                let is_unavailable = wifi.map(|i| i.state == NetworkConnectionState::Unavailable).unwrap_or(false);
                debug!("Network Widget: WiFi is_connected={} is_unavailable={}", is_connected, is_unavailable);
                if is_connected && let Some(iface) = wifi {
                    let interface_name = iface.interface_name.to_string();
                    debug!("Network Widget: broadcasting disconnect for WiFi interface={}", interface_name);
                    let command = NetworkCommandMessage::disconnect(&interface_name);
                    broadcaster.broadcast_message_to_topic(command);
                } else if is_unavailable {
                    debug!("Network Widget: WiFi unavailable, broadcasting connect (service will enable radio first)");
                    let interface_name = wifi.map(|i| i.interface_name.to_string()).unwrap_or_default();
                    if !interface_name.is_empty() {
                        let command = NetworkCommandMessage::connect(&interface_name);
                        broadcaster.broadcast_message_to_topic(command);
                    }
                } else if let Some(iface) = wifi {
                    let interface_name = iface.interface_name.to_string();
                    if let Some(ssid) = iface.ssid.as_ref() {
                        debug!("Network Widget: broadcasting connect_wifi ssid={}", ssid);
                        let command = NetworkCommandMessage::connect_wifi(&ssid.to_string(), None);
                        broadcaster.broadcast_message_to_topic(command);
                    } else {
                        debug!("Network Widget: broadcasting connect for WiFi interface={} (no SSID, using saved profile)", interface_name);
                        let command = NetworkCommandMessage::connect(&interface_name);
                        broadcaster.broadcast_message_to_topic(command);
                    }
                }
            }
            NetworkView::EthernetStatus => {
                let status_ref = self.latest_status.borrow();
                let eth = status_ref.as_ref().and_then(|s| find_interface(s, NetworkInterfaceType::Ethernet));
                if let Some(iface) = eth {
                    let is_connected = iface.state == NetworkConnectionState::Connected;
                    let interface_name = iface.interface_name.to_string();
                    debug!("Network Widget: Ethernet interface={} is_connected={}", interface_name, is_connected);
                    if is_connected {
                        debug!("Network Widget: broadcasting disconnect for Ethernet interface={}", interface_name);
                        let command = NetworkCommandMessage::disconnect(&interface_name);
                        broadcaster.broadcast_message_to_topic(command);
                    } else {
                        debug!("Network Widget: broadcasting connect for Ethernet interface={}", interface_name);
                        let command = NetworkCommandMessage::connect(&interface_name);
                        broadcaster.broadcast_message_to_topic(command);
                    }
                }
            }
            NetworkView::Airplane => {
                let is_on = self.latest_status.borrow().as_ref().map(|s| s.airplane_mode).unwrap_or(false);
                debug!("Network Widget: Airplane current_mode={}, toggling to {}", is_on, !is_on);
                let command = NetworkCommandMessage::toggle_radio("all", is_on);
                broadcaster.broadcast_message_to_topic(command);
            }
            NetworkView::Vpn => {
                if let Some(ref vpn) = *self.latest_vpn.borrow() {
                    if let Some(profile) = vpn.profiles.iter().find(|p| p.is_active) {
                        debug!("Network Widget: VPN deactivating profile={} uuid={}", profile.name, profile.uuid);
                        let command = NetworkCommandMessage::toggle_vpn(&profile.uuid, false);
                        broadcaster.broadcast_message_to_topic(command);
                    } else if let Some(profile) = vpn.profiles.first() {
                        debug!("Network Widget: VPN activating profile={} uuid={}", profile.name, profile.uuid);
                        let command = NetworkCommandMessage::toggle_vpn(&profile.uuid, true);
                        broadcaster.broadcast_message_to_topic(command);
                    } else {
                        debug!("Network Widget: VPN no profiles available");
                    }
                } else {
                    debug!("Network Widget: VPN no profile data received yet");
                }
            }
            NetworkView::WifiScan => {
                let wifi_available = self
                    .latest_status
                    .borrow()
                    .as_ref()
                    .and_then(|s| find_interface(s, NetworkInterfaceType::Wifi))
                    .map(|i| i.state != NetworkConnectionState::Unavailable)
                    .unwrap_or(false);
                if wifi_available {
                    debug!("Network Widget: broadcasting scan_wifi");
                    let command = NetworkCommandMessage::scan_wifi();
                    broadcaster.broadcast_message_to_topic(command);
                } else {
                    debug!("Network Widget: skipping scan_wifi, WiFi unavailable");
                }
            }
            NetworkView::Throughput | NetworkView::QrCode => {}
        }
    }
}

fn format_bytes(bytes_per_second: u64) -> String {
    if bytes_per_second >= 1_073_741_824 {
        format!("{:.1} GB", bytes_per_second as f64 / 1_073_741_824.0)
    } else if bytes_per_second >= 1_048_576 {
        format!("{:.1} MB", bytes_per_second as f64 / 1_048_576.0)
    } else if bytes_per_second >= 1024 {
        format!("{:.1} KB", bytes_per_second as f64 / 1024.0)
    } else {
        format!("{} B", bytes_per_second)
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
