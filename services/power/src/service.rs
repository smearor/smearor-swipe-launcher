use crate::config::PowerServiceConfig;
use crate::dbus::execute_power_action;
use crate::dbus::refresh_capabilities;
use crate::dbus::refresh_inhibitors;
use crate::scheduler::PowerState;
use crate::scheduler::run_countdown;
use crate::scheduler::run_scheduled_action;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_PROMPT;
use smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
use smearor_power_model::PowerAction;
use smearor_power_model::PowerCommandAction;
use smearor_power_model::PowerCommandMessage;
use smearor_power_model::PowerStatusMessage;
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
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::Notify;
use tracing::debug;
use tracing::error;
use tracing::trace;

/// Internal command enum for the service event loop.
pub enum PowerCommand {
    /// Execute a power action (with countdown if configured).
    Execute(PowerAction),
    /// Schedule a power action for the future.
    Schedule(PowerAction, u64),
    /// Cancel a running countdown or scheduled action.
    Cancel,
    /// Refresh capabilities and inhibitors from the system.
    Refresh,
}

pub struct PowerService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: PowerServiceConfig,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<PowerCommand>,
    pub command_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<PowerCommand>>,
    pub shared_state: Arc<Mutex<PowerState>>,
}

impl PowerService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        smearor_power_model::register_json_converters(core_context);

        let power_config: PowerServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;

        let (command_sender, command_receiver) = tokio::sync::mpsc::unbounded_channel::<PowerCommand>();
        let meta = PluginMeta::try_from(&config)?;
        let shared_state = Arc::new(Mutex::new(PowerState {
            status: PowerStatusMessage::default(),
        }));

        let mut service = PowerService {
            meta,
            core_context,
            config: power_config,
            command_sender,
            command_receiver: Some(command_receiver),
            shared_state,
        };

        service.spawn_async_runtime();

        Ok(service)
    }

    fn spawn_async_runtime(&mut self) {
        debug!(
            "Power Service: spawn_async_runtime called, core_context is {}",
            if self.core_context.is_some() { "Some" } else { "None" }
        );
        if let Some(ctx) = &self.core_context {
            let meta = self.meta.clone();
            let core_context = *ctx;
            let command_receiver = self.command_receiver.take();
            let config = self.config.clone();
            let shared_state = self.shared_state.clone();
            self.register_mcp_capabilities();
            trace!("Power Service: spawning async runtime thread");
            std::thread::spawn(move || {
                trace!("Power Service: async thread started, creating runtime");
                let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                    Ok(rt) => rt,
                    Err(e) => {
                        error!("Power Service: failed to create tokio runtime: {e}");
                        return;
                    }
                };
                let local_set = tokio::task::LocalSet::new();
                trace!("Power Service: runtime created, running async loop");
                local_set.block_on(&rt, async move {
                    if let Some(receiver) = command_receiver {
                        run_power_async(meta, core_context, receiver, config, shared_state).await;
                    } else {
                        error!("Power Service: command_receiver was None in async thread");
                    }
                });
            });
        } else {
            error!("Power Service: core_context is None, cannot spawn async runtime");
        }
    }

    pub(crate) fn state_snapshot(&self) -> PowerStatusMessage {
        self.shared_state.lock().map(|s| s.status.clone()).unwrap_or_default()
    }
}

impl MessageHandler<FfiEnvelopePayload<PowerCommandMessage>> for PowerService {
    fn handle_message(&self, message: FfiEnvelopePayload<PowerCommandMessage>, _sender_id: &str) {
        trace!("Power Service: received command {:?} for action {:?}", message.action, message.power_action);
        match message.action {
            PowerCommandAction::Execute => {
                if let Err(e) = self.command_sender.send(PowerCommand::Execute(message.power_action.clone())) {
                    trace!("Power Service: failed to send Execute command to async loop: {e}");
                }
            }
            PowerCommandAction::Schedule => {
                if let Err(e) = self
                    .command_sender
                    .send(PowerCommand::Schedule(message.power_action.clone(), message.delay_minutes as u64))
                {
                    trace!("Power Service: failed to send Schedule command to async loop: {e}");
                }
            }
            PowerCommandAction::Cancel => {
                if let Err(e) = self.command_sender.send(PowerCommand::Cancel) {
                    trace!("Power Service: failed to send Cancel command to async loop: {e}");
                }
            }
            PowerCommandAction::Refresh => {
                if let Err(e) = self.command_sender.send(PowerCommand::Refresh) {
                    trace!("Power Service: failed to send Refresh command to async loop: {e}");
                }
            }
        }
    }
}

impl MessageBroadcaster for PowerService {}

impl MessageTopicBroadcaster<PowerStatusMessage> for PowerService {}

impl PluginMetaGetter for PowerService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for PowerService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl ServicePlugin for PowerService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                let topic = envelope.topic.to_string();
                if envelope.type_id == FfiEnvelopePayload::<PowerCommandMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<PowerCommandMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_TOOL && envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_RESOURCE && envelope.type_id == FfiEnvelopePayload::<InvokeResourceMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_PROMPT && envelope.type_id == FfiEnvelopePayload::<InvokePromptMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokePromptMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

pub(crate) fn parse_action_from_string(s: &str) -> PowerAction {
    match s {
        "shutdown" => PowerAction::Shutdown,
        "reboot" => PowerAction::Reboot,
        "suspend" => PowerAction::Suspend,
        "hibernate" => PowerAction::Hibernate,
        "lock" => PowerAction::Lock,
        "logout" => PowerAction::Logout,
        _ => PowerAction::Cancel,
    }
}

fn current_iso8601() -> String {
    let now = std::time::SystemTime::now();
    let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    format!("{}", duration.as_secs())
}

fn send_status(meta: &PluginMeta, core_context: &FfiCoreContext, status: PowerStatusMessage) {
    let payload_ptr = box_payload(status);
    let envelope = FfiEnvelope::builder()
        .sender_id(meta.id.clone())
        .target_instance_id("*")
        .topic(PowerStatusMessage::topic())
        .type_id(PowerStatusMessage::TYPE_ID)
        .payload(payload_ptr)
        .destroy_payload(Some(default_destroy_payload))
        .clone_payload(Some(default_clone_payload::<PowerStatusMessage>))
        .build();
    core_context.send_message(envelope);
}

async fn run_power_async(
    meta: PluginMeta,
    core_context: FfiCoreContext,
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<PowerCommand>,
    config: PowerServiceConfig,
    state: Arc<Mutex<PowerState>>,
) {
    debug!("Power Service: starting async task");

    let connection = match zbus::Connection::system().await {
        Ok(c) => c,
        Err(e) => {
            error!("Power Service: failed to connect to system D-Bus: {e}");
            return;
        }
    };

    let (status_sender, mut status_receiver) = tokio::sync::mpsc::unbounded_channel::<PowerStatusMessage>();

    let forward_meta = meta.clone();
    let forward_core = core_context.clone();
    tokio::task::spawn_local(async move {
        while let Some(status) = status_receiver.recv().await {
            send_status(&forward_meta, &forward_core, status);
        }
    });

    let mut countdown_cancel: Option<Arc<Notify>> = None;
    let mut schedule_cancel: Option<Arc<Notify>> = None;

    let refresh_interval = Duration::from_secs(config.refresh_interval_seconds);
    let mut interval = tokio::time::interval(refresh_interval);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let conn_for_refresh = connection.clone();
    let state_for_refresh = state.clone();
    let status_sender_for_refresh = status_sender.clone();
    let config_for_refresh = config.clone();

    do_refresh(&conn_for_refresh, &state_for_refresh, &status_sender_for_refresh, &config_for_refresh).await;

    loop {
        tokio::select! {
            _ = interval.tick() => {
                do_refresh(&conn_for_refresh, &state_for_refresh, &status_sender_for_refresh, &config_for_refresh).await;
            }
            command = command_receiver.recv() => {
                match command {
                    Some(PowerCommand::Execute(action)) => {
                        if let Some(token) = countdown_cancel.take() {
                            token.notify_waiters();
                        }
                        if config.countdown_seconds > 0 && action != PowerAction::Cancel {
                            let token = Arc::new(Notify::new());
                            countdown_cancel = Some(token.clone());
                            let state_clone = state.clone();
                            let sender_clone = status_sender.clone();
                            let conn_clone = connection.clone();
                            let lock_cmd = config.lock_command.clone();
                            let logout_cmd = config.logout_command.clone();
                            tokio::task::spawn_local(async move {
                                run_countdown(action, config.countdown_seconds, state_clone, sender_clone, token, conn_clone, lock_cmd, logout_cmd).await;
                            });
                        } else {
                            let conn_clone = connection.clone();
                            let lock_cmd = config.lock_command.clone();
                            let logout_cmd = config.logout_command.clone();
                            execute_power_action(&conn_clone, &action, &lock_cmd, &logout_cmd).await;
                        }
                    }
                    Some(PowerCommand::Schedule(action, delay_minutes)) => {
                        if !config.enable_scheduled_actions {
                            debug!("Power Service: scheduled actions disabled, ignoring");
                            continue;
                        }
                        if let Some(token) = schedule_cancel.take() {
                            token.notify_waiters();
                        }
                        let token = Arc::new(Notify::new());
                        schedule_cancel = Some(token.clone());
                        let state_clone = state.clone();
                        let sender_clone = status_sender.clone();
                        let conn_clone = connection.clone();
                        let lock_cmd = config.lock_command.clone();
                        let logout_cmd = config.logout_command.clone();
                        let delay_seconds = delay_minutes * 60;
                        tokio::task::spawn_local(async move {
                            run_scheduled_action(action, delay_seconds, state_clone, sender_clone, token, conn_clone, lock_cmd, logout_cmd).await;
                        });
                    }
                    Some(PowerCommand::Cancel) => {
                        if let Some(token) = countdown_cancel.take() {
                            token.notify_waiters();
                        }
                        if let Some(token) = schedule_cancel.take() {
                            token.notify_waiters();
                        }
                        if let Ok(mut current) = state.lock() {
                            current.status.countdown_active = false;
                            current.status.countdown_remaining_seconds = 0;
                            current.status.countdown_total_seconds = 0;
                            current.status.scheduled_action = stabby::option::Option::None();
                            let status = current.status.clone();
                            drop(current);
                            let _ = status_sender.send(status);
                        }
                    }
                    Some(PowerCommand::Refresh) => {
                        do_refresh(&conn_for_refresh, &state_for_refresh, &status_sender_for_refresh, &config_for_refresh).await;
                    }
                    None => {
                        debug!("Power Service: command channel disconnected, exiting");
                        break;
                    }
                }
            }
        }
    }
}

async fn do_refresh(
    connection: &zbus::Connection,
    state: &Arc<Mutex<PowerState>>,
    status_sender: &tokio::sync::mpsc::UnboundedSender<PowerStatusMessage>,
    config: &PowerServiceConfig,
) {
    let capabilities = refresh_capabilities(connection).await;
    let inhibitors = if config.enable_inhibitor_detection {
        refresh_inhibitors(connection).await
    } else {
        Vec::new()
    };
    if let Ok(mut current) = state.lock() {
        current.status.capabilities = capabilities;
        let mut inhibitor_vec = stabby::vec::Vec::new();
        for inh in inhibitors {
            inhibitor_vec.push(inh);
        }
        current.status.inhibitors = inhibitor_vec;
        current.status.last_updated = stabby::string::String::from(current_iso8601());
        let status = current.status.clone();
        drop(current);
        let _ = status_sender.send(status);
    }
}
