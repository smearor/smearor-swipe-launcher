use crate::collector::CollectorState;
use crate::config::SysinfoServiceConfig;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::MessageTopicBroadcaster;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::Service;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;
use smearor_sysinfo_model::BatteryStatusMessage;
use smearor_sysinfo_model::CpuStatusMessage;
use smearor_sysinfo_model::DisksStatusMessage;
use smearor_sysinfo_model::MemoryStatusMessage;
use smearor_sysinfo_model::NetworkStatusMessage;
use smearor_sysinfo_model::UptimeStatusMessage;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::debug;
use tracing::trace;

/// Command action for the sysinfo service.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub enum SysinfoCommandAction {
    /// No operation.
    #[default]
    None,
    /// Force an immediate refresh of all metrics.
    Refresh,
}

/// Command message sent to the sysinfo service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct SysinfoCommandMessage {
    /// Action to execute.
    pub action: SysinfoCommandAction,
}

/// Latest collected sysinfo metrics shared between the update loop and the
/// MCP invocation handlers.
#[derive(Clone, Default)]
pub struct LatestState {
    pub cpu: CpuStatusMessage,
    pub memory: MemoryStatusMessage,
    pub battery: BatteryStatusMessage,
    pub disks: DisksStatusMessage,
    pub network: NetworkStatusMessage,
    pub uptime: UptimeStatusMessage,
}

pub struct SysinfoService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: SysinfoServiceConfig,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<SysinfoCommandAction>,
    pub latest_state: Arc<RwLock<LatestState>>,
}

impl SysinfoService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let service_config: SysinfoServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let (command_sender, command_receiver) = tokio::sync::mpsc::unbounded_channel::<SysinfoCommandAction>();
        let meta = PluginMeta::try_from(&config)?;
        let meta_clone = meta.clone();
        let core_context_clone = core_context.clone();
        let service_config_clone = service_config.clone();
        let latest_state = Arc::new(RwLock::new(LatestState::default()));
        let latest_state_clone = latest_state.clone();

        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(runtime) => runtime,
                Err(error) => {
                    debug!("Sysinfo service: failed to create tokio runtime: {error}");
                    return;
                }
            };
            runtime.block_on(async move {
                run_update_loop(service_config_clone, command_receiver, meta_clone, core_context_clone, latest_state_clone).await;
            });
        });

        let service = SysinfoService {
            meta,
            core_context: core_context.clone(),
            config: service_config,
            command_sender,
            latest_state,
        };
        service.register_mcp_capabilities();
        Ok(service)
    }

    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let resources = [
            ("sysinfo://cpu", "CPU Status", "Current CPU usage and temperature.", "application/json"),
            ("sysinfo://memory", "Memory Status", "Current memory usage and available memory.", "application/json"),
            ("sysinfo://battery", "Battery Status", "Current battery level and charging state.", "application/json"),
            ("sysinfo://disks", "Disk Status", "Per-mount usage and disk throughput.", "application/json"),
            ("sysinfo://network", "Network Status", "Inbound and outbound network throughput.", "application/json"),
            ("sysinfo://uptime", "Uptime Status", "System uptime and load averages.", "application/json"),
        ];
        for (uri, name, description, mime_type) in resources {
            let resource = RegisterResourceMessage::new(uri, name, description, mime_type);
            broadcaster.broadcast_message_to_topic(resource);
        }

        let tool = RegisterToolMessage::new(
            "sysinfo_refresh",
            "Force an immediate refresh of all sysinfo metrics.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(tool);
    }

    fn send_response<T: TypedMessage + SharedMessage + Clone>(&self, message: T, sender_id: &str) {
        let payload_ptr = Box::into_raw(Box::new(message.clone())) as *mut core::ffi::c_void;
        let sender_id_string = sender_id.to_string();
        let topic = message.topic();
        debug!("sysinfo: send_response topic={} to sender_id={}", topic, sender_id);
        let envelope = FfiEnvelope {
            sender_id: stabby::string::String::from(self.meta.id.clone()),
            target_instance_id: stabby::string::String::from(sender_id_string.as_str()),
            topic: stabby::string::String::from(topic),
            type_id: T::TYPE_ID,
            payload: payload_ptr,
            destroy_payload: Some(destroy_payload::<T>),
            clone_payload: Some(clone_payload::<T>),
        };
        if let Some(context) = &self.core_context {
            debug!("sysinfo: calling context.send_message");
            context.send_message(envelope);
        } else {
            debug!("sysinfo: no core_context, cannot send response");
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<SysinfoCommandMessage>> for SysinfoService {
    fn handle_message(&self, message: FfiEnvelopePayload<SysinfoCommandMessage>, _sender_id: &str) {
        match message.action {
            SysinfoCommandAction::None => {}
            SysinfoCommandAction::Refresh => {
                let _ = self.command_sender.send(SysinfoCommandAction::Refresh);
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for SysinfoService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, sender_id: &str) {
        debug!("sysinfo: InvokeToolMessage handler name={} sender_id={}", message.0.name, sender_id);
        if message.0.name.to_string() != "sysinfo_refresh" {
            debug!("sysinfo: InvokeToolMessage not sysinfo_refresh, ignoring");
            return;
        }
        let _ = self.command_sender.send(SysinfoCommandAction::Refresh);
        let correlation_id = message.0.correlation_id.to_string();
        let response = InvokeToolResponse::success(&correlation_id, "Refresh triggered");
        debug!("sysinfo: sending InvokeToolResponse correlation_id={}", correlation_id);
        self.send_response(response, sender_id);
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for SysinfoService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        debug!("sysinfo: InvokeResourceMessage handler uri={} sender_id={}", message.0.uri, sender_id);
        let uri = message.0.uri.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        let state = match self.latest_state.read() {
            Ok(state) => state.clone(),
            Err(_) => {
                let response = InvokeResourceResponse::error(&correlation_id, "Failed to read sysinfo state");
                self.send_response(response, sender_id);
                return;
            }
        };
        let result = serialize_resource_state(&uri, &state);
        let response = match result {
            Ok(contents) => InvokeResourceResponse::success(&correlation_id, &contents),
            Err(error) => InvokeResourceResponse::error(&correlation_id, &error),
        };
        debug!("sysinfo: sending InvokeResourceResponse correlation_id={} uri={}", correlation_id, uri);
        self.send_response(response, sender_id);
    }
}

impl MessageBroadcaster for SysinfoService {}

impl MessageTopicBroadcaster<CpuStatusMessage> for SysinfoService {}
impl MessageTopicBroadcaster<MemoryStatusMessage> for SysinfoService {}
impl MessageTopicBroadcaster<BatteryStatusMessage> for SysinfoService {}
impl MessageTopicBroadcaster<DisksStatusMessage> for SysinfoService {}
impl MessageTopicBroadcaster<NetworkStatusMessage> for SysinfoService {}
impl MessageTopicBroadcaster<UptimeStatusMessage> for SysinfoService {}

impl PluginMetaGetter for SysinfoService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for SysinfoService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Service for SysinfoService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                debug!("sysinfo: on_message topic={} type_id={}", envelope.topic, envelope.type_id);
                debug!("sysinfo: expecting tool type_id={}", FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID);
                debug!("sysinfo: expecting resource type_id={}", FfiEnvelopePayload::<InvokeResourceMessage>::TYPE_ID);
                if envelope.type_id == FfiEnvelopePayload::<SysinfoCommandMessage>::TYPE_ID {
                    debug!("sysinfo: handling command message");
                    MessageHandler::<FfiEnvelopePayload<SysinfoCommandMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    debug!("sysinfo: handling invoke tool message");
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeResourceMessage>::TYPE_ID {
                    debug!("sysinfo: handling invoke resource message");
                    MessageHandler::<FfiEnvelopePayload<InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                } else {
                    debug!("sysinfo: unknown type_id");
                }
            }
        }
    }
}

async fn run_update_loop(
    config: SysinfoServiceConfig,
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<SysinfoCommandAction>,
    meta: PluginMeta,
    core_context: Option<FfiCoreContext>,
    latest_state: Arc<RwLock<LatestState>>,
) {
    let mut state = CollectorState::default();
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(config.update_interval_ms));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {}
            command = command_receiver.recv() => {
                if command.is_none() {
                    break;
                }
            }
        }

        let cpu = crate::collector::collect_cpu(config.enable_cpu_temperature, config.cpu_temperature_source.as_deref(), &mut state).await;
        let memory = crate::collector::collect_memory().await;
        let battery = if config.enable_battery {
            crate::collector::collect_battery().await
        } else {
            BatteryStatusMessage::default()
        };
        let disks = if config.enable_disks {
            crate::collector::collect_disks(&mut state).await
        } else {
            DisksStatusMessage::default()
        };
        let network = if config.enable_network {
            crate::collector::collect_network(&mut state).await
        } else {
            NetworkStatusMessage::default()
        };
        let uptime = crate::collector::collect_uptime().await;

        {
            let mut guard = latest_state.write().expect("latest state lock poisoned");
            guard.cpu.clone_from(&cpu);
            guard.memory.clone_from(&memory);
            guard.battery.clone_from(&battery);
            guard.disks.clone_from(&disks);
            guard.network.clone_from(&network);
            guard.uptime.clone_from(&uptime);
        }

        broadcast(&meta, &core_context, cpu);
        broadcast(&meta, &core_context, memory);
        broadcast(&meta, &core_context, battery);
        broadcast(&meta, &core_context, disks);
        broadcast(&meta, &core_context, network);
        broadcast(&meta, &core_context, uptime);

        trace!("Sysinfo service broadcasted all metrics");
    }
}

fn broadcast<T: Clone + MessageTopic + TypedMessage>(meta: &PluginMeta, core_context: &Option<FfiCoreContext>, message: T) {
    let payload_ptr = Box::into_raw(Box::new(message.clone())) as *mut core::ffi::c_void;
    let envelope = FfiEnvelope {
        sender_id: stabby::string::String::from(meta.id.clone()),
        target_instance_id: stabby::string::String::from(""),
        topic: stabby::string::String::from(T::topic()),
        type_id: T::TYPE_ID,
        payload: payload_ptr,
        destroy_payload: Some(destroy_payload::<T>),
        clone_payload: Some(clone_payload::<T>),
    };

    if let Some(context) = core_context {
        context.send_message(envelope);
    }
}

extern "C" fn clone_payload<T: Clone>(ptr: *mut core::ffi::c_void) -> *mut core::ffi::c_void {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let value = unsafe { &*(ptr as *const T) };
    Box::into_raw(Box::new(value.clone())) as *mut core::ffi::c_void
}

extern "C" fn destroy_payload<T>(ptr: *mut core::ffi::c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut T);
        }
    }
}

fn serialize_resource_state(uri: &str, state: &LatestState) -> Result<String, String> {
    match uri {
        "sysinfo://cpu" => Ok(serde_json::json!({
            "cpu_usage": state.cpu.cpu_usage,
            "cpu_temperature": state.cpu.cpu_temperature.as_ref().copied(),
        })
        .to_string()),
        "sysinfo://memory" => Ok(serde_json::json!({
            "memory_usage": state.memory.memory_usage,
            "memory_total": state.memory.memory_total,
            "memory_used": state.memory.memory_used,
            "memory_available": state.memory.memory_available,
        })
        .to_string()),
        "sysinfo://battery" => Ok(serde_json::json!({
            "level": state.battery.level,
            "status": format!("{:?}", state.battery.status),
        })
        .to_string()),
        "sysinfo://disks" => Ok(serde_json::json!({
            "mounts": state.disks.mounts.iter().map(|disk| serde_json::json!({
                "mount_point": disk.mount_point.to_string(),
                "usage": disk.usage,
                "total": disk.total,
                "used": disk.used,
                "available": disk.available,
            })).collect::<Vec<_>>(),
            "read_bytes_per_second": state.disks.read_bytes_per_second,
            "write_bytes_per_second": state.disks.write_bytes_per_second,
        })
        .to_string()),
        "sysinfo://network" => Ok(serde_json::json!({
            "received_bytes_per_second": state.network.received_bytes_per_second,
            "transmitted_bytes_per_second": state.network.transmitted_bytes_per_second,
        })
        .to_string()),
        "sysinfo://uptime" => Ok(serde_json::json!({
            "uptime_seconds": state.uptime.uptime_seconds,
            "load_average_1_minute": state.uptime.load_average_1_minute,
            "load_average_5_minute": state.uptime.load_average_5_minute,
            "load_average_15_minute": state.uptime.load_average_15_minute,
        })
        .to_string()),
        _ => Err(format!("Unknown resource uri: {}", uri)),
    }
}

#[cfg(test)]
mod tests {
    use super::LatestState;
    use super::serialize_resource_state;

    #[test]
    fn serialize_cpu_resource_includes_usage() {
        let mut state = LatestState::default();
        state.cpu.cpu_usage = 42.5;
        let json = serialize_resource_state("sysinfo://cpu", &state).expect("valid cpu resource");
        assert!(json.contains("\"cpu_usage\":42.5"), "cpu_usage should be serialized: {}", json);
    }

    #[test]
    fn serialize_memory_resource_includes_total() {
        let mut state = LatestState::default();
        state.memory.memory_total = 16_000_000_000;
        let json = serialize_resource_state("sysinfo://memory", &state).expect("valid memory resource");
        assert!(json.contains("\"memory_total\":16000000000"), "memory_total should be serialized: {}", json);
    }

    #[test]
    fn serialize_resource_unknown_uri_returns_error() {
        let state = LatestState::default();
        let result = serialize_resource_state("sysinfo://unknown", &state);
        assert!(result.is_err());
    }
}

impl TypedMessage for SysinfoCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_sysinfo_service::SysinfoCommandMessage");
}

// TODO: Check if this works actually

impl MessageTopic for SysinfoCommandMessage {
    fn topic() -> &'static str {
        "service.sysinfo.command"
    }
}

// TODO: Check if this works actually

impl SharedMessage for SysinfoCommandMessage {
    fn topic(&self) -> &'static str {
        "service.sysinfo.command"
    }
}
