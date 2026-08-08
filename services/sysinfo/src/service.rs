use crate::collector::CollectorState;
use crate::config::SysinfoServiceConfig;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_PROMPT;
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
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::box_payload;
use smearor_swipe_launcher_plugin_api::default_clone_payload;
use smearor_swipe_launcher_plugin_api::default_destroy_payload;
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
    #[allow(unused)]
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

impl ServicePlugin for SysinfoService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                trace!("sysinfo: on_message topic={} type_id={}", envelope.topic, envelope.type_id);
                if envelope.type_id == FfiEnvelopePayload::<SysinfoCommandMessage>::TYPE_ID {
                    trace!("sysinfo: handling command message");
                    MessageHandler::<FfiEnvelopePayload<SysinfoCommandMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    trace!("sysinfo: handling invoke tool message");
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeResourceMessage>::TYPE_ID {
                    trace!("sysinfo: handling invoke resource message");
                    MessageHandler::<FfiEnvelopePayload<InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.topic.to_string() == TOPIC_MCP_INVOKE_PROMPT && envelope.type_id == FfiEnvelopePayload::<InvokePromptMessage>::TYPE_ID {
                    trace!("sysinfo: handling invoke prompt message");
                    MessageHandler::<FfiEnvelopePayload<InvokePromptMessage>>::handle_envelope_message(self, envelope);
                } else {
                    trace!("sysinfo: unknown type_id");
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

        let cpu = crate::collector::collect_cpu(
            config.enable_cpu_temperature,
            config.cpu_temperature_source.as_deref(),
            config.cpu_temperature_component.as_deref(),
            &mut state,
        )
        .await;
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
    let payload_ptr = box_payload(message.clone());
    let envelope = FfiEnvelope::builder()
        .sender_id(meta.id.clone())
        .target_instance_id("")
        .topic(T::topic())
        .type_id(T::TYPE_ID)
        .payload(payload_ptr)
        .destroy_payload(Some(default_destroy_payload))
        .clone_payload(Some(default_clone_payload::<T>))
        .build();

    if let Some(context) = core_context {
        context.send_message(envelope);
    }
}

impl TypedMessage for SysinfoCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_sysinfo_service::SysinfoCommandMessage");
}

impl MessageTopic for SysinfoCommandMessage {
    fn topic() -> &'static str {
        "service.sysinfo.command"
    }
}

impl SharedMessage for SysinfoCommandMessage {
    fn topic(&self) -> &'static str {
        "service.sysinfo.command"
    }
}
