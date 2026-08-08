use crate::config::NetworkServiceConfig;
use crate::dbus::activate_vpn;
use crate::dbus::connect_interface;
use crate::dbus::connect_to_wifi;
use crate::dbus::deactivate_vpn;
use crate::dbus::disconnect_interface;
use crate::dbus::disconnect_wifi;
use crate::dbus::get_all_access_points;
use crate::dbus::get_all_interfaces;
use crate::dbus::get_radio_state;
use crate::dbus::get_vpn_profiles;
use crate::dbus::request_wifi_scan;
use crate::dbus::set_wireless_enabled;
use crate::dbus::set_wwan_enabled;
use crate::throughput::SharedThroughputTracker;
use crate::throughput::ThroughputTracker;
use smearor_http_model::HttpMethod;
use smearor_http_model::HttpRequestMessage;
use smearor_http_model::TOPIC_HTTP_REQUEST;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_PROMPT;
use smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
use smearor_network_model::InterfaceStatus;
use smearor_network_model::NetworkCommandAction;
use smearor_network_model::NetworkCommandMessage;
use smearor_network_model::NetworkConnectionState;
use smearor_network_model::NetworkStatusMessage;
use smearor_network_model::ScanResultsMessage;
use smearor_network_model::VpnProfilesMessage;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::MessageTopicBroadcaster;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::ServicePlugin;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::box_payload;
use smearor_swipe_launcher_plugin_api::default_clone_payload;
use smearor_swipe_launcher_plugin_api::default_destroy_payload;
use smearor_swipe_launcher_plugin_api::generate_type_id;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tracing::debug;
use tracing::error;

/// Internal command enum for the service event loop.
pub enum NetworkCommand {
    /// Connect to a WLAN access point.
    ConnectWifi(String, Option<String>),
    /// Disconnect from an interface by interface name.
    Disconnect(String),
    /// Activate a connection on a specific interface (e.g., Ethernet reconnect).
    Connect(String),
    /// Toggle a radio technology.
    ToggleRadio(String, bool),
    /// Request a WLAN scan and broadcast results.
    ScanWifi,
    /// Toggle a VPN connection.
    ToggleVpn(String, bool),
    /// Refresh all status information.
    Refresh,
    /// Query the public IP address.
    GetPublicIp,
}

/// Shared state for the network service.
pub struct NetworkState {
    pub status: NetworkStatusMessage,
    pub scan_results: ScanResultsMessage,
    pub vpn_profiles: VpnProfilesMessage,
}

pub struct NetworkService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: NetworkServiceConfig,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<NetworkCommand>,
    pub command_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<NetworkCommand>>,
    pub shared_state: Arc<Mutex<NetworkState>>,
    pub throughput_tracker: SharedThroughputTracker,
}

impl NetworkService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        smearor_network_model::register_json_converters(core_context);

        let network_config: NetworkServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;

        let (command_sender, command_receiver) = tokio::sync::mpsc::unbounded_channel::<NetworkCommand>();
        let meta = PluginMeta::try_from(&config)?;
        let shared_state = Arc::new(Mutex::new(NetworkState {
            status: NetworkStatusMessage::default(),
            scan_results: ScanResultsMessage::default(),
            vpn_profiles: VpnProfilesMessage::default(),
        }));
        let throughput_tracker = Arc::new(Mutex::new(ThroughputTracker::new()));

        let service = NetworkService {
            meta,
            core_context,
            config: network_config,
            command_sender,
            command_receiver: Some(command_receiver),
            shared_state,
            throughput_tracker,
        };
        Ok(service)
    }

    pub(crate) fn state_snapshot(&self) -> NetworkStatusMessage {
        self.shared_state.lock().map(|s| s.status.clone()).unwrap_or_default()
    }

    pub(crate) fn scan_snapshot(&self) -> ScanResultsMessage {
        self.shared_state.lock().map(|s| s.scan_results.clone()).unwrap_or_default()
    }

    pub(crate) fn vpn_snapshot(&self) -> VpnProfilesMessage {
        self.shared_state.lock().map(|s| s.vpn_profiles.clone()).unwrap_or_default()
    }
}

impl MessageHandler<FfiEnvelopePayload<NetworkCommandMessage>> for NetworkService {
    fn handle_message(&self, message: FfiEnvelopePayload<NetworkCommandMessage>, _sender_id: &str) {
        debug!("Network Service: received command {:?}", message.action);
        match message.action {
            NetworkCommandAction::ConnectWifi => {
                let ssid = message.ssid.as_ref().map(|s| s.to_string()).unwrap_or_default();
                let password = message.password.as_ref().map(|s| s.to_string());
                let _ = self.command_sender.send(NetworkCommand::ConnectWifi(ssid, password));
            }
            NetworkCommandAction::Disconnect => {
                let interface_name = message.interface_name.as_ref().map(|s| s.to_string()).unwrap_or_default();
                debug!("Network Service: dispatching Disconnect interface={}", interface_name);
                let _ = self.command_sender.send(NetworkCommand::Disconnect(interface_name));
            }
            NetworkCommandAction::Connect => {
                let interface_name = message.interface_name.as_ref().map(|s| s.to_string()).unwrap_or_default();
                debug!("Network Service: dispatching Connect interface={}", interface_name);
                let _ = self.command_sender.send(NetworkCommand::Connect(interface_name));
            }
            NetworkCommandAction::ToggleRadio => {
                let technology = message.technology.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "wifi".to_string());
                debug!("Network Service: dispatching ToggleRadio technology={} enabled={}", technology, message.enabled);
                let _ = self.command_sender.send(NetworkCommand::ToggleRadio(technology, message.enabled));
            }
            NetworkCommandAction::ScanWifi => {
                debug!("Network Service: dispatching ScanWifi");
                let _ = self.command_sender.send(NetworkCommand::ScanWifi);
            }
            NetworkCommandAction::ToggleVpn => {
                let profile_name = message.profile_name.as_ref().map(|s| s.to_string()).unwrap_or_default();
                debug!("Network Service: dispatching ToggleVpn profile={} active={}", profile_name, message.active);
                let _ = self.command_sender.send(NetworkCommand::ToggleVpn(profile_name, message.active));
            }
            NetworkCommandAction::Refresh => {
                let _ = self.command_sender.send(NetworkCommand::Refresh);
            }
            NetworkCommandAction::GetPublicIp => {
                let _ = self.command_sender.send(NetworkCommand::GetPublicIp);
            }
        }
    }
}

impl MessageBroadcaster for NetworkService {}

impl MessageTopicBroadcaster<NetworkStatusMessage> for NetworkService {}

impl MessageTopicBroadcaster<ScanResultsMessage> for NetworkService {}

impl MessageTopicBroadcaster<VpnProfilesMessage> for NetworkService {}

impl PluginMetaGetter for NetworkService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for NetworkService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl ServicePlugin for NetworkService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                let topic = envelope.topic.to_string();
                if envelope.type_id == FfiEnvelopePayload::<NetworkCommandMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<NetworkCommandMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_TOOL && envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_RESOURCE && envelope.type_id == FfiEnvelopePayload::<InvokeResourceMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_PROMPT && envelope.type_id == FfiEnvelopePayload::<smearor_model_mcp::InvokePromptMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<smearor_model_mcp::InvokePromptMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }

    fn start(&mut self) {
        if let Some(ctx) = &self.core_context {
            let meta = self.meta.clone();
            let core_context = *ctx;
            let command_receiver = self.command_receiver.take();
            let config = self.config.clone();
            let shared_state = self.shared_state.clone();
            let throughput_tracker = self.throughput_tracker.clone();
            self.register_mcp_capabilities();
            std::thread::spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                    Ok(rt) => rt,
                    Err(e) => {
                        error!("Network Service: failed to create tokio runtime: {e}");
                        return;
                    }
                };
                let local_set = tokio::task::LocalSet::new();
                local_set.block_on(&rt, async move {
                    if let Some(receiver) = command_receiver {
                        run_network_async(meta, core_context, receiver, config, shared_state, throughput_tracker).await;
                    }
                });
            });
        }
    }
}

fn current_iso8601() -> String {
    let now = std::time::SystemTime::now();
    let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    format!("{}", duration.as_secs())
}

fn send_status(meta: &PluginMeta, core_context: &FfiCoreContext, status: NetworkStatusMessage) {
    let payload_ptr = box_payload(status);
    let envelope = FfiEnvelope::builder()
        .sender_id(meta.id.clone())
        .target_instance_id("*")
        .topic(NetworkStatusMessage::topic())
        .type_id(NetworkStatusMessage::TYPE_ID)
        .payload(payload_ptr)
        .destroy_payload(Some(default_destroy_payload))
        .clone_payload(Some(default_clone_payload::<NetworkStatusMessage>))
        .build();
    core_context.send_message(envelope);
}

fn send_scan_results(meta: &PluginMeta, core_context: &FfiCoreContext, scan: ScanResultsMessage) {
    let payload_ptr = box_payload(scan);
    let envelope = FfiEnvelope::builder()
        .sender_id(meta.id.clone())
        .target_instance_id("*")
        .topic(ScanResultsMessage::topic())
        .type_id(ScanResultsMessage::TYPE_ID)
        .payload(payload_ptr)
        .destroy_payload(Some(default_destroy_payload))
        .clone_payload(Some(default_clone_payload::<ScanResultsMessage>))
        .build();
    core_context.send_message(envelope);
}

fn send_vpn_profiles(meta: &PluginMeta, core_context: &FfiCoreContext, profiles: VpnProfilesMessage) {
    let payload_ptr = box_payload(profiles);
    let envelope = FfiEnvelope::builder()
        .sender_id(meta.id.clone())
        .target_instance_id("*")
        .topic(VpnProfilesMessage::topic())
        .type_id(VpnProfilesMessage::TYPE_ID)
        .payload(payload_ptr)
        .destroy_payload(Some(default_destroy_payload))
        .clone_payload(Some(default_clone_payload::<VpnProfilesMessage>))
        .build();
    core_context.send_message(envelope);
}

fn select_primary_interface(interfaces: &[InterfaceStatus]) -> InterfaceStatus {
    for iface in interfaces {
        if iface.state == NetworkConnectionState::Connected && iface.internet_accessible {
            return iface.clone();
        }
    }
    for iface in interfaces {
        if iface.state == NetworkConnectionState::Connected {
            return iface.clone();
        }
    }
    interfaces.first().cloned().unwrap_or_default()
}

#[allow(clippy::too_many_arguments)]
async fn run_network_async(
    meta: PluginMeta,
    core_context: FfiCoreContext,
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<NetworkCommand>,
    config: NetworkServiceConfig,
    state: Arc<Mutex<NetworkState>>,
    throughput_tracker: SharedThroughputTracker,
) {
    debug!("Network Service: starting async task");

    let connection = match zbus::Connection::system().await {
        Ok(c) => c,
        Err(e) => {
            error!("Network Service: failed to connect to system D-Bus: {e}");
            return;
        }
    };

    let (status_sender, mut status_receiver) = tokio::sync::mpsc::unbounded_channel::<NetworkStatusMessage>();
    let (scan_sender, mut scan_receiver) = tokio::sync::mpsc::unbounded_channel::<ScanResultsMessage>();
    let (vpn_sender, mut vpn_receiver) = tokio::sync::mpsc::unbounded_channel::<VpnProfilesMessage>();

    let forward_meta = meta.clone();
    let forward_core = core_context;
    tokio::task::spawn_local(async move {
        while let Some(status) = status_receiver.recv().await {
            send_status(&forward_meta, &forward_core, status);
        }
    });

    let forward_meta_scan = meta.clone();
    let forward_core_scan = core_context;
    tokio::task::spawn_local(async move {
        while let Some(scan) = scan_receiver.recv().await {
            send_scan_results(&forward_meta_scan, &forward_core_scan, scan);
        }
    });

    let forward_meta_vpn = meta.clone();
    let forward_core_vpn = core_context;
    tokio::task::spawn_local(async move {
        while let Some(vpn) = vpn_receiver.recv().await {
            send_vpn_profiles(&forward_meta_vpn, &forward_core_vpn, vpn);
        }
    });

    let refresh_interval = Duration::from_secs(config.refresh_interval_seconds);
    let mut interval = tokio::time::interval(refresh_interval);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let throughput_interval = Duration::from_secs(config.throughput_sample_interval_seconds);
    let mut throughput_timer = tokio::time::interval(throughput_interval);
    throughput_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let conn_for_refresh = connection.clone();
    let state_for_refresh = state.clone();
    let status_sender_for_refresh = status_sender.clone();
    let throughput_for_refresh = throughput_tracker.clone();

    do_refresh(&conn_for_refresh, &state_for_refresh, &status_sender_for_refresh, &throughput_for_refresh).await;

    if config.enable_vpn_management {
        refresh_vpn(&connection, &state, &vpn_sender).await;
    }

    loop {
        tokio::select! {
            _ = interval.tick() => {
                do_refresh(&conn_for_refresh, &state_for_refresh, &status_sender_for_refresh, &throughput_for_refresh).await;
                if config.enable_vpn_management {
                    refresh_vpn(&connection, &state, &vpn_sender).await;
                }
            }
            _ = throughput_timer.tick() => {
                if config.enable_throughput_monitoring {
                    let (rx_bps, tx_bps) = {
                        let mut tracker = throughput_tracker.lock().unwrap();
                        tracker.sample()
                    };
                    if let Ok(mut current) = state.lock() {
                        current.status.received_bytes_per_second = rx_bps;
                        current.status.transmitted_bytes_per_second = tx_bps;
                        let status = current.status.clone();
                        drop(current);
                        let _ = status_sender.send(status);
                    }
                }
            }
            command = command_receiver.recv() => {
                match command {
                    Some(NetworkCommand::ConnectWifi(ssid, password)) => {
                        debug!("Network Service: connect WiFi ssid={ssid}");
                        connect_to_wifi(&connection, &ssid, password.as_deref()).await;
                    }
                    Some(NetworkCommand::Disconnect(interface_name)) => {
                        debug!("Network Service: disconnect {interface_name}");
                        if interface_name.is_empty() {
                            disconnect_wifi(&connection).await;
                        } else {
                            disconnect_interface(&connection, &interface_name).await;
                        }
                        do_refresh(&conn_for_refresh, &state_for_refresh, &status_sender_for_refresh, &throughput_for_refresh).await;
                    }
                    Some(NetworkCommand::Connect(interface_name)) => {
                        debug!("Network Service: connect interface {interface_name}");
                        // If WiFi radio is off, enable it and wait for the device to become available
                        let (wifi_enabled, _) = get_radio_state(&connection).await;
                        if !wifi_enabled && interface_name.starts_with("wl") {
                            debug!("Network Service: WiFi radio off, enabling before connect");
                            set_wireless_enabled(&connection, true).await;
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                        connect_interface(&connection, &interface_name).await;
                        do_refresh(&conn_for_refresh, &state_for_refresh, &status_sender_for_refresh, &throughput_for_refresh).await;
                    }
                    Some(NetworkCommand::ToggleRadio(technology, enabled)) => {
                        debug!("Network Service: toggle radio {technology}={enabled}");
                        match technology.as_str() {
                            "wifi" => set_wireless_enabled(&connection, enabled).await,
                            "wwan" => set_wwan_enabled(&connection, enabled).await,
                            "all" => {
                                set_wireless_enabled(&connection, enabled).await;
                                set_wwan_enabled(&connection, enabled).await;
                            }
                            _ => {}
                        }
                        do_refresh(&conn_for_refresh, &state_for_refresh, &status_sender_for_refresh, &throughput_for_refresh).await;
                    }
                    Some(NetworkCommand::ScanWifi) => {
                        debug!("Network Service: scan WiFi");
                        if config.enable_wifi_scan {
                            request_wifi_scan(&connection).await;
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            let aps = get_all_access_points(&connection).await;
                            let scan_time = stabby::string::String::from(current_iso8601());
                            let mut ap_vec = stabby::vec::Vec::new();
                            for ap in aps {
                                ap_vec.push(ap);
                            }
                            let scan_msg = ScanResultsMessage {
                                access_points: ap_vec,
                                scan_time,
                            };
                            if let Ok(mut current) = state.lock() {
                                current.scan_results = scan_msg.clone();
                            }
                            let _ = scan_sender.send(scan_msg);
                        }
                    }
                    Some(NetworkCommand::ToggleVpn(profile_name, active)) => {
                        debug!("Network Service: toggle VPN {profile_name}={active}");
                        if config.enable_vpn_management {
                            if active {
                                activate_vpn(&connection, &profile_name).await;
                            } else {
                                deactivate_vpn(&connection, &profile_name).await;
                            }
                            refresh_vpn(&connection, &state, &vpn_sender).await;
                        }
                    }
                    Some(NetworkCommand::Refresh) => {
                        do_refresh(&conn_for_refresh, &state_for_refresh, &status_sender_for_refresh, &throughput_for_refresh).await;
                        if config.enable_vpn_management {
                            refresh_vpn(&connection, &state, &vpn_sender).await;
                        }
                    }
                    Some(NetworkCommand::GetPublicIp) => {
                        debug!("Network Service: requesting public IP via HTTP service");
                        let request = HttpRequestMessage {
                            method: HttpMethod::Get,
                            url: config.public_ip_url.clone(),
                            response_topic: String::from("service.network.public_ip_response"),
                            body: None,
                            headers: None,
                            timeout_ms: None,
                        };
                        let payload = match serde_json::to_string(&request) {
                            Ok(s) => s,
                            Err(e) => {
                                error!("Network Service: failed to serialize HTTP request for public IP: {e}");
                                continue;
                            }
                        };
                        let payload_ptr = box_payload(payload);
                        let envelope = FfiEnvelope::builder()
                            .sender_id(meta.id.clone())
                            .target_instance_id("*")
                            .topic(TOPIC_HTTP_REQUEST)
                            .type_id(generate_type_id("std::string::String"))
                            .payload(payload_ptr)
                            .destroy_payload(Some(default_destroy_payload))
                            .clone_payload(Some(default_clone_payload::<String>))
                            .build();
                        core_context.send_message(envelope);
                    }
                    None => {
                        debug!("Network Service: command channel disconnected, exiting");
                        break;
                    }
                }
            }
        }
    }
}

async fn do_refresh(
    connection: &zbus::Connection,
    state: &Arc<Mutex<NetworkState>>,
    status_sender: &tokio::sync::mpsc::UnboundedSender<NetworkStatusMessage>,
    throughput_tracker: &SharedThroughputTracker,
) {
    let interfaces = get_all_interfaces(connection).await;
    let (wifi_enabled, wwan_enabled) = get_radio_state(connection).await;
    let airplane_mode = !wifi_enabled && !wwan_enabled;

    let primary_interface = select_primary_interface(&interfaces);

    let (rx_bps, tx_bps) = {
        let mut tracker = throughput_tracker.lock().unwrap();
        tracker.sample()
    };

    let mut iface_vec = stabby::vec::Vec::new();
    for iface in interfaces {
        iface_vec.push(iface);
    }

    if let Ok(mut current) = state.lock() {
        current.status.primary_interface = primary_interface;
        current.status.interfaces = iface_vec;
        current.status.wifi_enabled = wifi_enabled;
        current.status.wwan_enabled = wwan_enabled;
        current.status.airplane_mode = airplane_mode;
        current.status.received_bytes_per_second = rx_bps;
        current.status.transmitted_bytes_per_second = tx_bps;
        current.status.last_updated = stabby::string::String::from(current_iso8601());
        let status = current.status.clone();
        drop(current);
        let _ = status_sender.send(status);
    }
}

async fn refresh_vpn(connection: &zbus::Connection, state: &Arc<Mutex<NetworkState>>, vpn_sender: &tokio::sync::mpsc::UnboundedSender<VpnProfilesMessage>) {
    let profiles = get_vpn_profiles(connection).await;
    let mut profile_vec = stabby::vec::Vec::new();
    for p in profiles {
        profile_vec.push(p);
    }
    if let Ok(mut current) = state.lock() {
        current.vpn_profiles.profiles = profile_vec;
        current.vpn_profiles.last_updated = stabby::string::String::from(current_iso8601());
        let vpn_msg = current.vpn_profiles.clone();
        drop(current);
        let _ = vpn_sender.send(vpn_msg);
    }
}
