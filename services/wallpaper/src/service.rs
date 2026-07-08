use crate::config::WallpaperServiceConfig;
use crate::process::spawn_wallpaper;
use crate::process::to_monitor_processes;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
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
use smearor_swipe_launcher_plugin_api::MessageTopicBroadcaster;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::Service;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_wallpaper_model::MonitorProcess;
use smearor_wallpaper_model::TOPIC_STATUS;
use smearor_wallpaper_model::WallpaperCommandAction;
use smearor_wallpaper_model::WallpaperCommandMessage;
use smearor_wallpaper_model::WallpaperStatusMessage;
use smearor_wallpaper_model::WallpaperTheme;
use smearor_wallpaper_model::WallpaperThemeInfo;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;
use tracing::debug;
use tracing::error;

/// Internal command union for the service event loop.
pub enum WallpaperCommand {
    /// Select a theme without starting it.
    SelectTheme(String),
    /// Start the currently selected theme (stops any running theme first).
    StartSelected,
    /// Stop the currently running wallpaper process.
    StopCurrent,
    /// Refresh the status broadcast.
    Refresh,
    /// Permanently add a new theme to the configuration store.
    AddTheme(WallpaperTheme),
    /// Permanently remove a theme from the configuration store.
    RemoveTheme(String),
}

/// Internal state of the wallpaper service.
#[derive(Clone, Default)]
pub struct WallpaperState {
    /// Name of the currently running theme.
    pub current_theme: Option<String>,
    /// PIDs of active wallpaper processes per monitor.
    pub current_processes: Vec<MonitorProcess>,
    /// Index of the selected theme in the themes list.
    pub selected_theme_index: usize,
}

pub struct WallpaperService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: RwLock<WallpaperServiceConfig>,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<WallpaperCommand>,
    pub state: Arc<RwLock<WallpaperState>>,
}

impl WallpaperService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let service_config: WallpaperServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;

        let (command_sender, command_receiver) = tokio::sync::mpsc::unbounded_channel::<WallpaperCommand>();
        let meta = PluginMeta::try_from(&config)?;
        let state = Arc::new(RwLock::new(WallpaperState::default()));
        let state_clone = state.clone();
        let config_clone = service_config.clone();
        let meta_clone = meta.clone();
        let core_context_clone = core_context;

        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    error!("Wallpaper service: failed to create tokio runtime: {e}");
                    return;
                }
            };
            runtime.block_on(async move {
                run_command_loop(config_clone, command_receiver, meta_clone, core_context_clone, state_clone).await;
            });
        });

        let service = WallpaperService {
            meta,
            core_context,
            config: RwLock::new(service_config),
            command_sender,
            state,
        };
        service.register_mcp_capabilities();
        Ok(service)
    }

    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let resource_status = RegisterResourceMessage::new(
            "wallpaper://status",
            "Wallpaper Status",
            "Current wallpaper service status including running theme and configured themes.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(resource_status);

        let resource_themes = RegisterResourceMessage::new(
            "wallpaper://themes",
            "Wallpaper Themes",
            "List of all configured wallpaper themes with their configurations.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(resource_themes);

        let tool_add = RegisterToolMessage::new(
            "add_wallpaper_theme",
            "Permanently appends a new wallpaper theme to the configuration store.",
            r#"{ "type": "object", "properties": { "name": { "type": "string" }, "type": { "type": "string", "enum": ["Video", "Image", "Application"] }, "config": { "type": "object" }, "description": { "type": "string" }, "preview_image_path": { "type": "string" } }, "required": ["name", "type", "config"] }"#,
        );
        broadcaster.broadcast_message_to_topic(tool_add);

        let tool_remove = RegisterToolMessage::new(
            "remove_wallpaper_theme",
            "Deletes a wallpaper theme from the configuration store.",
            r#"{ "type": "object", "properties": { "name": { "type": "string" } }, "required": ["name"] }"#,
        );
        broadcaster.broadcast_message_to_topic(tool_remove);

        let tool_select = RegisterToolMessage::new(
            "select_wallpaper_theme",
            "Selects a wallpaper theme by name without starting it. Updates the selected_theme state.",
            r#"{ "type": "object", "properties": { "name": { "type": "string" } }, "required": ["name"] }"#,
        );
        broadcaster.broadcast_message_to_topic(tool_select);

        let tool_start = RegisterToolMessage::new(
            "start_selected_wallpaper_process",
            "Starts the currently selected wallpaper theme. Stops any running theme first, then spawns the engine process.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(tool_start);

        let tool_stop = RegisterToolMessage::new(
            "stop_current_wallpaper_process",
            "Stops the currently running wallpaper process immediately.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(tool_stop);
    }

    fn send_response<T: TypedMessage + SharedMessage + Clone>(&self, message: T, sender_id: &str) {
        let payload_ptr = Box::into_raw(Box::new(message.clone())) as *mut core::ffi::c_void;
        let sender_id_string = sender_id.to_string();
        let topic = message.topic();
        debug!("wallpaper: send_response topic={} to sender_id={}", topic, sender_id);
        let envelope = FfiEnvelope {
            sender_id: self.meta.id.clone(),
            target_instance_id: stabby::string::String::from(sender_id_string.as_str()),
            topic: stabby::string::String::from(topic),
            type_id: T::TYPE_ID,
            payload: payload_ptr,
            destroy_payload: Some(destroy_payload::<T>),
            clone_payload: Some(clone_payload::<T>),
        };
        if let Some(context) = &self.core_context {
            context.send_message(envelope);
        } else {
            debug!("wallpaper: no core_context, cannot send response");
        }
    }

    fn build_theme_infos(&self) -> Vec<WallpaperThemeInfo> {
        let config_guard = self.config.read();
        match config_guard {
            Ok(config) => config
                .themes
                .iter()
                .map(|t| WallpaperThemeInfo::new(&t.name, &t.description, &t.preview_image_path, t.wallpaper_type.clone()))
                .collect(),
            Err(e) => {
                error!("Wallpaper service: config lock poisoned: {e}");
                Vec::new()
            }
        }
    }

    fn build_status_message(&self) -> WallpaperStatusMessage {
        let state_guard = self.state.read();
        let theme_infos = self.build_theme_infos();
        let (selected_index, current_theme, current_processes) = match state_guard {
            Ok(s) => (s.selected_theme_index, s.current_theme.clone(), s.current_processes.clone()),
            Err(e) => {
                error!("Wallpaper service: state lock poisoned: {e}");
                (0, None, Vec::new())
            }
        };
        let mut msg = WallpaperStatusMessage::new(theme_infos, selected_index);
        let current_theme_stabby: stabby::option::Option<stabby::string::String> = current_theme.map(|s| s.into()).into();
        msg.current_theme = current_theme_stabby;
        let mut processes = stabby::vec::Vec::with_capacity(current_processes.len());
        for p in &current_processes {
            processes.push(p.clone());
        }
        msg.current_processes = processes;
        msg
    }

    fn broadcast_status(&self) {
        let status = self.build_status_message();
        self.broadcast_message_to_topic(status);
        debug!("Wallpaper service broadcasted status");
    }
}

impl MessageHandler<FfiEnvelopePayload<WallpaperCommandMessage>> for WallpaperService {
    fn handle_message(&self, message: FfiEnvelopePayload<WallpaperCommandMessage>, _sender_id: &str) {
        let command = match message.action {
            WallpaperCommandAction::SelectTheme => WallpaperCommand::SelectTheme(message.theme_name.to_string()),
            WallpaperCommandAction::StartSelected => WallpaperCommand::StartSelected,
            WallpaperCommandAction::StopCurrent => WallpaperCommand::StopCurrent,
            WallpaperCommandAction::Refresh => WallpaperCommand::Refresh,
        };
        let _ = self.command_sender.send(command);
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for WallpaperService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, sender_id: &str) {
        let tool_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        let arguments_str = message.0.arguments.to_string();
        debug!("wallpaper: InvokeToolMessage name={}", tool_name);

        if tool_name == "select_wallpaper_theme" {
            let args: serde_json::Value = serde_json::from_str(arguments_str.as_str()).unwrap_or_default();
            let theme_name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let _ = self.command_sender.send(WallpaperCommand::SelectTheme(theme_name.to_string()));
            let response = InvokeToolResponse::success(&correlation_id, &format!("Selected theme: {theme_name}"));
            self.send_response(response, sender_id);
        } else if tool_name == "start_selected_wallpaper_process" {
            let _ = self.command_sender.send(WallpaperCommand::StartSelected);
            let response = InvokeToolResponse::success(&correlation_id, "Start command sent");
            self.send_response(response, sender_id);
        } else if tool_name == "stop_current_wallpaper_process" {
            let _ = self.command_sender.send(WallpaperCommand::StopCurrent);
            let response = InvokeToolResponse::success(&correlation_id, "Stop command sent");
            self.send_response(response, sender_id);
        } else if tool_name == "add_wallpaper_theme" {
            let args: serde_json::Value = serde_json::from_str(arguments_str.as_str()).unwrap_or_default();
            match parse_theme_from_json(&args) {
                Ok(theme) => {
                    let theme_name = theme.name.clone();
                    let _ = self.command_sender.send(WallpaperCommand::AddTheme(theme));
                    let response = InvokeToolResponse::success(&correlation_id, &format!("Added theme: {theme_name}"));
                    self.send_response(response, sender_id);
                }
                Err(e) => {
                    let response = InvokeToolResponse::error(&correlation_id, &format!("Failed to parse theme: {e}"));
                    self.send_response(response, sender_id);
                }
            }
        } else if tool_name == "remove_wallpaper_theme" {
            let args: serde_json::Value = serde_json::from_str(arguments_str.as_str()).unwrap_or_default();
            let theme_name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if theme_name.is_empty() {
                let response = InvokeToolResponse::error(&correlation_id, "Missing required field: name");
                self.send_response(response, sender_id);
            } else {
                let _ = self.command_sender.send(WallpaperCommand::RemoveTheme(theme_name.to_string()));
                let response = InvokeToolResponse::success(&correlation_id, &format!("Removed theme: {theme_name}"));
                self.send_response(response, sender_id);
            }
        } else {
            debug!("wallpaper: unknown tool name '{}', ignoring", tool_name);
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for WallpaperService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        let uri = message.0.uri.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        debug!("wallpaper: InvokeResourceMessage uri={}", uri);

        if uri == "wallpaper://status" {
            let state_guard = self.state.read();
            let (selected_index, current_theme, current_processes) = match state_guard {
                Ok(s) => (s.selected_theme_index, s.current_theme.clone(), s.current_processes.clone()),
                Err(e) => {
                    error!("Wallpaper service: state lock poisoned: {e}");
                    (0, None, Vec::new())
                }
            };
            let config_guard = self.config.read();
            let themes: Vec<serde_json::Value> = match config_guard {
                Ok(config) => config
                    .themes
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "name": t.name,
                            "description": t.description,
                            "preview_image_path": t.preview_image_path,
                            "wallpaper_type": format!("{:?}", t.wallpaper_type),
                        })
                    })
                    .collect(),
                Err(e) => {
                    error!("Wallpaper service: config lock poisoned: {e}");
                    Vec::new()
                }
            };
            let current_processes_json: Vec<serde_json::Value> = current_processes
                .iter()
                .map(|p| serde_json::json!({ "monitor": p.monitor.to_string(), "process_id": p.process_id }))
                .collect();
            let json = serde_json::json!({
                "current_theme": current_theme,
                "current_processes": current_processes_json,
                "selected_theme_index": selected_index,
                "themes": themes,
            });
            let response = InvokeResourceResponse::success(&correlation_id, &json.to_string());
            self.send_response(response, sender_id);
        } else if uri == "wallpaper://themes" {
            let config_guard = self.config.read();
            let themes: Vec<serde_json::Value> = match config_guard {
                Ok(config) => config
                    .themes
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "name": t.name,
                            "description": t.description,
                            "preview_image_path": t.preview_image_path,
                            "wallpaper_type": format!("{:?}", t.wallpaper_type),
                        })
                    })
                    .collect(),
                Err(e) => {
                    error!("Wallpaper service: config lock poisoned: {e}");
                    Vec::new()
                }
            };
            let json = serde_json::json!({ "themes": themes });
            let response = InvokeResourceResponse::success(&correlation_id, &json.to_string());
            self.send_response(response, sender_id);
        } else {
            let response = InvokeResourceResponse::error(&correlation_id, &format!("Unknown resource uri: {uri}"));
            self.send_response(response, sender_id);
        }
    }
}

impl MessageBroadcaster for WallpaperService {}

impl MessageTopicBroadcaster<WallpaperStatusMessage> for WallpaperService {}

impl PluginMetaGetter for WallpaperService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for WallpaperService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Service for WallpaperService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                debug!("wallpaper: on_message topic={} type_id={}", envelope.topic, envelope.type_id);
                if envelope.type_id == FfiEnvelopePayload::<WallpaperCommandMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<WallpaperCommandMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeResourceMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

async fn run_command_loop(
    config: WallpaperServiceConfig,
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<WallpaperCommand>,
    meta: PluginMeta,
    core_context: Option<FfiCoreContext>,
    state: Arc<RwLock<WallpaperState>>,
) {
    // Auto-start the default theme if configured
    if config.auto_start && !config.default_theme.is_empty() {
        select_theme(&state, &config.default_theme, &config);
        start_selected_theme(&config, &meta, &core_context, &state).await;
        broadcast_status(&meta, &core_context, &config, &state);
    }

    while let Some(command) = command_receiver.recv().await {
        match command {
            WallpaperCommand::SelectTheme(name) => {
                select_theme(&state, &name, &config);
                broadcast_status(&meta, &core_context, &config, &state);
            }
            WallpaperCommand::StartSelected => {
                start_selected_theme(&config, &meta, &core_context, &state).await;
                broadcast_status(&meta, &core_context, &config, &state);
            }
            WallpaperCommand::StopCurrent => {
                stop_current_theme(&config, &meta, &core_context, &state).await;
                broadcast_status(&meta, &core_context, &config, &state);
            }
            WallpaperCommand::Refresh => {
                broadcast_status(&meta, &core_context, &config, &state);
            }
            WallpaperCommand::AddTheme(theme) => {
                add_theme_to_config(&config.config_path, &theme);
                broadcast_status(&meta, &core_context, &config, &state);
            }
            WallpaperCommand::RemoveTheme(name) => {
                remove_theme_from_config(&config.config_path, &name);
                let need_stop = {
                    if let Ok(s) = state.read() {
                        s.current_theme.as_deref() == Some(&name)
                    } else {
                        false
                    }
                };
                if need_stop {
                    stop_current_theme(&config, &meta, &core_context, &state).await;
                }
                broadcast_status(&meta, &core_context, &config, &state);
            }
        }
    }
}

async fn start_selected_theme(config: &WallpaperServiceConfig, meta: &PluginMeta, core_context: &Option<FfiCoreContext>, state: &Arc<RwLock<WallpaperState>>) {
    // Stop current wallpaper first
    stop_current_theme(config, meta, core_context, state).await;

    let theme_index = match state.read() {
        Ok(s) => s.selected_theme_index,
        Err(e) => {
            error!("Wallpaper service: state lock poisoned: {e}");
            return;
        }
    };

    let Some(theme) = config.themes.get(theme_index) else {
        debug!("Wallpaper service: no theme at index {}", theme_index);
        return;
    };

    debug!("Wallpaper service: starting theme '{}'", theme.name);
    let monitor_pids = spawn_wallpaper(theme);

    {
        if let Ok(mut s) = state.write() {
            s.current_processes = to_monitor_processes(&monitor_pids);
            if !monitor_pids.is_empty() {
                s.current_theme = Some(theme.name.clone());
            } else {
                s.current_theme = None;
            }
        }
    }

    broadcast_status(meta, core_context, config, state);
}

async fn stop_current_theme(config: &WallpaperServiceConfig, meta: &PluginMeta, core_context: &Option<FfiCoreContext>, state: &Arc<RwLock<WallpaperState>>) {
    let pids: Vec<u32> = match state.read() {
        Ok(s) => s.current_processes.iter().map(|p| p.process_id).collect(),
        Err(e) => {
            error!("Wallpaper service: state lock poisoned: {e}");
            return;
        }
    };

    if pids.is_empty() {
        debug!("Wallpaper service: no processes to stop");
        return;
    }

    // Send SIGTERM to all unique PIDs
    let unique_pids: HashSet<u32> = pids.into_iter().collect();
    for pid in &unique_pids {
        if *pid > 0 {
            let _ = kill(Pid::from_raw(*pid as i32), Signal::SIGTERM);
            debug!("Wallpaper service: sent SIGTERM to PID {}", pid);
        }
    }

    // Poll every 100ms to check if processes have exited
    if !unique_pids.is_empty() {
        let poll_interval = Duration::from_millis(100);
        let deadline = tokio::time::Instant::now() + Duration::from_millis(config.kill_grace_period_ms);
        loop {
            tokio::time::sleep(poll_interval).await;

            let alive: Vec<u32> = unique_pids
                .iter()
                .copied()
                .filter(|pid| *pid > 0 && kill(Pid::from_raw(*pid as i32), None).is_ok())
                .collect();

            if alive.is_empty() || tokio::time::Instant::now() >= deadline {
                for pid in &alive {
                    let _ = kill(Pid::from_raw(*pid as i32), Signal::SIGKILL);
                    debug!("Wallpaper service: sent SIGKILL to PID {}", pid);
                }
                break;
            }
        }
    }

    {
        if let Ok(mut s) = state.write() {
            s.current_processes.clear();
            s.current_theme = None;
        }
    }

    broadcast_status(meta, core_context, config, state);
}

fn select_theme(state: &Arc<RwLock<WallpaperState>>, name: &str, config: &WallpaperServiceConfig) {
    if let Ok(mut state) = state.write() {
        if let Some(index) = config.themes.iter().position(|t| t.name == name) {
            state.selected_theme_index = index;
            debug!("Wallpaper service: selected theme '{}'", name);
        } else {
            debug!("Wallpaper service: theme '{}' not found", name);
        }
    }
}

fn add_theme_to_config(config_path: &str, theme: &WallpaperTheme) {
    let path = std::path::Path::new(config_path);
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let mut doc: toml::Value = toml::from_str(&content).unwrap_or(toml::Value::Table(toml::map::Map::new()));
            if let toml::Value::Table(ref mut table) = doc {
                if let Some(toml::Value::Array(arr)) = table.get_mut("themes") {
                    if let Ok(theme_toml) = toml::to_string(theme) {
                        if let Ok(parsed) = toml::from_str::<toml::Value>(&format!("[[themes]]\n{theme_toml}")) {
                            if let toml::Value::Array(parsed_arr) = parsed {
                                for item in parsed_arr {
                                    arr.push(item);
                                }
                            }
                        }
                    }
                }
            }
            let _ = std::fs::write(path, toml::to_string(&doc).unwrap_or_default());
        }
        Err(_) => {
            let mut content = String::new();
            if let Ok(theme_toml) = toml::to_string(theme) {
                content.push_str("[[themes]]\n");
                content.push_str(&theme_toml);
            }
            let _ = std::fs::write(path, content);
        }
    }
    debug!("Wallpaper service: added theme '{}' to {}", theme.name, config_path);
}

fn remove_theme_from_config(config_path: &str, name: &str) {
    let path = std::path::Path::new(config_path);
    if let Ok(content) = std::fs::read_to_string(path) {
        let mut doc: toml::Value = toml::from_str(&content).unwrap_or(toml::Value::Table(toml::map::Map::new()));
        if let toml::Value::Table(ref mut table) = doc {
            if let Some(toml::Value::Array(arr)) = table.get_mut("themes") {
                arr.retain(|t| t.get("name").and_then(|n| n.as_str()).map(|n| n != name).unwrap_or(true));
            }
        }
        let _ = std::fs::write(path, toml::to_string(&doc).unwrap_or_default());
    }
    debug!("Wallpaper service: removed theme '{}' from {}", name, config_path);
}

fn parse_theme_from_json(args: &serde_json::Value) -> Result<WallpaperTheme, String> {
    let name = args.get("name").and_then(|v| v.as_str()).ok_or("Missing required field: name")?;
    let type_str = args.get("type").and_then(|v| v.as_str()).ok_or("Missing required field: type")?;
    let description = args.get("description").and_then(|v| v.as_str()).unwrap_or("");
    let preview_image_path = args.get("preview_image_path").and_then(|v| v.as_str()).unwrap_or("");
    let config_json = args.get("config").ok_or("Missing required field: config")?;

    let wallpaper_type = match type_str {
        "Video" => smearor_wallpaper_model::WallpaperType::Video,
        "Image" => smearor_wallpaper_model::WallpaperType::Image,
        "Application" => smearor_wallpaper_model::WallpaperType::Application,
        other => return Err(format!("Unknown wallpaper type: {other}")),
    };

    let theme_config = match wallpaper_type {
        smearor_wallpaper_model::WallpaperType::Video => {
            let video_config: smearor_wallpaper_model::VideoConfig =
                serde_json::from_value(config_json.clone()).map_err(|e| format!("Invalid VideoConfig: {e}"))?;
            smearor_wallpaper_model::WallpaperThemeConfig::Video(video_config)
        }
        smearor_wallpaper_model::WallpaperType::Image => {
            let image_config: smearor_wallpaper_model::ImageConfig =
                serde_json::from_value(config_json.clone()).map_err(|e| format!("Invalid ImageConfig: {e}"))?;
            smearor_wallpaper_model::WallpaperThemeConfig::Image(image_config)
        }
        smearor_wallpaper_model::WallpaperType::Application => {
            let app_config: smearor_wallpaper_model::AppConfig = serde_json::from_value(config_json.clone()).map_err(|e| format!("Invalid AppConfig: {e}"))?;
            smearor_wallpaper_model::WallpaperThemeConfig::Application(app_config)
        }
    };

    Ok(WallpaperTheme {
        name: name.to_string(),
        description: description.to_string(),
        preview_image_path: preview_image_path.to_string(),
        wallpaper_type,
        config: theme_config,
    })
}

fn broadcast_status(meta: &PluginMeta, core_context: &Option<FfiCoreContext>, config: &WallpaperServiceConfig, state: &Arc<RwLock<WallpaperState>>) {
    let (selected_index, current_theme, current_processes) = match state.read() {
        Ok(s) => (s.selected_theme_index, s.current_theme.clone(), s.current_processes.clone()),
        Err(e) => {
            error!("Wallpaper service: state lock poisoned: {e}");
            (0, None, Vec::new())
        }
    };
    let theme_infos: Vec<WallpaperThemeInfo> = config
        .themes
        .iter()
        .map(|t| WallpaperThemeInfo::new(&t.name, &t.description, &t.preview_image_path, t.wallpaper_type.clone()))
        .collect();
    let mut msg = WallpaperStatusMessage::new(theme_infos, selected_index);
    let current_theme_stabby: stabby::option::Option<stabby::string::String> = current_theme.map(|name| name.into()).into();
    msg.current_theme = current_theme_stabby;
    let mut processes = stabby::vec::Vec::with_capacity(current_processes.len());
    for p in &current_processes {
        processes.push(p.clone());
    }
    msg.current_processes = processes;

    let payload_ptr = Box::into_raw(Box::new(msg.clone())) as *mut core::ffi::c_void;
    let envelope = FfiEnvelope {
        sender_id: meta.id.clone(),
        target_instance_id: stabby::string::String::from(""),
        topic: stabby::string::String::from(TOPIC_STATUS),
        type_id: WallpaperStatusMessage::TYPE_ID,
        payload: payload_ptr,
        destroy_payload: Some(destroy_payload::<WallpaperStatusMessage>),
        clone_payload: Some(clone_payload::<WallpaperStatusMessage>),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallpaper_state_default() {
        let state = WallpaperState::default();
        assert!(state.current_theme.is_none());
        assert!(state.current_processes.is_empty());
        assert_eq!(state.selected_theme_index, 0);
    }
}
