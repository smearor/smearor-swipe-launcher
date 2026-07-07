use crate::config::NetworkWidgetConfig;
use gtk4::Align;
use gtk4::Box as GtkBox;
use gtk4::Button;
use gtk4::DrawingArea;
use gtk4::Entry;
use gtk4::GestureClick;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Overlay;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::prelude::BoxExt;
use gtk4::prelude::WidgetExt;
use gtk4::prelude::*;
use smearor_network_model::NetworkCommandMessage;
use smearor_network_model::NetworkInterfaceType;
use smearor_network_model::NetworkStatusMessage;
use smearor_network_model::ScanResultsMessage;
use smearor_network_model::TOPIC_SCAN_RESULTS;
use smearor_network_model::TOPIC_STATUS;
use smearor_network_model::TOPIC_VPN_PROFILES;
use smearor_network_model::VpnProfilesMessage;
use smearor_network_model::network_interface_icon_unicode;
use smearor_network_model::wifi_security_icon_unicode;
use smearor_network_model::wifi_signal_icon_unicode;
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
use std::cell::RefCell;
use std::rc::Rc;
use tracing::debug;

type SharedLabel = Rc<RefCell<Option<Label>>>;
type SharedBox = Rc<RefCell<Option<GtkBox>>>;
type SharedDrawingArea = Rc<RefCell<Option<DrawingArea>>>;
type SharedOverlay = Rc<RefCell<Option<Overlay>>>;
type ThroughputHistory = Rc<RefCell<Vec<f64>>>;

const THROUGHPUT_HISTORY_MAX: usize = 60;

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
    pub status_label: SharedLabel,
    pub throughput_label: SharedLabel,
    pub scan_box: SharedBox,
    pub vpn_box: SharedBox,
    pub airplane_button: SharedLabel,
    pub sparkline_area: SharedDrawingArea,
    pub sparkline_history: ThroughputHistory,
    pub qr_overlay: SharedOverlay,
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
            status_label: Rc::new(RefCell::new(None)),
            throughput_label: Rc::new(RefCell::new(None)),
            scan_box: Rc::new(RefCell::new(None)),
            vpn_box: Rc::new(RefCell::new(None)),
            airplane_button: Rc::new(RefCell::new(None)),
            sparkline_area: Rc::new(RefCell::new(None)),
            sparkline_history: Rc::new(RefCell::new(Vec::new())),
            qr_overlay: Rc::new(RefCell::new(None)),
        })
    }

    fn start_listeners(&mut self) {
        if let Some(mut receiver) = self.status_receiver.take() {
            let status_label = self.status_label.clone();
            let throughput_label = self.throughput_label.clone();
            let airplane_button = self.airplane_button.clone();
            let sparkline_area = self.sparkline_area.clone();
            let sparkline_history = self.sparkline_history.clone();
            let show_status = self.config.show_status;
            let show_throughput = self.config.show_throughput;
            let show_airplane = self.config.show_airplane_toggle;
            let show_sparkline = self.config.show_throughput;

            MainContext::default().spawn_local(async move {
                while let Some(status) = receiver.recv().await {
                    if show_status && let Some(ref label) = *status_label.borrow() {
                        update_status_label(label, &status);
                    }
                    if show_throughput && let Some(ref label) = *throughput_label.borrow() {
                        update_throughput_label(label, &status);
                    }
                    if show_sparkline {
                        let total_bps = (status.received_bytes_per_second + status.transmitted_bytes_per_second) as f64;
                        {
                            let mut history = sparkline_history.borrow_mut();
                            history.push(total_bps);
                            if history.len() > THROUGHPUT_HISTORY_MAX {
                                history.remove(0);
                            }
                        }
                        if let Some(ref area) = *sparkline_area.borrow() {
                            area.queue_draw();
                        }
                    }
                    if show_airplane && let Some(ref label) = *airplane_button.borrow() {
                        update_airplane_button(label, &status);
                    }
                }
            });
        }

        if let Some(mut receiver) = self.scan_receiver.take() {
            let scan_box = self.scan_box.clone();
            let max_aps = self.config.max_access_points;
            let scan_broadcaster = self.get_broadcaster();

            MainContext::default().spawn_local(async move {
                while let Some(scan) = receiver.recv().await {
                    if let Some(ref box_widget) = *scan_box.borrow() {
                        update_scan_list(box_widget, &scan, max_aps, &scan_broadcaster);
                    }
                }
            });
        }

        if let Some(mut receiver) = self.vpn_receiver.take() {
            let vpn_box = self.vpn_box.clone();

            MainContext::default().spawn_local(async move {
                while let Some(vpn) = receiver.recv().await {
                    if let Some(ref box_widget) = *vpn_box.borrow() {
                        update_vpn_list(box_widget, &vpn);
                    }
                }
            });
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<NetworkStatusMessage>> for NetworkWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<NetworkStatusMessage>, _sender_id: &str) {
        if let Err(e) = self.status_sender.send(message.0) {
            debug!("Network Widget: failed to forward status to UI thread: {e}");
        }
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

        let main_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(config.spacing)
            .css_classes(["network-widget".to_string()])
            .halign(Align::Center)
            .valign(Align::Center)
            .build();

        main_box.set_width_request(config.width);
        main_box.set_height_request(config.height);

        if config.show_status {
            let label = Label::builder().css_classes(["network-status".to_string()]).halign(Align::Start).build();
            label.set_text("\u{f0928} Loading...");
            main_box.append(&label);
            *self.status_label.borrow_mut() = Some(label);
        }

        if config.show_throughput {
            let label = Label::builder().css_classes(["network-throughput".to_string()]).halign(Align::Start).build();
            label.set_text("\u{f01b7} 0 B/s  \u{f01b1} 0 B/s");
            main_box.append(&label);
            *self.throughput_label.borrow_mut() = Some(label);

            let sparkline = DrawingArea::builder()
                .css_classes(["network-sparkline".to_string()])
                .height_request(32)
                .halign(Align::Fill)
                .build();
            let history = self.sparkline_history.clone();
            sparkline.set_draw_func(move |_, cr, width, height| {
                draw_sparkline(cr, width, height, &history.borrow());
            });
            main_box.append(&sparkline);
            *self.sparkline_area.borrow_mut() = Some(sparkline);
        }

        if config.show_airplane_toggle {
            let airplane_box = GtkBox::builder()
                .orientation(Orientation::Horizontal)
                .spacing(config.spacing)
                .halign(Align::Center)
                .build();

            let airplane_label = Label::builder().css_classes(["network-airplane".to_string()]).halign(Align::Center).build();
            airplane_label.set_text("\u{f001d} Airplane Mode");
            airplane_box.append(&airplane_label);

            let airplane_btn = Button::builder().label("Off").css_classes(["network-airplane-button".to_string()]).build();
            airplane_box.append(&airplane_btn);

            let airplane_status_label = Rc::new(RefCell::new(None::<Label>));
            *airplane_status_label.borrow_mut() = Some(airplane_label);
            *self.airplane_button.borrow_mut() = airplane_status_label.borrow().as_ref().cloned();

            let broadcaster_clone = broadcaster.clone();
            airplane_btn.connect_clicked(move |_| {
                let command = NetworkCommandMessage::toggle_radio("all", true);
                broadcaster_clone.broadcast_message_to_topic(command);
            });

            main_box.append(&airplane_box);
        }

        if config.show_qr_code {
            let qr_btn = Button::builder()
                .label("\u{f0347} Share WiFi QR")
                .css_classes(["network-qr-button".to_string()])
                .halign(Align::Center)
                .build();

            let qr_overlay = self.qr_overlay.clone();
            let status_label_qr = self.status_label.clone();
            let broadcaster_qr = broadcaster.clone();
            qr_btn.connect_clicked(move |_| {
                show_qr_overlay(&qr_overlay, &status_label_qr, &broadcaster_qr);
            });

            main_box.append(&qr_btn);
        }

        if config.show_scan_list {
            let scan_header = Label::builder().css_classes(["network-scan-header".to_string()]).halign(Align::Start).build();
            scan_header.set_text("\u{f0928} Available Networks");
            main_box.append(&scan_header);

            let scan_box = GtkBox::builder()
                .orientation(Orientation::Vertical)
                .spacing(4)
                .css_classes(["network-scan-list".to_string()])
                .halign(Align::Fill)
                .build();

            let scan_box_clone = scan_box.clone();
            let scan_box_for_click = scan_box.clone();
            *self.scan_box.borrow_mut() = Some(scan_box);
            main_box.append(&scan_box_clone);

            let _ = scan_box_for_click;
        }

        if config.show_vpn_toggles {
            let vpn_header = Label::builder().css_classes(["network-vpn-header".to_string()]).halign(Align::Start).build();
            vpn_header.set_text("\u{f099d} VPN Profiles");
            main_box.append(&vpn_header);

            let vpn_box = GtkBox::builder()
                .orientation(Orientation::Vertical)
                .spacing(4)
                .css_classes(["network-vpn-list".to_string()])
                .halign(Align::Fill)
                .build();
            main_box.append(&vpn_box);
            *self.vpn_box.borrow_mut() = Some(vpn_box);
        }

        let overlay = Overlay::new();
        overlay.set_child(Some(&main_box));

        let click_topic = config.click_topic.clone();
        let click_payload = config.click_payload.clone();
        let click_broadcaster = broadcaster.clone();
        let click_gesture = GestureClick::new();
        click_gesture.connect_released(move |_gesture, _n_press, _, _| {
            if let (Some(topic), Some(payload)) = (click_topic.clone(), click_payload.clone()) {
                let payload_str = payload.to_string();
                click_broadcaster.broadcast_string(&topic, &payload_str);
            }
        });
        overlay.add_controller(click_gesture);

        *self.qr_overlay.borrow_mut() = Some(overlay.clone());

        self.start_listeners();

        overlay.upcast::<Widget>()
    }
}

fn update_status_label(label: &Label, status: &NetworkStatusMessage) {
    let primary = &status.primary_interface;
    let icon = network_interface_icon_unicode(&primary.interface_type);
    let interface_name = primary.interface_name.to_string();

    let text = match primary.interface_type {
        NetworkInterfaceType::Wifi => {
            let ssid = primary.ssid.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "Unknown".to_string());
            let signal = primary.signal.as_ref().map(|s| *s).unwrap_or(0);
            let signal_icon = wifi_signal_icon_unicode(signal);
            let ip = primary.ipv4_address.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "No IP".to_string());
            format!("{icon} {ssid} {signal_icon} {signal}%  {interface_name}  {ip}")
        }
        NetworkInterfaceType::Ethernet => {
            let ip = primary.ipv4_address.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "No IP".to_string());
            format!("{icon} Ethernet  {interface_name}  {ip}")
        }
        NetworkInterfaceType::Vpn => {
            let ip = primary.ipv4_address.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "No IP".to_string());
            format!("{icon} VPN  {interface_name}  {ip}")
        }
        _ => {
            format!("{icon} {interface_name}")
        }
    };
    label.set_text(&text);
}

fn update_throughput_label(label: &Label, status: &NetworkStatusMessage) {
    let rx = format_bytes(status.received_bytes_per_second);
    let tx = format_bytes(status.transmitted_bytes_per_second);
    label.set_text(&format!("\u{f01b7} {rx}/s  \u{f01b1} {tx}/s"));
}

fn update_airplane_button(label: &Label, status: &NetworkStatusMessage) {
    if status.airplane_mode {
        label.set_text("\u{f001d} Airplane Mode ON");
    } else {
        label.set_text("\u{f001d} Airplane Mode");
    }
}

fn update_scan_list(box_widget: &GtkBox, scan: &ScanResultsMessage, max_aps: usize, broadcaster: &smearor_swipe_launcher_plugin_api::MessageBroadcasterInner) {
    while box_widget.first_child().is_some() {
        if let Some(child) = box_widget.first_child() {
            box_widget.remove(&child);
        }
    }

    let count = scan.access_points.len().min(max_aps);
    for ap in scan.access_points.iter().take(count) {
        let ssid = ap.ssid.to_string();
        let signal = ap.signal;
        let signal_icon = wifi_signal_icon_unicode(signal);
        let security_icon = wifi_security_icon_unicode(&ap.security);
        let connected_marker = if ap.is_connected { " \u{f00c}" } else { "" };
        let is_known = ap.is_known;

        let row = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .css_classes(["network-scan-row".to_string()])
            .halign(Align::Fill)
            .build();

        let label = Label::builder()
            .label(format!("{signal_icon} {ssid} {security_icon}{connected_marker}"))
            .css_classes(["network-scan-item".to_string()])
            .halign(Align::Start)
            .build();
        row.append(&label);

        let click_gesture = GestureClick::new();
        let ssid_click = ssid.clone();
        let is_known_click = is_known;
        let broadcaster_click = broadcaster.clone();
        click_gesture.connect_released(move |_, _, _, _| {
            if is_known_click {
                let command = NetworkCommandMessage::connect_wifi(&ssid_click, None);
                broadcaster_click.broadcast_message_to_topic(command);
            } else {
                show_password_dialog(&ssid_click, &broadcaster_click, None);
            }
        });
        row.add_controller(click_gesture);

        box_widget.append(&row);
    }
}

fn update_vpn_list(box_widget: &GtkBox, vpn: &VpnProfilesMessage) {
    while box_widget.first_child().is_some() {
        if let Some(child) = box_widget.first_child() {
            box_widget.remove(&child);
        }
    }

    if vpn.profiles.is_empty() {
        let label = Label::builder()
            .label("No VPN profiles configured")
            .css_classes(["network-vpn-empty".to_string()])
            .halign(Align::Start)
            .build();
        box_widget.append(&label);
        return;
    }

    for profile in vpn.profiles.iter() {
        let name = profile.name.to_string();
        let is_active = profile.is_active;
        let status_icon = if is_active { "\u{f00c}" } else { "\u{f0c8}" };

        let row = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .css_classes(["network-vpn-row".to_string()])
            .halign(Align::Fill)
            .build();

        let label = Label::builder()
            .label(format!("\u{f099d} {name} {status_icon}"))
            .css_classes(["network-vpn-item".to_string()])
            .halign(Align::Start)
            .build();
        row.append(&label);

        box_widget.append(&row);
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

/// Draws a sparkline graph of throughput history on a DrawingArea.
fn draw_sparkline(cr: &gtk4::cairo::Context, width: i32, height: i32, history: &[f64]) {
    if history.is_empty() {
        return;
    }

    let w = width as f64;
    let h = height as f64;
    let max_val = history.iter().cloned().fold(0.0f64, f64::max).max(1.0);
    let step_x = w / (THROUGHPUT_HISTORY_MAX as f64 - 1.0).max(1.0);

    cr.set_source_rgba(0.3, 0.6, 1.0, 0.8);
    cr.set_line_width(1.5);

    let start_x = w - (history.len() as f64 - 1.0) * step_x;
    for (i, val) in history.iter().enumerate() {
        let x = start_x + i as f64 * step_x;
        let y = h - (val / max_val) * (h - 4.0) - 2.0;
        if i == 0 {
            cr.move_to(x, y);
        } else {
            cr.line_to(x, y);
        }
    }
    let _ = cr.stroke();
}

/// Generates a WiFi QR code string in the format `WIFI:T:WPA;S:<ssid>;P:<password>;;`.
fn generate_wifi_qr_string(ssid: &str, password: &str, security: &str) -> String {
    format!("WIFI:T:{security};S:{ssid};P:{password};;")
}

/// Renders a QR code as a grid of black/white squares on a DrawingArea.
fn draw_qr_code(cr: &gtk4::cairo::Context, width: i32, height: i32, qr_data: &qrcode::QrCode) {
    let modules = qr_data.width() as i32;
    let size = width.min(height);
    let cell_size = size as f64 / modules as f64;
    let offset_x = (width as f64 - size as f64) / 2.0;
    let offset_y = (height as f64 - size as f64) / 2.0;

    cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
    cr.rectangle(offset_x, offset_y, size as f64, size as f64);
    let _ = cr.fill();

    cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
    for y in 0..modules {
        for x in 0..modules {
            if qr_data[(x as usize, y as usize)] == qrcode::Color::Dark {
                let px = offset_x + x as f64 * cell_size;
                let py = offset_y + y as f64 * cell_size;
                cr.rectangle(px, py, cell_size, cell_size);
                let _ = cr.fill();
            }
        }
    }
}

/// Shows a QR code overlay for the current WiFi connection.
fn show_qr_overlay(qr_overlay: &SharedOverlay, status_label: &SharedLabel, broadcaster: &smearor_swipe_launcher_plugin_api::MessageBroadcasterInner) {
    let ssid = status_label
        .borrow()
        .as_ref()
        .and_then(|l| l.text().to_string().split_whitespace().nth(1).map(|s| s.to_string()))
        .unwrap_or_else(|| "Unknown".to_string());

    let qr_string = generate_wifi_qr_string(&ssid, "", "WPA");
    let qr_code = match qrcode::QrCode::new(qr_string.as_bytes()) {
        Ok(code) => code,
        Err(e) => {
            debug!("Network Widget: failed to generate QR code: {e}");
            return;
        }
    };

    let popup = gtk4::Window::builder()
        .title("WiFi QR Code")
        .default_width(300)
        .default_height(350)
        .modal(true)
        .build();

    let content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let qr_area = DrawingArea::builder().width_request(256).height_request(256).halign(Align::Center).build();
    let qr_code_clone = qr_code.clone();
    qr_area.set_draw_func(move |_, cr, w, h| {
        draw_qr_code(cr, w, h, &qr_code_clone);
    });
    content.append(&qr_area);

    let info_label = Label::builder().label(format!("Scan to connect to: {ssid}")).halign(Align::Center).build();
    content.append(&info_label);

    let close_btn = Button::builder().label("Close").halign(Align::Center).build();
    let popup_clone = popup.clone();
    close_btn.connect_clicked(move |_| {
        popup_clone.close();
    });
    content.append(&close_btn);

    popup.set_child(Some(&content));
    popup.present();

    let _ = qr_overlay;
    let _ = broadcaster;
}

/// Shows a password dialog for connecting to an unknown WiFi network.
fn show_password_dialog(ssid: &str, broadcaster: &smearor_swipe_launcher_plugin_api::MessageBroadcasterInner, parent: Option<&gtk4::Window>) {
    let dialog = gtk4::Window::builder()
        .title(format!("Connect to {ssid}"))
        .default_width(300)
        .default_height(150)
        .modal(true)
        .build();

    if let Some(p) = parent {
        dialog.set_transient_for(Some(p));
    }

    let content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let label = Label::builder().label(format!("Enter password for {ssid}:")).halign(Align::Start).build();
    content.append(&label);

    let entry = Entry::builder().visibility(false).placeholder_text("Password").halign(Align::Fill).build();
    content.append(&entry);

    let btn_box = GtkBox::builder().orientation(Orientation::Horizontal).spacing(8).halign(Align::Center).build();

    let cancel_btn = Button::builder().label("Cancel").build();
    let dialog_cancel = dialog.clone();
    cancel_btn.connect_clicked(move |_| {
        dialog_cancel.close();
    });
    btn_box.append(&cancel_btn);

    let connect_btn = Button::builder().label("Connect").css_classes(["suggested-action".to_string()]).build();
    let dialog_connect = dialog.clone();
    let ssid_clone = ssid.to_string();
    let entry_clone = entry.clone();
    let broadcaster_clone = broadcaster.clone();
    connect_btn.connect_clicked(move |_| {
        let password = entry_clone.text().to_string();
        let command = NetworkCommandMessage::connect_wifi(&ssid_clone, Some(&password));
        broadcaster_clone.broadcast_message_to_topic(command);
        dialog_connect.close();
    });
    btn_box.append(&connect_btn);

    content.append(&btn_box);

    dialog.set_child(Some(&content));
    dialog.present();
}
