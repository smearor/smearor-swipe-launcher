use crate::config::PowerServiceConfig;
use crate::dbus::execute_power_action;
use crate::dbus::refresh_capabilities;
use crate::dbus::refresh_inhibitors;
use crate::scheduler::PowerState;
use crate::scheduler::run_countdown;
use crate::scheduler::run_scheduled_action;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
use smearor_power_model::PowerAction;
use smearor_power_model::PowerCommandAction;
use smearor_power_model::PowerCommandMessage;
use smearor_power_model::PowerStatusMessage;
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
use smearor_swipe_launcher_plugin_api::TypedMessage;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::Notify;
use tracing::debug;
use tracing::error;

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

        let service = PowerService {
            meta,
            core_context,
            config: power_config,
            command_sender,
            command_receiver: Some(command_receiver),
            shared_state,
        };
        Ok(service)
    }

    fn state_snapshot(&self) -> PowerStatusMessage {
        self.shared_state.lock().map(|s| s.status.clone()).unwrap_or_default()
    }

    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let capabilities_resource = RegisterResourceMessage::new(
            "power://capabilities",
            "Power Capabilities",
            "System power capabilities as reported by systemd-logind.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(capabilities_resource);

        let inhibitors_resource = RegisterResourceMessage::new(
            "power://inhibitors",
            "Power Inhibitors",
            "List of active inhibitor locks blocking power actions.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(inhibitors_resource);

        let scheduled_resource = RegisterResourceMessage::new(
            "power://scheduled_actions",
            "Scheduled Power Actions",
            "Currently scheduled power action, if any.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(scheduled_resource);

        let power_action_tool = RegisterToolMessage::new(
            "system_power_action",
            "Executes the desired power action immediately.",
            r#"{ "type": "object", "properties": { "action": { "type": "string", "enum": ["shutdown", "reboot", "suspend", "hibernate", "lock", "logout"], "description": "The power action to execute" } }, "required": ["action"] }"#,
        );
        broadcaster.broadcast_message_to_topic(power_action_tool);

        let schedule_tool = RegisterToolMessage::new(
            "system_schedule_power_action",
            "Schedules a shutdown or reboot in the future.",
            r#"{ "type": "object", "properties": { "action": { "type": "string", "enum": ["shutdown", "reboot"], "description": "The power action to schedule" }, "delay_minutes": { "type": "integer", "minimum": 1, "description": "Delay in minutes before the action executes" } }, "required": ["action", "delay_minutes"] }"#,
        );
        broadcaster.broadcast_message_to_topic(schedule_tool);

        let cancel_tool = RegisterToolMessage::new(
            "system_cancel_power_action",
            "Cancels a running shutdown timer or scheduled action.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(cancel_tool);

        let uefi_tool = RegisterToolMessage::new(
            "system_reboot_to_uefi",
            "Sets the firmware reboot flag and reboots directly into BIOS/UEFI.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(uefi_tool);
    }
}

impl MessageHandler<FfiEnvelopePayload<PowerCommandMessage>> for PowerService {
    fn handle_message(&self, message: FfiEnvelopePayload<PowerCommandMessage>, _sender_id: &str) {
        debug!("Power Service: received command {:?}", message.action);
        match message.action {
            PowerCommandAction::Execute => {
                let _ = self.command_sender.send(PowerCommand::Execute(message.power_action.clone()));
            }
            PowerCommandAction::Schedule => {
                let _ = self
                    .command_sender
                    .send(PowerCommand::Schedule(message.power_action.clone(), message.delay_minutes as u64));
            }
            PowerCommandAction::Cancel => {
                let _ = self.command_sender.send(PowerCommand::Cancel);
            }
            PowerCommandAction::Refresh => {
                let _ = self.command_sender.send(PowerCommand::Refresh);
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for PowerService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        debug!("Power Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();

        match tool_name.as_str() {
            "system_power_action" => {
                let action_str = serde_json::from_str::<serde_json::Value>(&message.0.arguments.to_string())
                    .ok()
                    .and_then(|v| v.get("action").and_then(|a| a.as_str()).map(|s| s.to_string()))
                    .unwrap_or_default();
                let power_action = parse_action_from_string(&action_str);
                let _ = self.command_sender.send(PowerCommand::Execute(power_action));
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Power action triggered");
                broadcaster.broadcast_message_to_topic(response);
            }
            "system_schedule_power_action" => {
                let parsed = serde_json::from_str::<serde_json::Value>(&message.0.arguments.to_string()).ok();
                let action_str = parsed
                    .as_ref()
                    .and_then(|v| v.get("action").and_then(|a| a.as_str()).map(|s| s.to_string()))
                    .unwrap_or_default();
                let delay_minutes = parsed.as_ref().and_then(|v| v.get("delay_minutes").and_then(|a| a.as_u64())).unwrap_or(0);
                let power_action = parse_action_from_string(&action_str);
                let _ = self.command_sender.send(PowerCommand::Schedule(power_action, delay_minutes));
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Power action scheduled");
                broadcaster.broadcast_message_to_topic(response);
            }
            "system_cancel_power_action" => {
                let _ = self.command_sender.send(PowerCommand::Cancel);
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Power action cancelled");
                broadcaster.broadcast_message_to_topic(response);
            }
            "system_reboot_to_uefi" => {
                let _ = self.command_sender.send(PowerCommand::Execute(PowerAction::RebootToFirmware));
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Reboot to UEFI triggered");
                broadcaster.broadcast_message_to_topic(response);
            }
            _ => {
                let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("Unknown tool: {tool_name}"));
                broadcaster.broadcast_message_to_topic(response);
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for PowerService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, _sender_id: &str) {
        let uri = message.0.uri.to_string();
        debug!("Power Service: InvokeResourceMessage uri={}", uri);
        let broadcaster = self.get_broadcaster();

        let state = self.state_snapshot();
        let response = match uri.as_str() {
            "power://capabilities" => {
                let caps = &state.capabilities;
                let json = serde_json::json!({
                    "can_shutdown": caps.can_shutdown,
                    "can_reboot": caps.can_reboot,
                    "can_suspend": caps.can_suspend,
                    "can_hibernate": caps.can_hibernate,
                    "can_reboot_to_firmware": caps.can_reboot_to_firmware,
                    "can_lock": caps.can_lock,
                    "can_logout": caps.can_logout,
                });
                InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
            }
            "power://inhibitors" => {
                let inhibitors: Vec<serde_json::Value> = state
                    .inhibitors
                    .iter()
                    .map(|inh| {
                        serde_json::json!({
                            "process_name": inh.process_name.to_string(),
                            "reason": inh.reason.to_string(),
                            "what": inh.what.to_string(),
                            "who": inh.who.to_string(),
                        })
                    })
                    .collect();
                let json = serde_json::Value::Array(inhibitors);
                InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
            }
            "power://scheduled_actions" => {
                let json = match state.scheduled_action.as_ref() {
                    Some(sched) => serde_json::json!({
                        "action": format!("{:?}", sched.action),
                        "remaining_seconds": sched.remaining_seconds,
                        "total_delay_seconds": sched.total_delay_seconds,
                    }),
                    None => serde_json::json!(null),
                };
                InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
            }
            _ => InvokeResourceResponse::error(&message.0.correlation_id, &format!("Unknown resource: {uri}")),
        };
        broadcaster.broadcast_message_to_topic(response);
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

impl Service for PowerService {
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
            self.register_mcp_capabilities();
            std::thread::spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                    Ok(rt) => rt,
                    Err(e) => {
                        error!("Power Service: failed to create tokio runtime: {e}");
                        return;
                    }
                };
                rt.block_on(async move {
                    if let Some(receiver) = command_receiver {
                        run_power_async(meta, core_context, receiver, config, shared_state).await;
                    }
                });
            });
        }
    }
}

fn parse_action_from_string(s: &str) -> PowerAction {
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
    let payload_ptr = Box::into_raw(Box::new(status)) as *mut core::ffi::c_void;
    let envelope = FfiEnvelope {
        sender_id: stabby::string::String::from(meta.id.clone()),
        target_instance_id: stabby::string::String::from("*"),
        topic: stabby::string::String::from(PowerStatusMessage::topic()),
        type_id: PowerStatusMessage::TYPE_ID,
        payload: payload_ptr,
        destroy_payload: Some(destroy_power_status),
        clone_payload: Some(clone_power_status),
    };
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

extern "C" fn clone_power_status(ptr: *mut core::ffi::c_void) -> *mut core::ffi::c_void {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let status = unsafe { &*(ptr as *const PowerStatusMessage) };
    Box::into_raw(Box::new(status.clone())) as *mut core::ffi::c_void
}

extern "C" fn destroy_power_status(ptr: *mut core::ffi::c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut PowerStatusMessage);
        }
    }
}
