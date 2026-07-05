use crate::collector::CollectorState;
use crate::config::SysinfoServiceConfig;
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
use tracing::debug;

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

pub struct SysinfoService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: SysinfoServiceConfig,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<SysinfoCommandAction>,
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

        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(runtime) => runtime,
                Err(error) => {
                    debug!("Sysinfo service: failed to create tokio runtime: {error}");
                    return;
                }
            };
            runtime.block_on(async move {
                run_update_loop(service_config_clone, command_receiver, meta_clone, core_context_clone).await;
            });
        });

        Ok(SysinfoService {
            meta,
            core_context,
            config: service_config,
            command_sender,
        })
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

impl Service for SysinfoService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                if envelope.type_id == FfiEnvelopePayload::<SysinfoCommandMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<SysinfoCommandMessage>>::handle_envelope_message(self, envelope);
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

        broadcast(&meta, &core_context, cpu);
        broadcast(&meta, &core_context, memory);
        broadcast(&meta, &core_context, battery);
        broadcast(&meta, &core_context, disks);
        broadcast(&meta, &core_context, network);
        broadcast(&meta, &core_context, uptime);

        debug!("Sysinfo service broadcasted all metrics");
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
