use crate::config::launcher::SwipeLauncherConfig;
use crate::config::services::ServicesConfig;
use crate::config::watcher::ConfigWatcher;
use crate::context::GLOBAL_JSON_CONVERTER_REGISTRY;
use crate::context::initialize_global_json_converter_registry;
use crate::css::CssWatcher;
use crate::css::create_css_provider;
use crate::display::AreaSize;
use crate::instance::LauncherInstance;
use crate::instance::PersistedInstance;
use crate::instance::get_instances_state_path;
use crate::instance::read_instances_state;
use crate::instance::write_instances_state;
use crate::mcp::McpRegistry;
use crate::mcp::McpResponseTracker;
use crate::messages::try_convert_string_to_typed_envelope;
use crate::service::ServiceManager;
use async_channel::unbounded;
use gtk4::Application;
use gtk4::gdk::Display;
use gtk4::gdk::Monitor;
use gtk4::gio;
use gtk4::gio::prelude::*;
use gtk4::glib::MainContext;
use gtk4::prelude::*;
use serde_json;
use smearor_model_compositor::TOPIC_CREATE_WORKSPACE;
use smearor_model_compositor::TOPIC_SWITCH_WORKSPACE;
use smearor_model_compositor::TOPIC_WORKSPACE_CHANGED;
use smearor_model_compositor::TOPIC_WORKSPACE_LIFECYCLE;
use smearor_model_compositor::TOPIC_WORKSPACE_SNAPSHOT;
use smearor_model_compositor::TOPIC_WORKSPACE_SNAPSHOT_REQUEST;
use smearor_model_instance_control::InstanceLoadMessage;
use smearor_model_instance_control::InstanceReloadMessage;
use smearor_model_instance_control::InstanceStartMessage;
use smearor_model_instance_control::InstanceStatusMessage;
use smearor_model_instance_control::InstanceStopMessage;
use smearor_model_instance_control::InstanceType as ModelInstanceType;
use smearor_model_instance_control::InstanceUnloadMessage;
use smearor_model_instance_control::LauncherInstanceLifecycle;
use smearor_model_instance_control::TOPIC_CORE_INSTANCE_LOAD;
use smearor_model_instance_control::TOPIC_CORE_INSTANCE_RELOAD;
use smearor_model_instance_control::TOPIC_CORE_INSTANCE_START;
use smearor_model_instance_control::TOPIC_CORE_INSTANCE_STATUS;
use smearor_model_instance_control::TOPIC_CORE_INSTANCE_STOP;
use smearor_model_instance_control::TOPIC_CORE_INSTANCE_UNLOAD;
use smearor_model_macropad::MacroPadCommand;
use smearor_model_macropad::MacroPadCommandMessage;
use smearor_model_macropad::MacroPadConnectionStatus;
use smearor_model_macropad::MacroPadInputMessage;
use smearor_model_macropad::TOPIC_MACROPAD_COMMAND;
use smearor_model_macropad::TOPIC_MACROPAD_CONNECTION;
use smearor_model_macropad::TOPIC_MACROPAD_INPUT;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokePromptResponse;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_model_widget::TOPIC_WIDGET_UPDATE;
use smearor_model_widget::WidgetUpdateMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::JsonConverterRegistry;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::box_payload;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::unbounded_channel;
use tracing::debug;
use tracing::error;
use tracing::trace;

use smearor_swipe_launcher_plugin_api::default_clone_payload;
use smearor_swipe_launcher_plugin_api::default_destroy_payload;

/// Duration threshold for MacroPad longpress detection (500ms).
const MACROPAD_LONGPRESS_THRESHOLD: Duration = Duration::from_millis(500);

/// Time window for MacroPad double-press detection (300ms).
const MACROPAD_DOUBLE_PRESS_WINDOW: Duration = Duration::from_millis(300);

/// Time window for compound longpress detection — buttons must be pressed
/// within this duration of each other to qualify as a compound press.
const MACROPAD_COMPOUND_PRESS_WINDOW: Duration = Duration::from_millis(100);

/// Extract a rectangular slice from an RGBA pixel buffer.
///
/// Used to split a 2D span group's combined render into individual button
/// images. Crops the region (x_offset, y_offset) to (slice_width, slice_height)
/// from the source buffer.
fn extract_grid_slice(pixels: &[u8], src_width: u32, src_height: u32, x_offset: u32, y_offset: u32, slice_width: u32, slice_height: u32) -> Vec<u8> {
    let mut result = Vec::with_capacity((slice_width * slice_height * 4) as usize);
    for y in y_offset..(y_offset + slice_height) {
        let start = ((y * src_width + x_offset) * 4) as usize;
        let end = start + (slice_width * 4) as usize;
        result.extend_from_slice(&pixels[start..end]);
    }
    result
}

/// Extract a horizontal slice from an RGBA pixel buffer.
///
/// Used to split a span-group's combined render into individual button images.
/// Delegates to `extract_grid_slice` with full height and zero y-offset.
fn extract_horizontal_slice(pixels: &[u8], src_width: u32, src_height: u32, x_offset: u32, slice_width: u32) -> Vec<u8> {
    extract_grid_slice(pixels, src_width, src_height, x_offset, 0, slice_width, src_height)
}

/// Host that manages all launcher instances in a single process.
///
/// Owns the single `gtk4::Application`, the shared `ServiceManager`,
/// the central message broker, and a collection of `LauncherInstance`s.
#[derive(Clone)]
pub struct LauncherHost {
    pub(crate) gtk_app: Application,
    pub(crate) service_manager: Arc<ServiceManager>,
    pub(crate) broker_sender: UnboundedSender<FfiEnvelope>,
    pub(crate) broker_receiver: Arc<Mutex<Option<UnboundedReceiver<FfiEnvelope>>>>,
    pub(crate) instances: Arc<Mutex<HashMap<String, LauncherInstance>>>,
    pub(crate) mcp_registry: McpRegistry,
    pub(crate) mcp_response_tracker: McpResponseTracker,
    pub(crate) mcp_command_sender: Arc<Mutex<Option<async_channel::Sender<smearor_mcp_server::McpCommand>>>>,
    pub(crate) hotplug_last_event: Arc<Mutex<Option<Instant>>>,
    pub(crate) services_config: Arc<Mutex<Option<ServicesConfig>>>,
    pub(crate) web_server: Arc<Mutex<Option<crate::web::WebServer>>>,
    /// Tracks MacroPad button press start times for longpress detection.
    /// Key: (instance_id, button_index), Value: press start Instant.
    pub(crate) macropad_press_times: Arc<Mutex<HashMap<(String, u8), Instant>>>,
    /// Tracks pending clicks for double-press detection.
    /// Key: (instance_id, button_index), Value: time the first click was recorded.
    /// If a second click arrives within MACROPAD_DOUBLE_PRESS_WINDOW, a
    /// "double_press" action is dispatched instead of "click".
    pub(crate) macropad_pending_clicks: Arc<Mutex<HashMap<(String, u8), Instant>>>,
    /// Tracks compound presses for span-group longpress detection.
    /// Key: (instance_id, span_group), Value: list of (button_index, press_start).
    /// When 2+ buttons in the same span group are pressed within
    /// MACROPAD_COMPOUND_PRESS_WINDOW and held >= MACROPAD_LONGPRESS_THRESHOLD,
    /// a "compound_longpress" action is dispatched to all group members.
    pub(crate) macropad_compound_presses: Arc<Mutex<HashMap<(String, String), Vec<(u8, Instant)>>>>,
    /// Tracks which span groups had a compound longpress actually dispatched.
    /// Key: (instance_id, span_group). Cleared on button release.
    pub(crate) macropad_compound_dispatched: Arc<Mutex<HashMap<(String, String), ()>>>,
    /// CSS file watcher for global and per-instance CSS hot-reload.
    pub(crate) css_watcher: Arc<CssWatcher>,
    /// Config file watcher for TOML hot-reload.
    pub(crate) config_watcher: Arc<ConfigWatcher>,
    /// Registry mapping `auto_start_topic` / `auto_stop_topic` strings to instance IDs and actions.
    /// Used for event-driven lifecycle control via the message broker.
    pub(crate) topic_instance_registry: Arc<Mutex<HashMap<String, (String, TopicAction)>>>,
}

/// Action to perform when a message is received on an `auto_start_topic` or `auto_stop_topic`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopicAction {
    /// Start the associated instance.
    Start,
    /// Stop the associated instance.
    Stop,
}

impl LauncherHost {
    pub fn new(gtk_app: Application) -> Self {
        let (broker_sender, broker_receiver) = unbounded_channel::<FfiEnvelope>();
        let service_manager = Arc::new(ServiceManager::new(broker_sender.clone()));
        let global_json_converter_registry = Arc::new(JsonConverterRegistry::new());
        let _ = initialize_global_json_converter_registry(global_json_converter_registry);

        LauncherHost {
            gtk_app,
            service_manager,
            broker_sender,
            broker_receiver: Arc::new(Mutex::new(Some(broker_receiver))),
            instances: Arc::new(Mutex::new(HashMap::new())),
            mcp_registry: McpRegistry::new(),
            mcp_response_tracker: McpResponseTracker::new(),
            mcp_command_sender: Arc::new(Mutex::new(None)),
            hotplug_last_event: Arc::new(Mutex::new(None)),
            services_config: Arc::new(Mutex::new(None)),
            web_server: Arc::new(Mutex::new(None)),
            macropad_press_times: Arc::new(Mutex::new(HashMap::new())),
            macropad_pending_clicks: Arc::new(Mutex::new(HashMap::new())),
            macropad_compound_presses: Arc::new(Mutex::new(HashMap::new())),
            macropad_compound_dispatched: Arc::new(Mutex::new(HashMap::new())),
            css_watcher: Arc::new(CssWatcher::new()),
            config_watcher: Arc::new(ConfigWatcher::new()),
            topic_instance_registry: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Sets the MCP command sender used to dispatch core tool/resource/prompt
    /// invocations from the broker router.
    pub fn set_mcp_command_sender(&self, sender: async_channel::Sender<smearor_mcp_server::McpCommand>) {
        if let Ok(mut guard) = self.mcp_command_sender.lock() {
            *guard = Some(sender);
        }
    }

    pub fn create_instance(&self, instance_id: String, config: SwipeLauncherConfig, instance_type: crate::instance::InstanceType) {
        let instance = LauncherInstance::new(config, instance_id.clone(), instance_type, self.broker_sender.clone());
        instance.load_plugins();
        if let Ok(mut instances) = self.instances.lock() {
            instances.insert(instance_id.clone(), instance);
        }

        // Register with WebSocket manager if this is a Web instance and the server is running
        if instance_type == crate::instance::InstanceType::Web {
            if let Ok(ws_guard) = self.web_server.lock() {
                if let Some(ref web_server) = *ws_guard {
                    web_server.register_instance(&instance_id);
                }
            }
        }
    }

    /// Start the embedded web server with the given configuration.
    pub fn start_web_server(&self, config: crate::web::WebServerConfig) {
        let mcp_command_sender = self.mcp_command_sender.lock().ok().and_then(|g| g.clone()).unwrap_or_else(|| {
            let (_tx, _rx) = async_channel::unbounded::<smearor_mcp_server::McpCommand>();
            _tx
        });
        let web_server = crate::web::WebServer::new(config, self.instances.clone(), self.broker_sender.clone(), mcp_command_sender);

        // Register all existing Web instances for WebSocket updates
        if let Ok(instances) = self.instances.lock() {
            for instance in instances.values() {
                if instance.instance_type == crate::instance::InstanceType::Web {
                    web_server.register_instance(&instance.instance_id);
                }
            }
        }

        web_server.start();
        if let Ok(mut guard) = self.web_server.lock() {
            *guard = Some(web_server);
        }
    }

    pub fn load_services(&self, services_config: &ServicesConfig) {
        let discovery_service = crate::config::discovery::ConfigDiscoveryService::new();
        let wallpaper_config_path = discovery_service.discover_wallpaper_config();

        for service_entry in &services_config.services {
            trace!("Loading service {}", service_entry.id);
            let mut service_config = services_config.plugin_config(&service_entry.id);

            if service_entry.id == "wallpaper" {
                if let Some(ref path) = wallpaper_config_path {
                    let path_str = path.display().to_string();
                    let needs_inject = service_config.config.get("config_path").and_then(|v| v.as_str()).is_none_or(|s| s.is_empty());
                    if needs_inject {
                        trace!("Injecting wallpaper config_path: {}", path_str);
                        if let Some(obj) = service_config.config.as_object_mut() {
                            obj.insert("config_path".to_string(), serde_json::Value::String(path_str));
                        }
                    }
                }
            }

            trace!("Service config: {service_config:?}");
            if let Err(e) = self.service_manager.load_service(&service_entry, service_config) {
                error!("Failed to load service {}: {}", service_entry.id, e);
            }
        }
        debug!("Successfully loaded {} services", self.service_manager.services.len());

        // Defer tool replay so the broker has time to process pending
        // RegisterToolMessage broadcasts from services that just started.
        let self_clone = self.clone();
        gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(500), move || {
            self_clone.replay_registered_tools_to_services();
        });
    }

    /// Replays all tools currently in the McpRegistry to the voice_assistant
    /// service. This ensures voice_assistant, which is loaded after other
    /// services, still receives all registrations and can build a complete
    /// tool catalog.
    fn replay_registered_tools_to_services(&self) {
        let tools = self.mcp_registry.list_tools();
        if tools.is_empty() {
            return;
        }
        debug!("Replaying {} registered tools to voice_assistant", tools.len());
        for tool in tools {
            let message = RegisterToolMessage::new(&tool.name, &tool.description, &tool.input_schema.to_string());
            let payload_ptr = box_payload(message);
            let envelope = FfiEnvelope::builder()
                .sender_id(tool.plugin_id.as_str())
                .target_instance_id("")
                .topic(RegisterToolMessage::topic())
                .type_id(RegisterToolMessage::TYPE_ID)
                .payload(payload_ptr)
                .destroy_payload(Some(default_destroy_payload))
                .clone_payload(Some(default_clone_payload::<RegisterToolMessage>))
                .build();
            if let Err(error) = self.broker_sender.send(envelope) {
                error!("Failed to replay tool registration {}: {}", tool.name, error);
            }
        }
    }

    pub fn build_ui(&self) -> miette::Result<()> {
        self.calculate_coordinated_sizes();

        let self_clone = self.clone();

        self.gtk_app.connect_activate(move |app| {
            trace!("GTK application activated");

            // Register GResources first so CSS @font-face can resolve resource:// URLs
            match gio::resources_register_include!("compiled.gresource") {
                Ok(_) => {
                    // IconTheme::default().add_resource_path("/io/smearor/icons");
                }
                Err(e) => {
                    error!("Failed to register gresource: {e}");
                }
            }

            // Register Nerd Font icons as GTK GResource for native icon loading
            if let Err(e) = nerd_gtk_icons::register_icons() {
                error!("Failed to register nerd font icons: {e}");
            }
            if let Some(display) = gtk4::gdk::Display::default() {
                gtk4::IconTheme::for_display(&display).add_resource_path("/com/nerd/icons");
            }

            create_css_provider();

            let instances = if let Ok(instances) = self_clone.instances.lock() {
                instances
                    .values()
                    .filter(|i| i.instance_type == crate::instance::InstanceType::Gtk)
                    .map(|i| (i.instance_id.clone(), i.build_window(app)))
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };

            for (instance_id, result) in instances {
                match result {
                    Ok(window) => {
                        if let Ok(instances) = self_clone.instances.lock() {
                            if let Some(instance) = instances.get(&instance_id) {
                                if let Ok(mut w) = instance.window.lock() {
                                    *w = Some(window);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to build window for instance {}: {}", instance_id, e);
                    }
                }
            }

            // Register monitor hotplug signal handlers via GListModel
            if let Some(display) = Display::default() {
                let monitors_list = display.monitors();
                let hotplug_clone = self_clone.clone();
                monitors_list.connect_items_changed(move |_list, _position, _removed, _added| {
                    if hotplug_clone.should_process_hotplug() {
                        debug!("Monitor configuration changed (added/removed) — recalculating coordinated sizes");
                        hotplug_clone.calculate_coordinated_sizes();
                        hotplug_clone.rebuild_windows();
                    }
                });
            }
        });

        self.start_broker_loop()?;

        Ok(())
    }

    /// Calculate coordinated window sizes for instances on the same monitor.
    ///
    /// Long-side launchers (0° / 180°) take full monitor width and have priority.
    /// Short-side launchers (90° / 270°) shrink their height so they do not
    /// overlap into the reserved space of long-side launchers.
    ///
    /// Instances are grouped by their configured monitor index so that
    /// coordination only applies to instances sharing the same monitor.
    pub fn calculate_coordinated_sizes(&self) {
        let Some(display) = Display::default() else {
            return;
        };
        let monitors = display.monitors();

        let Ok(instances) = self.instances.lock() else {
            return;
        };

        let mut monitor_groups: HashMap<u32, Vec<&LauncherInstance>> = HashMap::new();
        for instance in instances.values() {
            let monitor_index = instance.config.launcher.layer.monitor.unwrap_or(0);
            monitor_groups.entry(monitor_index).or_default().push(instance);
        }

        for (monitor_index, group) in &monitor_groups {
            let Some(monitor) = monitors.item(*monitor_index).and_then(|m| m.downcast::<Monitor>().ok()) else {
                continue;
            };
            let geometry = monitor.geometry();
            let monitor_height = geometry.height();

            let mut long_side_height_sum = 0_i32;
            for instance in group {
                let rotation = instance.config.launcher.rotation.rotation().to_degrees();
                let is_long_side = (rotation - 0.0).abs() < 0.1 || (rotation - 180.0).abs() < 0.1;
                if is_long_side {
                    let height = instance.config.launcher.layer.exclusive_zone().unwrap_or(150);
                    long_side_height_sum += height;
                }
            }

            for instance in group {
                let rotation = instance.config.launcher.rotation.rotation().to_degrees();
                let is_short_side = (rotation - 90.0).abs() < 0.1 || (rotation - 270.0).abs() < 0.1;
                if is_short_side {
                    let default_size = instance.config.launcher.layer.exclusive_zone().unwrap_or(150);
                    let adjusted_height = (monitor_height - long_side_height_sum).max(default_size);
                    let coordinated_size = AreaSize::new(default_size, adjusted_height);
                    if let Ok(mut size) = instance.coordinated_size.lock() {
                        *size = Some(coordinated_size);
                    }
                    debug!(
                        "Instance {} short-side coordinated size: {}x{} (monitor {})",
                        instance.instance_id, coordinated_size.width, coordinated_size.height, monitor_index
                    );
                }
            }
        }
    }

    /// Debounce hotplug events to avoid excessive rebuilds during display negotiation.
    /// Returns true if the event should be processed, false if it was suppressed.
    fn should_process_hotplug(&self) -> bool {
        const HOTPLUG_DEBOUNCE: Duration = Duration::from_millis(500);
        let Ok(mut last) = self.hotplug_last_event.lock() else {
            return false;
        };
        let now = Instant::now();
        if let Some(last_time) = *last {
            if now.duration_since(last_time) < HOTPLUG_DEBOUNCE {
                return false;
            }
        }
        *last = Some(now);
        true
    }

    /// Rebuild all launcher windows after a monitor configuration change.
    /// Closes existing windows and re-creates them with updated monitor assignment.
    pub fn rebuild_windows(&self) {
        let Ok(instances) = self.instances.lock() else {
            return;
        };

        for instance in instances.values() {
            if let Ok(mut window_guard) = instance.window.lock() {
                if let Some(window) = window_guard.take() {
                    window.close();
                }
            }
        }

        for instance in instances.values() {
            match instance.build_window(&self.gtk_app) {
                Ok(window) => {
                    if let Ok(mut window_guard) = instance.window.lock() {
                        *window_guard = Some(window);
                    }
                }
                Err(error) => {
                    error!("Failed to rebuild window for instance {}: {}", instance.instance_id, error);
                }
            }
        }
    }

    fn start_broker_loop(&self) -> miette::Result<()> {
        let Ok(mut receiver_guard) = self.broker_receiver.lock() else {
            return Err(miette::miette!("Failed to lock broker receiver"));
        };
        let Some(mut receiver) = receiver_guard.take() else {
            return Err(miette::miette!("Broker receiver already taken"));
        };

        let (async_sender, async_receiver) = unbounded::<FfiEnvelope>();
        let main_context = MainContext::default();

        tokio::spawn(async move {
            while let Some(envelope) = receiver.recv().await {
                if async_sender.try_send(envelope).is_err() {
                    break;
                }
            }
            error!("Central broker receive loop exited");
        });

        let self_clone = self.clone();
        main_context.spawn_local(async move {
            while let Ok(envelope) = async_receiver.recv().await {
                self_clone.route_message(envelope);
            }
            error!("Central broker receive loop exited");
        });

        Ok(())
    }

    /// Check if a specific trigger type (e.g. "hold_topic", "double_press_topic")
    /// is configured for the plugin at `button_index` in the given instance's
    /// currently visible area.
    ///
    /// Uses the physical-to-logical button map built during rendering to
    /// account for 2D span group alignment shifts.
    fn is_trigger_configured(&self, instance_id: &str, button_index: u8, trigger_field: &str) -> bool {
        if let Ok(instances) = self.instances.lock() {
            if let Some(instance) = instances.get(instance_id) {
                let plugin_id = if let Ok(map) = instance.button_map.lock() {
                    map.as_ref().and_then(|m| m.get(button_index as usize).and_then(|id| id.clone()))
                } else {
                    return false;
                };
                let Some(plugin_id) = plugin_id else {
                    return false;
                };
                if let Some(config) = instance.config.get_plugin_config(&plugin_id) {
                    return config.get(trigger_field).and_then(|v| v.as_str()).is_some();
                }
            }
        }
        false
    }

    /// Get the span group name and all physical button indices in that group
    /// for the given physical button. Returns `None` if the button is not
    /// part of a span group.
    ///
    /// Uses the physical-to-logical button map built during rendering.
    fn get_span_group_for_button(&self, instance_id: &str, button_index: u8) -> Option<(String, Vec<u8>)> {
        if let Ok(instances) = self.instances.lock() {
            if let Some(instance) = instances.get(instance_id) {
                let button_map = if let Ok(map) = instance.button_map.lock() {
                    map.clone().unwrap_or_default()
                } else {
                    return None;
                };
                let plugin_id = button_map.get(button_index as usize).and_then(|id| id.as_ref().map(|s| s.as_str()))?;

                let entries = if let Ok(area_manager) = instance.area_manager.lock() {
                    area_manager.visible_area_plugin_entries()
                } else {
                    return None;
                };
                let target_entry = entries.iter().find(|e| e.id == plugin_id)?;
                let span_group = target_entry.span_group.clone()?;

                let group_plugin_ids: std::collections::HashSet<&str> = entries
                    .iter()
                    .filter(|e| e.span_group.as_ref() == Some(&span_group))
                    .map(|e| e.id.as_str())
                    .collect();

                let mut group_buttons: Vec<u8> = button_map
                    .iter()
                    .enumerate()
                    .filter(|(_, id)| id.as_ref().map_or(false, |pid| group_plugin_ids.contains(pid.as_str())))
                    .map(|(i, _)| i as u8)
                    .collect();
                group_buttons.sort();

                return Some((span_group, group_buttons));
            }
        }
        None
    }

    /// Dispatch a `InvokeToolMessage` with the given action to the plugin at
    /// `button_index` in the instance's currently visible area.
    ///
    /// Uses the physical-to-logical button map built during rendering.
    fn dispatch_macropad_action(&self, instance_id: &str, button_index: u8, action: &str) {
        if let Ok(instances) = self.instances.lock() {
            if let Some(instance) = instances.get(instance_id) {
                let plugin_id = if let Ok(map) = instance.button_map.lock() {
                    map.as_ref().and_then(|m| m.get(button_index as usize).and_then(|id| id.clone()))
                } else {
                    None
                };

                if let Some(plugin_id) = plugin_id {
                    let tool_name = format!("button_{}", plugin_id);
                    let correlation_id = format!("macropad-{}-{}", instance_id, button_index);
                    let arguments = format!(r#"{{"action":"{}"}}"#, action);
                    let invoke_msg = InvokeToolMessage::new(&tool_name, &correlation_id, &arguments);
                    let payload_ptr = box_payload(invoke_msg);
                    let invoke_envelope = FfiEnvelope::builder()
                        .sender_id(instance_id)
                        .target_instance_id("*")
                        .topic(InvokeToolMessage::topic())
                        .type_id(FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID)
                        .payload(payload_ptr)
                        .destroy_payload(Some(default_destroy_payload))
                        .clone_payload(Some(default_clone_payload::<InvokeToolMessage>))
                        .build();
                    instance.handle_message(invoke_envelope);
                    debug!("MacroPad: dispatched {} to plugin '{}' for instance '{}'", action, plugin_id, instance_id);
                } else {
                    debug!("MacroPad: no plugin at physical button {} for instance '{}'", button_index, instance_id);
                }
            } else {
                debug!("MacroPad: instance '{}' not found", instance_id);
            }
        }
    }

    fn route_message(&self, envelope: FfiEnvelope) {
        let mut target = envelope.target_instance_id.to_string();
        let topic = envelope.topic.to_string();
        trace!(
            "route_message: topic={} target={} ServiceManager ptr={:p} count={}",
            topic,
            target,
            self.service_manager.as_ref(),
            self.service_manager.services.len()
        );

        // Global MCP registration messages are routed to the shared registry
        // and forwarded only to the voice_assistant service, which builds and
        // maintains the tool catalog used for ReAct tool selection. Other
        // services do not handle RegisterToolMessage and would log noise.
        if topic == smearor_model_mcp::TOPIC_MCP_REGISTER_TOOL {
            MessageHandler::<FfiEnvelopePayload<RegisterToolMessage>>::handle_envelope_message(&self.mcp_registry, &envelope);
            trace!("Plugin registered a new MCP tool, list_changed notification deferred to SDK runtime");
            if let Some(service) = self.service_manager.services.get("voice_assistant") {
                unsafe {
                    service.on_message(envelope);
                }
            }
            return;
        }
        if topic == smearor_model_mcp::TOPIC_MCP_REGISTER_RESOURCE {
            MessageHandler::<FfiEnvelopePayload<RegisterResourceMessage>>::handle_envelope_message(&self.mcp_registry, &envelope);
            trace!("Plugin registered a new MCP resource, list_changed notification deferred to SDK runtime");
            if let Some(service) = self.service_manager.services.get("voice_assistant") {
                unsafe {
                    service.on_message(envelope);
                }
            }
            return;
        }
        if topic == smearor_model_mcp::TOPIC_MCP_REGISTER_PROMPT {
            MessageHandler::<FfiEnvelopePayload<RegisterPromptMessage>>::handle_envelope_message(&self.mcp_registry, &envelope);
            trace!("Plugin registered a new MCP prompt, list_changed notification deferred to SDK runtime");
            if let Some(service) = self.service_manager.services.get("voice_assistant") {
                unsafe {
                    service.on_message(envelope);
                }
            }
            return;
        }

        // Global MCP invocation responses complete the pending response trackers
        // and are forwarded to the voice_assistant service which awaits
        // tool results via its own oneshot channels.
        if topic == smearor_model_mcp::TOPIC_MCP_TOOL_RESPONSE {
            trace!("Application: received mcp.tool.response, forwarding to tracker and voice_assistant");
            let response = unsafe { &*(envelope.payload as *const InvokeToolResponse) };
            let result = if response.error.is_empty() {
                Ok(response.result.to_string())
            } else {
                Err(response.error.to_string())
            };
            self.mcp_response_tracker.resolve(&response.correlation_id.to_string(), result);
            if let Some(service) = self.service_manager.services.get("voice_assistant") {
                unsafe {
                    service.on_message(envelope);
                }
            } else {
                trace!("voice_assistant service not found, dropping tool response");
            }
            return;
        }
        if topic == smearor_model_mcp::TOPIC_MCP_RESOURCE_RESPONSE {
            let response = unsafe { &*(envelope.payload as *const InvokeResourceResponse) };
            let result = if response.error.is_empty() {
                Ok(response.contents.to_string())
            } else {
                Err(response.error.to_string())
            };
            self.mcp_response_tracker.resolve(&response.correlation_id.to_string(), result);
            if let Some(service) = self.service_manager.services.get("voice_assistant") {
                unsafe {
                    service.on_message(envelope);
                }
            }
            return;
        }
        if topic == smearor_model_mcp::TOPIC_MCP_PROMPT_RESPONSE {
            let response = unsafe { &*(envelope.payload as *const InvokePromptResponse) };
            let messages_json = if response.error.is_empty() {
                let messages: Vec<serde_json::Value> = response
                    .messages
                    .iter()
                    .map(|m| serde_json::json!({"role": m.role.to_string(), "content": m.content.to_string()}))
                    .collect();
                serde_json::to_string(&serde_json::json!({
                    "description": "",
                    "messages": messages
                }))
                .unwrap_or_else(|_| "{}".to_string())
            } else {
                String::new()
            };
            let result = if response.error.is_empty() {
                Ok(messages_json)
            } else {
                Err(response.error.to_string())
            };
            self.mcp_response_tracker.resolve(&response.correlation_id.to_string(), result);
            if let Some(service) = self.service_manager.services.get("voice_assistant") {
                unsafe {
                    service.on_message(envelope);
                }
            }
            return;
        }

        // Try to convert a generic JSON-string payload into a typed message.
        // Services and instances share the same global registry.
        let mut envelope = envelope;
        if let Some(registry) = GLOBAL_JSON_CONVERTER_REGISTRY.get() {
            if let Some(converted) = try_convert_string_to_typed_envelope(registry, &envelope) {
                if !envelope.payload.is_null() {
                    if let Some(destroy) = envelope.destroy_payload {
                        unsafe {
                            (destroy)(envelope.payload);
                        }
                    }
                }
                envelope = converted;
            }
        }

        // Handle core.instance.* topics for dynamic instance management.
        if topic == TOPIC_CORE_INSTANCE_LOAD {
            if !envelope.payload.is_null() {
                let msg = unsafe { &*(envelope.payload as *const InstanceLoadMessage) };
                let instance_id = msg.instance_id.to_string();
                let config_path = msg.config_path.to_string();
                let instance_type = match &msg.instance_type {
                    ModelInstanceType::Gtk => crate::instance::InstanceType::Gtk,
                    ModelInstanceType::Headless => crate::instance::InstanceType::Headless,
                    ModelInstanceType::Web => crate::instance::InstanceType::Web,
                };
                let persist = msg.persist;
                let response_topic = msg.response_topic.to_string();
                let result = self.load_instance(instance_id, &config_path, instance_type, persist, true);
                self.send_broker_response(&response_topic, &result);
            }
            return;
        }
        if topic == TOPIC_CORE_INSTANCE_START {
            if !envelope.payload.is_null() {
                let msg = unsafe { &*(envelope.payload as *const InstanceStartMessage) };
                let instance_id = msg.instance_id.to_string();
                let response_topic = msg.response_topic.to_string();
                let result = self.start_instance(&instance_id);
                self.send_broker_response(&response_topic, &result);
            }
            return;
        }
        if topic == TOPIC_CORE_INSTANCE_STOP {
            if !envelope.payload.is_null() {
                let msg = unsafe { &*(envelope.payload as *const InstanceStopMessage) };
                let instance_id = msg.instance_id.to_string();
                let response_topic = msg.response_topic.to_string();
                let result = self.stop_instance(&instance_id);
                self.send_broker_response(&response_topic, &result);
            }
            return;
        }
        if topic == TOPIC_CORE_INSTANCE_UNLOAD {
            if !envelope.payload.is_null() {
                let msg = unsafe { &*(envelope.payload as *const InstanceUnloadMessage) };
                let instance_id = msg.instance_id.to_string();
                let response_topic = msg.response_topic.to_string();
                let result = self.unload_instance(&instance_id);
                self.send_broker_response(&response_topic, &result);
            }
            return;
        }
        if topic == TOPIC_CORE_INSTANCE_RELOAD {
            if !envelope.payload.is_null() {
                let msg = unsafe { &*(envelope.payload as *const InstanceReloadMessage) };
                let instance_id = msg.instance_id.to_string();
                let config_path = msg.config_path.to_string();
                let response_topic = msg.response_topic.to_string();
                let result = self.reload_instance(&instance_id, &config_path);
                self.send_broker_response(&response_topic, &result);
            }
            return;
        }

        // Handle auto_start_topic / auto_stop_topic for event-driven lifecycle control.
        let topic_action = {
            if let Ok(registry) = self.topic_instance_registry.lock() {
                registry.get(&topic).map(|(id, action)| (id.clone(), *action))
            } else {
                None
            }
        };
        if let Some((instance_id, action)) = topic_action {
            match action {
                TopicAction::Start => {
                    debug!("auto_start_topic '{}' triggered start for instance '{}'", topic, instance_id);
                    let _ = self.start_instance(&instance_id);
                }
                TopicAction::Stop => {
                    debug!("auto_stop_topic '{}' triggered stop for instance '{}'", topic, instance_id);
                    let _ = self.stop_instance(&instance_id);
                }
            }
            return;
        }

        // Route MacroPad command messages to the matching service by target_instance_id.
        if topic == TOPIC_MACROPAD_COMMAND && !envelope.payload.is_null() {
            let target_service_id = envelope.target_instance_id.to_string();
            if let Some(service) = self.service_manager.services.get(&target_service_id) {
                trace!("Routing MacroPad command to service {}", target_service_id);
                unsafe {
                    service.on_message(envelope);
                }
            } else {
                debug!("No MacroPad service found for target_instance_id '{}'", target_service_id);
            }
            return;
        }

        // Handle MacroPad connection status: load/stop headless instances.
        if topic == TOPIC_MACROPAD_CONNECTION && !envelope.payload.is_null() {
            let msg = unsafe { &*(envelope.payload as *const MacroPadConnectionStatus) };
            let instance_id = msg.instance_id.to_string();
            let device_id = msg.device_id.to_string();
            let connected = msg.connected;
            let key_count = msg.key_count;
            let key_columns = msg.key_columns;
            let key_width = msg.key_width;
            let key_height = msg.key_height;
            let driver = msg.driver.to_string();
            debug!(
                "MacroPad connection status: instance={} device={} connected={} key_count={} key_columns={} key_width={} key_height={} driver={}",
                instance_id, device_id, connected, key_count, key_columns, key_width, key_height, driver
            );

            if connected {
                // Check if instance already exists.
                let already_exists = self.instances.lock().map(|instances| instances.contains_key(&instance_id)).unwrap_or(false);
                if !already_exists {
                    // Try to load a config file for this instance.
                    let config_path = format!("configs/launcher/{}.toml", instance_id);
                    if std::path::Path::new(&config_path).exists() {
                        match self.load_instance(instance_id.clone(), &config_path, crate::instance::InstanceType::Headless, false, true) {
                            Ok(_) => {
                                debug!("Loaded headless instance '{}' for MacroPad device '{}'", instance_id, device_id);
                                // Attach device metadata to the instance.
                                if let Ok(instances) = self.instances.lock() {
                                    if let Some(instance) = instances.get(&instance_id) {
                                        if let Ok(mut metadata) = instance.device_metadata.lock() {
                                            *metadata = Some(crate::instance::MacroPadDeviceMetadata {
                                                device_id: device_id.clone(),
                                                key_count,
                                                key_columns,
                                                key_width,
                                                key_height,
                                                driver: driver.clone(),
                                            });
                                        }
                                    }
                                }
                                // Render initial button images to the device.
                                self.render_buttons_to_device(&instance_id);
                            }
                            Err(e) => {
                                error!("Failed to load headless instance '{}': {}", instance_id, e);
                            }
                        }
                    } else {
                        trace!("No config file found for MacroPad instance '{}' at {}", instance_id, config_path);
                    }
                } else {
                    // Instance already exists (e.g. loaded from persistence at startup).
                    // Update device metadata and re-render buttons.
                    debug!("Instance '{}' already exists, updating device metadata and rendering buttons", instance_id);
                    if let Ok(instances) = self.instances.lock() {
                        if let Some(instance) = instances.get(&instance_id) {
                            if let Ok(mut metadata) = instance.device_metadata.lock() {
                                *metadata = Some(crate::instance::MacroPadDeviceMetadata {
                                    device_id: device_id.clone(),
                                    key_count,
                                    key_columns,
                                    key_width,
                                    key_height,
                                    driver: driver.clone(),
                                });
                            }
                        }
                    }
                    self.render_buttons_to_device(&instance_id);
                }
            } else {
                // Unload the instance on disconnect.
                let exists = self.instances.lock().map(|instances| instances.contains_key(&instance_id)).unwrap_or(false);
                if exists {
                    match self.unload_instance(&instance_id) {
                        Ok(_) => debug!("Unloaded headless instance '{}' for MacroPad device '{}'", instance_id, device_id),
                        Err(e) => error!("Failed to unload instance '{}': {}", instance_id, e),
                    }
                }
            }
            // Fall through to broadcast to all plugins (don't return).
        }

        // Handle MacroPad input: route to the matching instance by instance_id.
        if topic == TOPIC_MACROPAD_INPUT && !envelope.payload.is_null() {
            let msg = unsafe { &*(envelope.payload as *const MacroPadInputMessage) };
            let instance_id = msg.instance_id.to_string();
            let button_index = msg.button_index;
            let pressed = msg.pressed;
            debug!("MacroPad input: instance={} button={} pressed={}", instance_id, button_index, pressed);

            if pressed {
                // Record press start time for longpress/hold detection.
                if let Ok(mut press_times) = self.macropad_press_times.lock() {
                    press_times.insert((instance_id.clone(), button_index), Instant::now());
                }
                // If hold is configured for this button, dispatch hold_start immediately.
                if self.is_trigger_configured(&instance_id, button_index, "hold_topic") {
                    self.dispatch_macropad_action(&instance_id, button_index, "hold_start");
                }
                // Compound longpress: track span group presses.
                if let Some((span_group, group_buttons)) = self.get_span_group_for_button(&instance_id, button_index) {
                    let now = Instant::now();
                    if let Ok(mut compound) = self.macropad_compound_presses.lock() {
                        let entry = compound.entry((instance_id.clone(), span_group.clone())).or_default();
                        // Only add if not already pressed.
                        if !entry.iter().any(|(b, _)| *b == button_index) {
                            entry.push((button_index, now));
                        }
                        // Check if 2+ buttons are pressed within the compound window.
                        let recent_count = entry.iter().filter(|(_, t)| now.duration_since(*t) <= MACROPAD_COMPOUND_PRESS_WINDOW).count();
                        if recent_count >= 2 {
                            // Schedule compound longpress check after threshold.
                            let self_clone = self.clone();
                            let key = (instance_id.clone(), span_group.clone());
                            gtk4::glib::timeout_add_local_once(MACROPAD_LONGPRESS_THRESHOLD, move || {
                                if let Ok(compound) = self_clone.macropad_compound_presses.lock() {
                                    if let Some(presses) = compound.get(&key) {
                                        // Buttons are removed from tracking on release.
                                        // If 2+ are still tracked after 500ms, they're all held.
                                        if presses.len() >= 2 {
                                            for &btn in group_buttons.iter() {
                                                self_clone.dispatch_macropad_action(&key.0, btn, "compound_longpress");
                                            }
                                            if let Ok(mut dispatched) = self_clone.macropad_compound_dispatched.lock() {
                                                dispatched.insert(key.clone(), ());
                                            }
                                            debug!("Compound longpress triggered for span group '{}' in instance '{}'", key.1, key.0);
                                        }
                                    }
                                }
                            });
                        }
                    }
                }
            } else {
                // On release: determine action based on hold duration and configured triggers.
                let press_start = if let Ok(mut press_times) = self.macropad_press_times.lock() {
                    press_times.remove(&(instance_id.clone(), button_index))
                } else {
                    None
                };

                let duration = press_start.map(|start| start.elapsed()).unwrap_or(Duration::ZERO);

                // Check if this button is part of a span group and if a compound
                // longpress was triggered (all buttons held >= 500ms).
                let compound_triggered = if let Some((span_group, _)) = self.get_span_group_for_button(&instance_id, button_index) {
                    // Remove this button from compound tracking regardless.
                    if let Ok(mut compound) = self.macropad_compound_presses.lock() {
                        if let Some(presses) = compound.get_mut(&(instance_id.clone(), span_group.clone())) {
                            presses.retain(|(b, _)| *b != button_index);
                        }
                        if compound.get(&(instance_id.clone(), span_group.clone())).map_or(false, |p| p.is_empty()) {
                            compound.remove(&(instance_id.clone(), span_group.clone()));
                        }
                    }
                    // Check if compound longpress was actually dispatched for this span group.
                    let dispatched = if let Ok(mut dispatched) = self.macropad_compound_dispatched.lock() {
                        dispatched.remove(&(instance_id.clone(), span_group)).is_some()
                    } else {
                        false
                    };
                    if dispatched && duration >= MACROPAD_LONGPRESS_THRESHOLD {
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };

                if compound_triggered {
                    debug!(
                        "MacroPad: compound longpress suppressed individual action for instance '{}' button {} ({}ms)",
                        instance_id,
                        button_index,
                        duration.as_millis()
                    );
                    // Still dispatch hold_stop if hold was configured.
                    if self.is_trigger_configured(&instance_id, button_index, "hold_topic") {
                        self.dispatch_macropad_action(&instance_id, button_index, "hold_stop");
                    }
                } else {
                    let hold_configured = self.is_trigger_configured(&instance_id, button_index, "hold_topic");

                    // If hold is configured, dispatch hold_stop on release.
                    if hold_configured {
                        self.dispatch_macropad_action(&instance_id, button_index, "hold_stop");
                        // If hold is configured and press was short, suppress click
                        // (the hold_start + hold_stop pair already fired).
                        if duration < MACROPAD_LONGPRESS_THRESHOLD {
                            debug!(
                                "MacroPad: hold suppressed click for instance '{}' button {} ({}ms)",
                                instance_id,
                                button_index,
                                duration.as_millis()
                            );
                        } else {
                            // Hold was active but press was >= 500ms: dispatch longpress.
                            self.dispatch_macropad_action(&instance_id, button_index, "longpress");
                        }
                    } else if duration >= MACROPAD_LONGPRESS_THRESHOLD {
                        // No hold configured, long press.
                        self.dispatch_macropad_action(&instance_id, button_index, "longpress");
                    } else {
                        // Short press without hold: check for double press.
                        let double_press_configured = self.is_trigger_configured(&instance_id, button_index, "double_press_topic");

                        if double_press_configured && duration < MACROPAD_DOUBLE_PRESS_WINDOW {
                            // Check for pending click (double press detection).
                            let has_pending = if let Ok(mut pending) = self.macropad_pending_clicks.lock() {
                                pending.remove(&(instance_id.clone(), button_index)).is_some()
                            } else {
                                false
                            };

                            if has_pending {
                                // Second click within window: dispatch double_press.
                                self.dispatch_macropad_action(&instance_id, button_index, "double_press");
                            } else {
                                // First click: record as pending and schedule delayed click dispatch.
                                if let Ok(mut pending) = self.macropad_pending_clicks.lock() {
                                    pending.insert((instance_id.clone(), button_index), Instant::now());
                                }
                                let self_clone = self.clone();
                                let key = (instance_id.clone(), button_index);
                                gtk4::glib::timeout_add_local_once(MACROPAD_DOUBLE_PRESS_WINDOW, move || {
                                    // If the pending click is still there (no second press came),
                                    // dispatch it as a normal click.
                                    let still_pending = if let Ok(mut pending) = self_clone.macropad_pending_clicks.lock() {
                                        pending.remove(&key).is_some()
                                    } else {
                                        false
                                    };
                                    if still_pending {
                                        self_clone.dispatch_macropad_action(&key.0, key.1, "click");
                                    }
                                });
                            }
                        } else {
                            // Normal click (duration 300-500ms, or no double_press configured).
                            self.dispatch_macropad_action(&instance_id, button_index, "click");
                        }
                    }
                }
            }
            // Fall through to broadcast to all plugins (don't return).
        }

        // Route service.* topics to the shared ServiceManager, except for known
        // outbound topics that services broadcast to widgets.
        if topic.starts_with("service.")
            && !topic.ends_with(".status")
            && !topic.ends_with(".scan_results")
            && !topic.ends_with(".vpn_profiles")
            && !topic.contains(".response.")
            && topic != TOPIC_MACROPAD_INPUT
            && topic != TOPIC_MACROPAD_CONNECTION
        {
            let parts: Vec<&str> = topic.split('.').collect();
            if parts.len() >= 2 {
                let target_service_id = parts[1];
                if let Some(service) = self.service_manager.services.get(target_service_id) {
                    trace!("Routing message to service {}", target_service_id);
                    unsafe {
                        service.on_message(envelope);
                    }
                }
            }
            return;
        }

        // MCP invocation requests are routed exclusively to the owning service
        // or plugin, as determined by the McpRegistry. This prevents unrelated
        // services from responding with errors for URIs/tools they don't own.
        if topic == smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE
            || topic == smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL
            || topic == smearor_model_mcp::TOPIC_MCP_INVOKE_PROMPT
        {
            let plugin_id = if topic == smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE {
                if !envelope.payload.is_null() {
                    let msg = unsafe { &*(envelope.payload as *const InvokeResourceMessage) };
                    let uri = msg.uri.to_string();
                    self.mcp_registry.list_resources().iter().find(|r| r.uri == uri).map(|r| r.plugin_id.clone())
                } else {
                    None
                }
            } else if topic == smearor_model_mcp::TOPIC_MCP_INVOKE_PROMPT {
                if !envelope.payload.is_null() {
                    let msg = unsafe { &*(envelope.payload as *const InvokePromptMessage) };
                    let name = msg.name.to_string();
                    self.mcp_registry.list_prompts().iter().find(|p| p.name == name).map(|p| p.plugin_id.clone())
                } else {
                    None
                }
            } else if !envelope.payload.is_null() {
                let msg = unsafe { &*(envelope.payload as *const InvokeToolMessage) };
                let name = msg.name.to_string();
                self.mcp_registry.list_tools().iter().find(|t| t.name == name).map(|t| t.plugin_id.clone())
            } else {
                None
            };

            if let Some(plugin_id) = plugin_id {
                debug!("routing {} to plugin_id={}", topic, plugin_id);
                // Try service first
                if let Some(service) = self.service_manager.services.get(&plugin_id) {
                    debug!("sending {} to service {}", topic, plugin_id);
                    unsafe {
                        service.on_message(envelope);
                    }
                    return;
                }
                // Route to specific plugin in instances via targeted target_instance_id
                let mut targeted_envelope = envelope;
                targeted_envelope.target_instance_id = stabby::string::String::from(plugin_id);
                if let Ok(instances) = self.instances.lock() {
                    for instance in instances.values() {
                        instance.handle_message(targeted_envelope.clone());
                    }
                }
                return;
            }

            // Fallback: no plugin owns this invocation. Try the launcher core's
            // built-in MCP capabilities (area tools, resources, etc.) before giving up.
            debug!("no plugin owner for {}, trying core MCP capabilities", topic);
            if let Ok(sender_guard) = self.mcp_command_sender.lock() {
                if let Some(sender) = sender_guard.as_ref().cloned() {
                    let broker_sender = self.broker_sender.clone();
                    if topic == smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL && !envelope.payload.is_null() {
                        let msg = unsafe { &*(envelope.payload as *const InvokeToolMessage) };
                        let name = msg.name.to_string();
                        let correlation_id = msg.correlation_id.to_string();
                        let arguments = serde_json::from_str(&msg.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                        tokio::spawn(async move {
                            let tools = smearor_mcp_server::tools::core_tools();
                            let result = smearor_mcp_server::tools::invoke_tool_sdk(&tools, sender, &name, Some(&arguments)).await;
                            let response = match result {
                                Ok(text) => InvokeToolResponse::success(&correlation_id, &text),
                                Err(error) => InvokeToolResponse::error(&correlation_id, &error),
                            };
                            let payload_ptr = box_payload(response);
                            let _ = broker_sender.send(
                                FfiEnvelope::builder()
                                    .sender_id("launcher-core")
                                    .target_instance_id("*")
                                    .topic(InvokeToolResponse::topic())
                                    .type_id(FfiEnvelopePayload::<InvokeToolResponse>::TYPE_ID)
                                    .payload(payload_ptr)
                                    .destroy_payload(Some(default_destroy_payload))
                                    .clone_payload(Some(default_clone_payload::<InvokeToolResponse>))
                                    .build(),
                            );
                        });
                        return;
                    }
                    if topic == smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE && !envelope.payload.is_null() {
                        let msg = unsafe { &*(envelope.payload as *const InvokeResourceMessage) };
                        let uri = msg.uri.to_string();
                        let correlation_id = msg.correlation_id.to_string();
                        tokio::spawn(async move {
                            let resources = smearor_mcp_server::resources::core_resources();
                            let result = smearor_mcp_server::resources::read_resource_sdk(&resources, sender, &uri).await;
                            let response = match result {
                                Ok((contents, _mime_type)) => InvokeResourceResponse::success(&correlation_id, &contents),
                                Err(error) => InvokeResourceResponse::error(&correlation_id, &error),
                            };
                            let payload_ptr = box_payload(response);
                            let _ = broker_sender.send(
                                FfiEnvelope::builder()
                                    .sender_id("launcher-core")
                                    .target_instance_id("*")
                                    .topic(InvokeResourceResponse::topic())
                                    .type_id(FfiEnvelopePayload::<InvokeResourceResponse>::TYPE_ID)
                                    .payload(payload_ptr)
                                    .destroy_payload(Some(default_destroy_payload))
                                    .clone_payload(Some(default_clone_payload::<InvokeResourceResponse>))
                                    .build(),
                            );
                        });
                        return;
                    }
                }
            }

            // Last resort: broadcast to all services so a handler can respond.
            debug!("no handler for {}, broadcasting to all services", topic);
            let service_ids: Vec<String> = self.service_manager.services.iter().map(|s| s.key().to_string()).collect();
            for service_id in service_ids {
                if let Some(service) = self.service_manager.services.get(&service_id) {
                    debug!("sending {} to service {} (broadcast)", topic, service_id);
                    unsafe {
                        service.on_message(envelope.clone());
                    }
                }
            }
            // Fall through to broadcast to instances below
        }

        // Route compositor command topics (Widget -> Service) to all services.
        // Services that implement the relevant MessageHandler will process them;
        // others will ignore them via on_message dispatch.
        if topic == TOPIC_SWITCH_WORKSPACE || topic == TOPIC_CREATE_WORKSPACE || topic == TOPIC_WORKSPACE_SNAPSHOT_REQUEST {
            let service_ids: Vec<String> = self.service_manager.services.iter().map(|s| s.key().to_string()).collect();
            for service_id in service_ids {
                if let Some(service) = self.service_manager.services.get(&service_id) {
                    trace!("Routing compositor command {} to service {}", topic, service_id);
                    unsafe {
                        service.on_message(envelope.clone());
                    }
                }
            }
            return;
        }

        // Broadcast to all instances and services (used by shared services for status updates).
        // Services also need status messages for cross-service features like DoA VAD triggering
        // and audio ducking. Services without a matching MessageHandler ignore the message.
        if target == "*" || (target.is_empty() && topic.ends_with(".status")) {
            if topic.starts_with("service.") {
                let service_ids: Vec<String> = self.service_manager.services.iter().map(|s| s.key().to_string()).collect();
                for service_id in service_ids {
                    if let Some(service) = self.service_manager.services.get(&service_id) {
                        trace!("Forwarding status {} to service {}", topic, service_id);
                        unsafe {
                            service.on_message(envelope.clone());
                        }
                    }
                }
            }
            if let Ok(instances) = self.instances.lock() {
                for instance in instances.values() {
                    instance.handle_message(envelope.clone());
                    self.forward_to_websockets(&instance.instance_id, &envelope);
                }
            }
            return;
        }

        // Broadcast service response topics (e.g. service.http.response.*)
        // to all instances so widgets can react to HTTP responses.
        if target.is_empty() && topic.starts_with("service.") && topic.contains(".response.") {
            if let Ok(instances) = self.instances.lock() {
                for instance in instances.values() {
                    instance.handle_message(envelope.clone());
                    self.forward_to_websockets(&instance.instance_id, &envelope);
                }
            }
            return;
        }

        // Broadcast workspace events from compositor services to all instances.
        if target.is_empty() && topic == TOPIC_WORKSPACE_CHANGED {
            let instance_ids: Vec<String> = if let Ok(instances) = self.instances.lock() {
                instances.values().map(|i| i.instance_id.clone()).collect()
            } else {
                Vec::new()
            };
            for id in &instance_ids {
                if let Ok(instances) = self.instances.lock() {
                    if let Some(instance) = instances.get(id) {
                        instance.handle_message(envelope.clone());
                        self.forward_to_websockets(&instance.instance_id, &envelope);
                    }
                }
                // After a workspace change, re-render buttons for headless MacroPad instances
                // since on_workspace_changed may have rebuilt areas with different plugins.
                self.render_buttons_to_device(id);
            }
            return;
        }

        // Broadcast workspace snapshot and lifecycle events to all instances.
        if target.is_empty() && (topic == TOPIC_WORKSPACE_SNAPSHOT || topic == TOPIC_WORKSPACE_LIFECYCLE) {
            if let Ok(instances) = self.instances.lock() {
                for instance in instances.values() {
                    instance.handle_message(envelope.clone());
                    self.forward_to_websockets(&instance.instance_id, &envelope);
                }
            }
            return;
        }

        // Detect implicit cross-instance addressing when a plugin sends an area_id
        // containing a colon (e.g. "side2:submenu") without setting target_instance_id.
        if target.is_empty() && topic.starts_with("area.") {
            let parts: Vec<&str> = topic.split('.').collect();
            if parts.len() >= 2 && parts[1].contains(':') {
                let (instance, area) = parts[1].split_once(':').unwrap_or(("", ""));
                if !instance.is_empty() {
                    target = instance.to_string();
                    // Reconstruct topic with local area_id for the target instance
                    let new_topic = format!(
                        "area.{}{}",
                        area,
                        if parts.len() > 2 {
                            format!(".{}", &parts[2..].join("."))
                        } else {
                            String::new()
                        }
                    );
                    let mut envelope = envelope;
                    envelope.topic = stabby::string::String::from(new_topic);
                    if let Ok(instances) = self.instances.lock() {
                        if let Some(target_instance) = instances.get(&target) {
                            target_instance.handle_message(envelope);
                        } else {
                            debug!("Unknown target instance '{}' for area message, dropping", target);
                        }
                    }
                    // Re-render buttons for headless MacroPad instances after area switch.
                    self.render_buttons_to_device(&target);

                    // For web instances, push updated widgets via WebSocket.
                    if let Ok(instances) = self.instances.lock() {
                        if let Some(instance) = instances.get(&target) {
                            if instance.instance_type == crate::instance::InstanceType::Web {
                                let widgets_html = crate::web::routes::render_all_widgets_html(instance);
                                self.send_web_widgets_update(&target, &widgets_html);
                            }
                        }
                    }
                    return;
                }
            }
        }

        // Route to a specific instance
        let target_instance = if target.is_empty() {
            // Extract instance from sender_id (format: "instance_id:plugin_id")
            envelope.sender_id.to_string().split(':').next().unwrap_or("").to_string()
        } else {
            target
        };

        // Extract widget.update payload before envelope is moved.
        let widget_update_info: Option<(String, String)> = if topic == TOPIC_WIDGET_UPDATE && !envelope.payload.is_null() {
            let msg = unsafe { &*(envelope.payload as *const WidgetUpdateMessage) };
            Some((msg.instance_id.to_string(), msg.plugin_id.to_string()))
        } else {
            None
        };

        if let Ok(instances) = self.instances.lock() {
            if let Some(instance) = instances.get(&target_instance) {
                self.forward_to_websockets(&target_instance, &envelope);
                instance.handle_message(envelope);
            } else {
                debug!("Unknown target instance '{}', dropping message", target_instance);
            }
        }

        // After area open/close on a headless MacroPad instance, re-render buttons.
        if topic == "area.open" || topic == "area.close" {
            self.render_buttons_to_device(&target_instance);

            // For web instances, render the new visible area's widgets and
            // push them via WebSocket so the frontend can re-render.
            if let Ok(instances) = self.instances.lock() {
                if let Some(instance) = instances.get(&target_instance) {
                    if instance.instance_type == crate::instance::InstanceType::Web {
                        let widgets_html = crate::web::routes::render_all_widgets_html(instance);
                        self.send_web_widgets_update(&target_instance, &widgets_html);
                    }
                }
            }
        }

        // Handle widget.update: a widget's visual state changed, re-render it.
        // Prefer the instance_id from the message payload, but fall back to
        // target_instance (derived from sender_id) when the payload's
        // instance_id is empty — widgets can't know their own instance_id
        // because meta.id doesn't include the instance prefix.
        if let Some((msg_instance_id, plugin_id)) = widget_update_info {
            let render_instance = if msg_instance_id.is_empty() { &target_instance } else { &msg_instance_id };
            trace!("Widget update: instance='{}' plugin='{}'", render_instance, plugin_id);
            self.render_single_button_to_device(render_instance, &plugin_id);

            // For web instances, re-render the single widget and push via WebSocket.
            if let Ok(instances) = self.instances.lock() {
                if let Some(instance) = instances.get(render_instance) {
                    if instance.instance_type == crate::instance::InstanceType::Web {
                        let namespaced_id = format!("{}:{}", render_instance, plugin_id);
                        let html = crate::web::routes::render_single_widget_html(instance, &namespaced_id);
                        self.send_widget_update(render_instance, &namespaced_id, &html);
                    }
                }
            }
        }
    }

    /// Forward a broker message to WebSocket clients if the target instance
    /// is a Web instance with an active WebSocket channel.
    fn forward_to_websockets(&self, instance_id: &str, envelope: &FfiEnvelope) {
        if instance_id.is_empty() {
            return;
        }

        let Ok(web_server_guard) = self.web_server.lock() else {
            return;
        };
        let Some(ref web_server) = *web_server_guard else {
            return;
        };

        let ws_manager = web_server.ws_manager();
        if ws_manager.get_sender(instance_id).is_none() {
            return;
        }

        let payload = crate::web::routes::extract_payload_as_json(envelope);
        let update = crate::web::web_update::WebUpdate {
            instance_id: instance_id.to_string(),
            topic: envelope.topic.to_string(),
            sender_id: envelope.sender_id.to_string(),
            payload,
        };
        ws_manager.broadcast(&update);
    }

    /// Push rendered widgets HTML to WebSocket clients of a web instance.
    fn send_web_widgets_update(&self, instance_id: &str, widgets_html: &str) {
        let Ok(web_server_guard) = self.web_server.lock() else {
            return;
        };
        let Some(ref web_server) = *web_server_guard else {
            return;
        };

        let ws_manager = web_server.ws_manager();
        if ws_manager.get_sender(instance_id).is_none() {
            return;
        }

        let update = crate::web::web_update::WebUpdate {
            instance_id: instance_id.to_string(),
            topic: "area.changed".to_string(),
            sender_id: "system".to_string(),
            payload: serde_json::json!({ "widgets_html": widgets_html }).to_string(),
        };
        ws_manager.broadcast(&update);
    }

    /// Push a single widget's updated HTML to WebSocket clients.
    fn send_widget_update(&self, instance_id: &str, plugin_id: &str, html: &str) {
        let Ok(web_server_guard) = self.web_server.lock() else {
            return;
        };
        let Some(ref web_server) = *web_server_guard else {
            return;
        };

        let ws_manager = web_server.ws_manager();
        if ws_manager.get_sender(instance_id).is_none() {
            return;
        }

        let update = crate::web::web_update::WebUpdate {
            instance_id: instance_id.to_string(),
            topic: "widget.update".to_string(),
            sender_id: "system".to_string(),
            payload: serde_json::json!({ "plugin_id": plugin_id, "html": html }).to_string(),
        };
        ws_manager.broadcast(&update);
    }

    /// Dynamically load a new launcher instance from a config file path.
    ///
    /// Loads the instance into `Ready` state. If `persist` is true, the instance
    /// is written to the state file. If `auto_start` is true and the config's
    /// `auto_start` field is true, the instance is automatically started.
    pub fn load_instance(
        &self,
        instance_id: String,
        config_path: &str,
        instance_type: crate::instance::InstanceType,
        persist: bool,
        auto_start: bool,
    ) -> Result<String, String> {
        validate_config_path(config_path)?;
        validate_instance_id(&instance_id)?;

        let config_content = std::fs::read_to_string(config_path).map_err(|e| format!("Failed to read config file '{}': {}", config_path, e))?;
        let mut config: SwipeLauncherConfig = toml::from_str(&config_content).map_err(|e| format!("Failed to parse config '{}': {}", config_path, e))?;
        config.resolve_defaults();

        if let Ok(instances) = self.instances.lock() {
            if instances.contains_key(&instance_id) {
                return Err(format!("Instance '{}' already exists", instance_id));
            }
        }

        // Set lifecycle to Loading before creating the instance.
        self.create_instance(instance_id.clone(), config.clone(), instance_type);

        // Store the config path for later reload.
        if let Ok(instances) = self.instances.lock() {
            if let Some(instance) = instances.get(&instance_id) {
                if let Ok(mut config_path_guard) = instance.config_path.lock() {
                    *config_path_guard = Some(config_path.to_string());
                }
            }
        }

        // Set lifecycle to Ready.
        if let Ok(instances) = self.instances.lock() {
            if let Some(instance) = instances.get(&instance_id) {
                if let Ok(mut lifecycle) = instance.lifecycle.lock() {
                    *lifecycle = LauncherInstanceLifecycle::Ready;
                }
            }
        }

        if instance_type == crate::instance::InstanceType::Gtk {
            self.css_watcher.watch_instance_css(std::path::Path::new(config_path));
        }

        // Subscribe to auto_start_topic / auto_stop_topic if configured.
        self.subscribe_instance_topics(&instance_id, &config);

        if persist {
            self.persist_instance(&instance_id, config_path, instance_type);
        }

        self.broadcast_instance_status(&instance_id, LauncherInstanceLifecycle::Ready);

        // Auto-start if requested and config allows it.
        if auto_start && config.launcher.auto_start {
            self.start_instance(&instance_id)?;
        }

        Ok(format!("Instance '{}' loaded from {}", instance_id, config_path))
    }

    /// Start a loaded (Ready) launcher instance — builds its window or headless areas.
    ///
    /// Transitions the instance from `Ready` to `Running` via the `Starting` intermediate state.
    /// If the instance is already `Running`, this is a no-op (idempotent).
    /// If `auto_stop_ttl` is configured, spawns a TTL timer task.
    pub fn start_instance(&self, instance_id: &str) -> Result<String, String> {
        let instance_type = {
            let instances = self.instances.lock().map_err(|e| format!("Failed to lock instances: {}", e))?;
            let instance = instances.get(instance_id).ok_or_else(|| format!("Instance '{}' not found", instance_id))?;
            instance.instance_type
        };

        // Check current lifecycle state and abort stale TTL timer.
        {
            let instances = self.instances.lock().map_err(|e| format!("Failed to lock instances: {}", e))?;
            let instance = instances.get(instance_id).ok_or_else(|| format!("Instance '{}' not found", instance_id))?;
            let mut lifecycle_guard = instance.lifecycle.lock().map_err(|e| format!("Failed to lock lifecycle: {}", e))?;
            if *lifecycle_guard == LauncherInstanceLifecycle::Running {
                return Ok(format!("Instance '{}' is already running", instance_id));
            }
            if *lifecycle_guard != LauncherInstanceLifecycle::Ready {
                return Err(format!("Instance '{}' is not in Ready state (current: {:?})", instance_id, *lifecycle_guard));
            }
            *lifecycle_guard = LauncherInstanceLifecycle::Starting;

            // Abort any stale TTL timer from a previous run.
            if let Ok(mut auto_stop_guard) = instance.auto_stop_handle.lock() {
                if let Some(old_handle) = auto_stop_guard.take() {
                    old_handle.abort();
                }
            }
        }

        // Build window or headless areas.
        if instance_type == crate::instance::InstanceType::Gtk {
            let self_clone = self.clone();
            let instance_id_clone = instance_id.to_string();
            gtk4::glib::idle_add_local_once(move || {
                if let Ok(instances) = self_clone.instances.lock() {
                    if let Some(instance) = instances.get(&instance_id_clone) {
                        match instance.build_window(&self_clone.gtk_app) {
                            Ok(window) => {
                                if let Ok(mut window_guard) = instance.window.lock() {
                                    *window_guard = Some(window);
                                }
                                debug!("Started GTK instance '{}'", instance_id_clone);
                            }
                            Err(e) => {
                                error!("Failed to build window for instance '{}': {}", instance_id_clone, e);
                                // Rollback lifecycle to Ready.
                                if let Ok(mut lifecycle) = instance.lifecycle.lock() {
                                    *lifecycle = LauncherInstanceLifecycle::Ready;
                                }
                            }
                        }
                    }
                }
            });
        } else {
            debug!("Started {:?} instance '{}'", instance_type, instance_id);
            if instance_type == crate::instance::InstanceType::Headless || instance_type == crate::instance::InstanceType::Web {
                if let Ok(instances) = self.instances.lock() {
                    if let Some(instance) = instances.get(instance_id) {
                        instance.build_headless();
                    }
                }
            }
        }

        // Set lifecycle to Running.
        let auto_stop_ttl = {
            let instances = self.instances.lock().map_err(|e| format!("Failed to lock instances: {}", e))?;
            let instance = instances.get(instance_id).ok_or_else(|| format!("Instance '{}' not found", instance_id))?;
            if let Ok(mut lifecycle) = instance.lifecycle.lock() {
                *lifecycle = LauncherInstanceLifecycle::Running;
            }
            instance.config.launcher.auto_stop_ttl
        };

        self.calculate_coordinated_sizes();

        // Update persisted state to running.
        self.update_persisted_lifecycle(instance_id, "running");

        self.broadcast_instance_status(instance_id, LauncherInstanceLifecycle::Running);

        // Spawn auto-stop TTL timer if configured.
        if let Some(ttl) = auto_stop_ttl {
            let instance_id_owned = instance_id.to_string();
            let broker_sender = self.broker_sender.clone();
            let handle = tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(ttl)).await;
                debug!("Auto-stop TTL expired for instance '{}', stopping", instance_id_owned);
                let stop_msg = InstanceStopMessage::new(&instance_id_owned, "");
                let payload_ptr = box_payload(stop_msg);
                let envelope = FfiEnvelope::builder()
                    .sender_id("auto-stop-ttl")
                    .target_instance_id("launcher-host")
                    .topic(TOPIC_CORE_INSTANCE_STOP)
                    .type_id(<InstanceStopMessage as TypedMessage>::TYPE_ID)
                    .payload(payload_ptr)
                    .destroy_payload(Some(default_destroy_payload))
                    .clone_payload(Some(default_clone_payload::<InstanceStopMessage>))
                    .build();
                let _ = broker_sender.send(envelope);
            });
            if let Ok(instances) = self.instances.lock() {
                if let Some(instance) = instances.get(instance_id) {
                    if let Ok(mut auto_stop_guard) = instance.auto_stop_handle.lock() {
                        *auto_stop_guard = Some(handle);
                    }
                }
            }
        }

        Ok(format!("Instance '{}' started", instance_id))
    }

    /// Subscribe to `auto_start_topic` and `auto_stop_topic` for event-driven lifecycle control.
    ///
    /// Called during `load_instance`. The subscriptions are topic-based — any message
    /// sent to the configured topic triggers the corresponding lifecycle action.
    fn subscribe_instance_topics(&self, instance_id: &str, config: &SwipeLauncherConfig) {
        if let Some(ref start_topic) = config.launcher.auto_start_topic {
            debug!("Instance '{}' subscribed to auto_start_topic '{}'", instance_id, start_topic);
            // Topic subscriptions are handled by the broker routing in route_message.
            // We store the mapping so that when a message arrives on this topic,
            // we know which instance to start.
            if let Ok(mut registry) = self.topic_instance_registry.lock() {
                registry.insert(start_topic.clone(), (instance_id.to_string(), TopicAction::Start));
            }
        }
        if let Some(ref stop_topic) = config.launcher.auto_stop_topic {
            debug!("Instance '{}' subscribed to auto_stop_topic '{}'", instance_id, stop_topic);
            if let Ok(mut registry) = self.topic_instance_registry.lock() {
                registry.insert(stop_topic.clone(), (instance_id.to_string(), TopicAction::Stop));
            }
        }
    }

    /// Unsubscribe from `auto_start_topic` and `auto_stop_topic`.
    fn unsubscribe_instance_topics(&self, instance_id: &str) {
        if let Ok(mut registry) = self.topic_instance_registry.lock() {
            registry.retain(|_, (id, _)| id != instance_id);
        }
    }

    /// Update the lifecycle field in the persisted state file without reloading the instance.
    fn update_persisted_lifecycle(&self, instance_id: &str, lifecycle: &str) {
        let state_path = get_instances_state_path();
        let mut entries = read_instances_state(&state_path);
        for entry in &mut entries {
            if entry.instance_id == instance_id {
                entry.lifecycle = lifecycle.to_string();
                break;
            }
        }
        write_instances_state(&state_path, &entries);
    }

    /// Unload a stopped (Ready) launcher instance — removes plugins, watchers, and persistence.
    ///
    /// Transitions the instance from `Ready` to removed. If the instance is `Running`,
    /// it is stopped first. This is the full removal — the instance ID is freed.
    pub fn unload_instance(&self, instance_id: &str) -> Result<String, String> {
        // Check if instance exists and its lifecycle state.
        let current_lifecycle = {
            let instances = self.instances.lock().map_err(|e| format!("Failed to lock instances: {}", e))?;
            let instance = instances.get(instance_id).ok_or_else(|| format!("Instance '{}' not found", instance_id))?;
            instance.lifecycle.lock().map(|g| g.clone()).unwrap_or(LauncherInstanceLifecycle::Ready)
        };

        // If running, stop first.
        if current_lifecycle == LauncherInstanceLifecycle::Running {
            self.stop_instance(instance_id)?;
        }

        // Now instance should be in Ready. Remove it.
        let instance = {
            let mut instances = self.instances.lock().map_err(|e| format!("Failed to lock instances: {}", e))?;
            instances.remove(instance_id).ok_or_else(|| format!("Instance '{}' not found", instance_id))?
        };

        // Cancel any remaining TTL timer.
        if let Ok(mut auto_stop_guard) = instance.auto_stop_handle.lock() {
            if let Some(handle) = auto_stop_guard.take() {
                handle.abort();
            }
        }

        // Unsubscribe from topic-based lifecycle control.
        self.unsubscribe_instance_topics(instance_id);

        // Remove CSS provider and file watch for this instance (GTK only).
        if instance.instance_type == crate::instance::InstanceType::Gtk {
            if let Some(config_path) = self.config_watcher.get_config_path(instance_id) {
                self.css_watcher.remove_instance_css(&config_path);
            }
        }

        // Remove config file watches.
        self.config_watcher.remove_instance(instance_id);

        // Unload plugins.
        instance.plugin_manager.unload_plugins();

        // Remove MCP tools/resources/prompts.
        self.mcp_registry.remove_tools_by_instance(instance_id);
        self.mcp_registry.remove_resources_by_instance(instance_id);
        self.mcp_registry.remove_prompts_by_instance(instance_id);

        // Unpersist.
        self.unpersist_instance(instance_id);

        self.broadcast_instance_status(instance_id, LauncherInstanceLifecycle::Unloading);

        Ok(format!("Instance '{}' unloaded", instance_id))
    }

    /// Render all visible area plugins to button images and send them to the MacroPad device.
    ///
    /// For each plugin in the currently visible area, calls `render_graphic`
    /// with the device's key dimensions and sends a `SetButtonImage` command
    /// to the MacroPad service via the message broker.
    pub fn render_buttons_to_device(&self, instance_id: &str) {
        let (device_id, driver, key_count, key_columns, key_width, key_height) = {
            let Ok(instances) = self.instances.lock() else {
                return;
            };
            let Some(instance) = instances.get(instance_id) else {
                return;
            };
            let Ok(metadata_guard) = instance.device_metadata.lock() else {
                return;
            };
            let Some(ref metadata) = *metadata_guard else {
                return;
            };
            (
                metadata.device_id.clone(),
                metadata.driver.clone(),
                metadata.key_count,
                metadata.key_columns,
                metadata.key_width,
                metadata.key_height,
            )
        };

        let plugin_entries: Vec<smearor_model_plugin::PluginEntry> = {
            let Ok(instances) = self.instances.lock() else {
                return;
            };
            let Some(instance) = instances.get(instance_id) else {
                return;
            };
            let Ok(area_manager) = instance.area_manager.lock() else {
                return;
            };
            area_manager.visible_area_plugin_entries()
        };

        debug!(
            "Rendering {} buttons to device '{}' ({}x{}) for instance '{}'",
            plugin_entries.len(),
            device_id,
            key_width,
            key_height,
            instance_id
        );

        // Track which physical buttons are rendered, for gap clearing and button_map.
        let mut rendered_buttons: Vec<bool> = vec![false; key_count as usize];
        let mut button_map: Vec<Option<String>> = vec![None; key_count as usize];

        // Group plugins by span_group. Plugins without span_group are individual.
        // We iterate in order, collecting consecutive plugins with the same span_group.
        let mut button_index: usize = 0;
        let mut iter = plugin_entries.iter().enumerate().peekable();

        while let Some((_, entry)) = iter.next() {
            if button_index as u8 >= key_count {
                break;
            }

            if let Some(ref span_group) = entry.span_group {
                // Collect all consecutive plugins with the same span_group.
                let mut group_members = vec![entry];
                while let Some(&(_, peek_entry)) = iter.peek() {
                    if peek_entry.span_group.as_ref() == Some(span_group) {
                        group_members.push(peek_entry);
                        iter.next();
                    } else {
                        break;
                    }
                }

                // Sort by span_index for deterministic ordering.
                group_members.sort_by_key(|e| e.span_index.unwrap_or(0));

                // Read span_rows and span_cols from the first member.
                // If both absent: backward-compatible 1×N horizontal span.
                let member_count = group_members.len() as u32;
                let (span_rows, span_cols) = match (group_members[0].span_rows, group_members[0].span_cols) {
                    (Some(rows), Some(cols)) => {
                        let expected = rows * cols;
                        if expected != member_count {
                            debug!(
                                "Span group '{}': member count {} does not match span_rows*span_cols={} ({}×{}), falling back to 1×N",
                                span_group, member_count, expected, rows, cols
                            );
                            (1, member_count)
                        } else {
                            (rows, cols)
                        }
                    }
                    _ => (1, member_count),
                };
                let group_size = span_rows * span_cols;

                // Find the next available position where the entire span_rows × span_cols
                // rectangle fits without overlapping any already-rendered button.
                let device_rows = if key_columns > 0 { key_count / key_columns } else { 0 };
                let mut effective_base: Option<u32> = None;
                let start_button = button_index as u32;

                'search: for candidate in start_button..(key_count as u32) {
                    let cand_col = candidate % key_columns as u32;
                    let cand_row = candidate / key_columns as u32;

                    // Check column overflow.
                    if cand_col + span_cols > key_columns as u32 {
                        continue;
                    }

                    // Check row overflow.
                    if device_rows > 0 && cand_row + span_rows > device_rows as u32 {
                        break;
                    }

                    // Check all buttons in the rectangle are free.
                    for r in 0..span_rows {
                        for c in 0..span_cols {
                            let physical = candidate + r * key_columns as u32 + c;
                            if physical as u8 >= key_count || rendered_buttons[physical as usize] {
                                continue 'search;
                            }
                        }
                    }

                    effective_base = Some(candidate);
                    break 'search;
                }

                let Some(effective_base) = effective_base else {
                    debug!("Span group '{}': no free position for {}×{} grid, skipping", span_group, span_rows, span_cols);
                    continue;
                };

                debug!(
                    "Span group '{}': placed at button {} (col={}, row={})",
                    span_group,
                    effective_base,
                    effective_base % key_columns as u32,
                    effective_base / key_columns as u32
                );

                let combined_width = key_width * span_cols;
                let combined_height = key_height * span_rows;

                // Render the first member at combined dimensions.
                let first_plugin_id = &group_members[0].id;
                let namespaced_id = format!("{}:{}", instance_id, first_plugin_id);
                let graphic = {
                    let Ok(instances) = self.instances.lock() else {
                        continue;
                    };
                    let Some(instance) = instances.get(instance_id) else {
                        continue;
                    };
                    let Some(plugin) = instance.plugin_manager.plugins.get(&namespaced_id) else {
                        debug!("Span group: plugin '{}' not found for rendering, skipping", namespaced_id);
                        continue;
                    };
                    unsafe { plugin.render_graphic(combined_width, combined_height) }
                };

                if let Some(graphic) = graphic {
                    let pixels = graphic.as_pixels();
                    let graphic_width = graphic.width;
                    let graphic_height = graphic.height;

                    // Split the combined image into grid slices and send each to its physical button.
                    for (i, member) in group_members.iter().enumerate() {
                        let row = i as u32 / span_cols;
                        let col = i as u32 % span_cols;
                        let physical_button = effective_base + row * key_columns as u32 + col;
                        if physical_button as u8 >= key_count {
                            break;
                        }
                        let x_offset = col * key_width;
                        let y_offset = row * key_height;
                        let slice_pixels = extract_grid_slice(pixels, graphic_width, graphic_height, x_offset, y_offset, key_width, key_height);
                        self.send_button_image(&device_id, &driver, instance_id, physical_button as u8, key_width, key_height, slice_pixels);
                        rendered_buttons[physical_button as usize] = true;
                        button_map[physical_button as usize] = Some(member.id.clone());
                        debug!("Sent span group slice {} (plugin '{}') to button {} on device '{}'", i, member.id, physical_button, device_id);
                    }
                } else {
                    debug!("Span group: plugin '{}' has no render_graphic, skipping {} buttons", first_plugin_id, group_size);
                }
                button_index = (effective_base + span_cols) as usize;
            } else {
                // Individual plugin — render at standard dimensions.
                // Search from the beginning for the first free button to fill gaps
                // left by span groups that were placed further ahead.
                button_index = 0;
                while button_index < key_count as usize && rendered_buttons[button_index] {
                    button_index += 1;
                }
                if button_index as u8 >= key_count {
                    break;
                }

                let plugin_id = &entry.id;
                let namespaced_id = format!("{}:{}", instance_id, plugin_id);
                let graphic = {
                    let Ok(instances) = self.instances.lock() else {
                        continue;
                    };
                    let Some(instance) = instances.get(instance_id) else {
                        continue;
                    };
                    let Some(plugin) = instance.plugin_manager.plugins.get(&namespaced_id) else {
                        debug!("Plugin '{}' not found for rendering, skipping", namespaced_id);
                        button_index += 1;
                        continue;
                    };
                    unsafe { plugin.render_graphic(key_width, key_height) }
                };

                if let Some(graphic) = graphic {
                    let pixels = graphic.as_pixels().to_vec();
                    self.send_button_image(&device_id, &driver, instance_id, button_index as u8, graphic.width, graphic.height, pixels);
                    rendered_buttons[button_index] = true;
                    button_map[button_index] = Some(plugin_id.clone());
                    trace!("Sent button image for index {} (plugin '{}') to device '{}'", button_index, plugin_id, device_id);
                } else {
                    debug!("Plugin '{}' has no render_graphic, skipping button {}", plugin_id, button_index);
                }
                button_index += 1;
            }
        }

        // Clear all buttons that were not rendered (gaps from alignment shifts + trailing empty buttons).
        for idx in 0..key_count as usize {
            if !rendered_buttons[idx] {
                let command = MacroPadCommand::clear_button(idx as u8);
                let msg = MacroPadCommandMessage::new(&device_id, command);
                let payload_ptr = box_payload(msg);
                let envelope = FfiEnvelope::builder()
                    .sender_id(instance_id)
                    .target_instance_id(driver.as_str())
                    .topic(MacroPadCommandMessage::topic())
                    .type_id(FfiEnvelopePayload::<MacroPadCommandMessage>::TYPE_ID)
                    .payload(payload_ptr)
                    .destroy_payload(Some(default_destroy_payload))
                    .clone_payload(Some(default_clone_payload::<MacroPadCommandMessage>))
                    .build();
                let _ = self.broker_sender.send(envelope);
                trace!("Cleared unrendered button {} on device '{}'", idx, device_id);
            }
        }

        // Store the button map for input dispatch.
        if let Ok(instances) = self.instances.lock() {
            if let Some(instance) = instances.get(instance_id) {
                if let Ok(mut map) = instance.button_map.lock() {
                    *map = Some(button_map);
                }
            }
        }
    }

    /// Send a `SetButtonImage` command for a single button to the MacroPad device.
    fn send_button_image(&self, device_id: &str, driver: &str, instance_id: &str, button_index: u8, width: u32, height: u32, pixels: Vec<u8>) {
        let command = MacroPadCommand::set_button_image(button_index, width, height, pixels);
        let msg = MacroPadCommandMessage::new(device_id, command);
        let payload_ptr = box_payload(msg);
        let envelope = FfiEnvelope::builder()
            .sender_id(instance_id)
            .target_instance_id(driver)
            .topic(MacroPadCommandMessage::topic())
            .type_id(FfiEnvelopePayload::<MacroPadCommandMessage>::TYPE_ID)
            .payload(payload_ptr)
            .destroy_payload(Some(default_destroy_payload))
            .clone_payload(Some(default_clone_payload::<MacroPadCommandMessage>))
            .build();
        let _ = self.broker_sender.send(envelope);
    }

    /// Re-render a single plugin's button image and send it to the MacroPad device.
    ///
    /// Called when a widget sends a `widget.update` message indicating its
    /// visual state has changed. Finds the plugin's button index in the
    /// visible area and sends only that button's updated image.
    /// If the plugin is part of a span group, re-renders the entire group.
    pub fn render_single_button_to_device(&self, instance_id: &str, plugin_id: &str) {
        let (device_id, driver, key_count, key_columns, key_width, key_height) = {
            let Ok(instances) = self.instances.lock() else {
                return;
            };
            let Some(instance) = instances.get(instance_id) else {
                return;
            };
            let Ok(metadata_guard) = instance.device_metadata.lock() else {
                return;
            };
            let Some(ref metadata) = *metadata_guard else {
                return;
            };
            (
                metadata.device_id.clone(),
                metadata.driver.clone(),
                metadata.key_count,
                metadata.key_columns,
                metadata.key_width,
                metadata.key_height,
            )
        };

        let plugin_entries: Vec<smearor_model_plugin::PluginEntry> = {
            let Ok(instances) = self.instances.lock() else {
                return;
            };
            let Some(instance) = instances.get(instance_id) else {
                return;
            };
            let Ok(area_manager) = instance.area_manager.lock() else {
                return;
            };
            area_manager.visible_area_plugin_entries()
        };

        // Find the plugin and check if it's part of a span group.
        let target_entry = plugin_entries.iter().find(|e| e.id == plugin_id);
        let Some(target_entry) = target_entry else {
            trace!("render_single_button: plugin '{}' not in visible area for instance '{}'", plugin_id, instance_id);
            return;
        };

        if let Some(ref span_group) = target_entry.span_group {
            // Plugin is part of a span group — re-render the entire group.
            let mut group_members: Vec<&smearor_model_plugin::PluginEntry> =
                plugin_entries.iter().filter(|e| e.span_group.as_ref() == Some(span_group)).collect();
            group_members.sort_by_key(|e| e.span_index.unwrap_or(0));

            // Read span_rows and span_cols from the first member.
            let member_count = group_members.len() as u32;
            let (span_rows, span_cols) = match (group_members[0].span_rows, group_members[0].span_cols) {
                (Some(rows), Some(cols)) => {
                    let expected = rows * cols;
                    if expected != member_count { (1, member_count) } else { (rows, cols) }
                }
                _ => (1, member_count),
            };
            let group_size = span_rows * span_cols;

            let combined_width = key_width * span_cols;
            let combined_height = key_height * span_rows;

            // Find the physical starting button index for this group from the button_map.
            let first_member_id = &group_members[0].id;
            let button_index = {
                let Ok(instances) = self.instances.lock() else {
                    return;
                };
                let Some(instance) = instances.get(instance_id) else {
                    return;
                };
                let Ok(map) = instance.button_map.lock() else {
                    return;
                };
                let Some(ref button_map) = *map else {
                    return;
                };
                match button_map.iter().position(|id| id.as_deref() == Some(first_member_id)) {
                    Some(idx) => idx,
                    None => return,
                }
            };

            if button_index as u8 >= key_count {
                return;
            }

            // Alignment validation: check row and column overflow.
            let base_button = button_index as u32;
            let base_col = base_button % key_columns as u32;
            let base_row = base_button / key_columns as u32;

            let mut effective_base = base_button;
            if base_col + span_cols > key_columns as u32 {
                let next_row_start = (base_row + 1) * key_columns as u32;
                debug!(
                    "render_single_button: span group '{}' would overflow row boundary, advancing to button {}",
                    span_group, next_row_start
                );
                effective_base = next_row_start;
            }

            let device_rows = if key_columns > 0 { key_count / key_columns } else { 0 };
            let effective_row = effective_base / key_columns as u32;
            if device_rows > 0 && effective_row + span_rows > device_rows as u32 {
                debug!("render_single_button: span group '{}' would overflow device bottom, skipping", span_group);
                return;
            }

            let namespaced_id = format!("{}:{}", instance_id, first_member_id);
            let graphic = {
                let Ok(instances) = self.instances.lock() else {
                    return;
                };
                let Some(instance) = instances.get(instance_id) else {
                    return;
                };
                let Some(plugin) = instance.plugin_manager.plugins.get(&namespaced_id) else {
                    trace!("render_single_button: span group plugin '{}' not found, skipping", namespaced_id);
                    return;
                };
                unsafe { plugin.render_graphic(combined_width, combined_height) }
            };

            if let Some(graphic) = graphic {
                let pixels = graphic.as_pixels();
                let graphic_width = graphic.width;
                let graphic_height = graphic.height;

                for (i, member) in group_members.iter().enumerate() {
                    let row = i as u32 / span_cols;
                    let col = i as u32 % span_cols;
                    let physical_button = effective_base + row * key_columns as u32 + col;
                    if physical_button as u8 >= key_count {
                        break;
                    }
                    let x_offset = col * key_width;
                    let y_offset = row * key_height;
                    let slice_pixels = extract_grid_slice(pixels, graphic_width, graphic_height, x_offset, y_offset, key_width, key_height);
                    self.send_button_image(&device_id, &driver, instance_id, physical_button as u8, key_width, key_height, slice_pixels);
                    trace!(
                        "Re-rendered span group slice {} (plugin '{}') for button {} on device '{}'",
                        i, member.id, physical_button, device_id
                    );
                }
            } else {
                trace!("render_single_button: span group plugin '{}' has no render_graphic, skipping", first_member_id);
            }
            return;
        }

        // Individual plugin — render at standard dimensions.
        // Look up physical button index from the button_map.
        let button_index = {
            let Ok(instances) = self.instances.lock() else {
                return;
            };
            let Some(instance) = instances.get(instance_id) else {
                return;
            };
            let Ok(map) = instance.button_map.lock() else {
                return;
            };
            let Some(ref button_map) = *map else {
                return;
            };
            match button_map.iter().position(|id| id.as_deref() == Some(plugin_id)) {
                Some(idx) => idx,
                None => {
                    trace!("render_single_button: plugin '{}' not in button_map for instance '{}'", plugin_id, instance_id);
                    return;
                }
            }
        };

        if button_index as u8 >= key_count {
            trace!("render_single_button: button index {} >= key_count {} for plugin '{}'", button_index, key_count, plugin_id);
            return;
        }

        let namespaced_id = format!("{}:{}", instance_id, plugin_id);
        let graphic = {
            let Ok(instances) = self.instances.lock() else {
                return;
            };
            let Some(instance) = instances.get(instance_id) else {
                return;
            };
            let Some(plugin) = instance.plugin_manager.plugins.get(&namespaced_id) else {
                trace!("render_single_button: plugin '{}' not found, skipping", namespaced_id);
                return;
            };
            unsafe { plugin.render_graphic(key_width, key_height) }
        };

        if let Some(graphic) = graphic {
            let pixels = graphic.as_pixels().to_vec();
            self.send_button_image(&device_id, &driver, instance_id, button_index as u8, graphic.width, graphic.height, pixels);
            trace!("Re-rendered single button {} (plugin '{}') for device '{}'", button_index, plugin_id, device_id);
        } else {
            trace!("render_single_button: plugin '{}' has no render_graphic, skipping", plugin_id);
        }
    }

    /// Stop a running launcher instance — closes its window or headless areas.
    ///
    /// Transitions the instance from `Running` to `Ready`. The instance remains
    /// loaded (plugins stay in memory) and can be started again with `start_instance`.
    /// If the instance is already `Ready`, this is a no-op (idempotent).
    /// Cancels any active `auto_stop_ttl` timer.
    pub fn stop_instance(&self, instance_id: &str) -> Result<String, String> {
        // Check lifecycle state and cancel TTL timer.
        {
            let instances = self.instances.lock().map_err(|e| format!("Failed to lock instances: {}", e))?;
            let instance = instances.get(instance_id).ok_or_else(|| format!("Instance '{}' not found", instance_id))?;
            let mut lifecycle_guard = instance.lifecycle.lock().map_err(|e| format!("Failed to lock lifecycle: {}", e))?;
            if *lifecycle_guard == LauncherInstanceLifecycle::Ready {
                return Ok(format!("Instance '{}' is already stopped (Ready)", instance_id));
            }
            if *lifecycle_guard != LauncherInstanceLifecycle::Running {
                return Err(format!("Instance '{}' is not in Running state (current: {:?})", instance_id, *lifecycle_guard));
            }
            *lifecycle_guard = LauncherInstanceLifecycle::Stopping;

            // Cancel any active TTL timer.
            if let Ok(mut auto_stop_guard) = instance.auto_stop_handle.lock() {
                if let Some(handle) = auto_stop_guard.take() {
                    handle.abort();
                }
            }
        }

        let instance_id_owned = instance_id.to_string();
        let instance_type = {
            let instances = self.instances.lock().map_err(|e| format!("Failed to lock instances: {}", e))?;
            instances
                .get(instance_id)
                .map(|i| i.instance_type)
                .unwrap_or(crate::instance::InstanceType::Gtk)
        };

        if instance_type == crate::instance::InstanceType::Gtk {
            let self_clone = self.clone();
            let instance_id_for_closure = instance_id_owned.clone();
            gtk4::glib::idle_add_local_once(move || {
                if let Ok(instances) = self_clone.instances.lock() {
                    if let Some(instance) = instances.get(&instance_id_for_closure) {
                        if let Some(handler_id) = instance.close_handler_id.lock().ok().and_then(|mut g| g.take()) {
                            if let Ok(window_guard) = instance.window.lock() {
                                if let Some(ref window) = *window_guard {
                                    window.disconnect(handler_id);
                                }
                            }
                        }
                        if let Ok(mut window_guard) = instance.window.lock() {
                            if let Some(window) = window_guard.take() {
                                window.close();
                            }
                        }
                        if let Ok(area_manager) = instance.area_manager.lock() {
                            area_manager.remove_all_areas_keep_plugins();
                        }
                        debug!("Stopped GTK instance '{}'", instance_id_for_closure);
                    }
                }
            });
        } else {
            if instance_type == crate::instance::InstanceType::Web {
                if let Ok(ws_guard) = self.web_server.lock() {
                    if let Some(ref web_server) = *ws_guard {
                        web_server.unregister_instance(&instance_id_owned);
                    }
                }
            }
            if let Ok(instances) = self.instances.lock() {
                if let Some(instance) = instances.get(&instance_id_owned) {
                    if let Ok(area_manager) = instance.area_manager.lock() {
                        area_manager.remove_all_areas_keep_plugins();
                    }
                }
            }
            debug!("Stopped {:?} instance '{}'", instance_type, instance_id_owned);
        }

        // Set lifecycle to Ready.
        if let Ok(instances) = self.instances.lock() {
            if let Some(instance) = instances.get(instance_id) {
                if let Ok(mut lifecycle) = instance.lifecycle.lock() {
                    *lifecycle = LauncherInstanceLifecycle::Ready;
                }
            }
        }

        self.calculate_coordinated_sizes();
        self.update_persisted_lifecycle(instance_id, "ready");
        self.broadcast_instance_status(instance_id, LauncherInstanceLifecycle::Ready);

        Ok(format!("Instance '{}' stopped", instance_id))
    }

    /// Hot-reload an instance: stop, unload, re-load, and restore the previous lifecycle state.
    ///
    /// The previous lifecycle state (Running or Ready) is preserved across the reload.
    /// `auto_start` from the config file is suppressed during reload — the instance
    /// returns to whatever state it was in before the reload.
    pub fn reload_instance(&self, instance_id: &str, config_path: &str) -> Result<String, String> {
        // Save previous lifecycle state.
        let previous_lifecycle = {
            if let Ok(instances) = self.instances.lock() {
                if let Some(instance) = instances.get(instance_id) {
                    instance.lifecycle.lock().map(|g| g.clone()).unwrap_or(LauncherInstanceLifecycle::Ready)
                } else {
                    return Err(format!("Instance '{}' not found", instance_id));
                }
            } else {
                return Err(format!("Failed to lock instances for reload of '{}'", instance_id));
            }
        };

        let instance_type = {
            if let Ok(instances) = self.instances.lock() {
                instances
                    .get(instance_id)
                    .map(|i| i.instance_type)
                    .unwrap_or(crate::instance::InstanceType::Gtk)
            } else {
                crate::instance::InstanceType::Gtk
            }
        };

        // Unload the instance entirely.
        self.unload_instance(instance_id)?;

        // Re-load with auto_start suppressed (false) — we'll restore the previous state manually.
        let result = self.load_instance(instance_id.to_string(), config_path, instance_type, true, false);

        // Restore previous lifecycle state.
        if previous_lifecycle == LauncherInstanceLifecycle::Running {
            if let Err(e) = self.start_instance(instance_id) {
                error!("Failed to restore Running state after reload of '{}': {}", instance_id, e);
            }
        }

        self.broadcast_instance_status(instance_id, LauncherInstanceLifecycle::Ready);
        result
    }

    /// List all currently loaded launcher instances with their lifecycle state.
    pub fn list_instances(&self) -> Result<String, String> {
        let instances = self.instances.lock().map_err(|e| format!("Failed to lock instances: {}", e))?;
        let list: Vec<serde_json::Value> = instances
            .values()
            .map(|inst| {
                let has_window = inst.window.lock().ok().map(|g| g.is_some()).unwrap_or(false);
                let lifecycle = inst.lifecycle.lock().map(|g| g.as_str().to_string()).unwrap_or_else(|_| "unknown".to_string());
                serde_json::json!({
                    "instance_id": inst.instance_id,
                    "instance_type": inst.instance_type.as_str(),
                    "has_window": has_window,
                    "lifecycle": lifecycle,
                })
            })
            .collect();
        serde_json::to_string(&list).map_err(|e| e.to_string())
    }

    /// Get the status of the embedded web server.
    ///
    /// Returns a JSON object with `port`, `enabled`, `bind_address`,
    /// `auth_required`, and `instances` (list of active web instance IDs).
    pub fn web_server_status(&self) -> Result<String, String> {
        let web_server_guard = self.web_server.lock().map_err(|e| format!("Failed to lock web_server: {}", e))?;
        match &*web_server_guard {
            Some(web_server) => {
                let config = web_server.config();
                let web_instance_ids: Vec<String> = {
                    let instances = self.instances.lock().map_err(|e| format!("Failed to lock instances: {}", e))?;
                    instances
                        .values()
                        .filter(|i| i.instance_type == crate::instance::InstanceType::Web)
                        .map(|i| i.instance_id.clone())
                        .collect()
                };
                let status = serde_json::json!({
                    "enabled": true,
                    "port": config.port,
                    "bind_address": config.bind_address,
                    "auth_required": config.auth_token.is_some(),
                    "instances": web_instance_ids,
                });
                serde_json::to_string(&status).map_err(|e| e.to_string())
            }
            None => {
                let status = serde_json::json!({
                    "enabled": false,
                    "port": null,
                    "bind_address": null,
                    "auth_required": false,
                    "instances": [],
                });
                serde_json::to_string(&status).map_err(|e| e.to_string())
            }
        }
    }

    /// Broadcast an instance status message to all instances and services.
    fn broadcast_instance_status(&self, instance_id: &str, event: LauncherInstanceLifecycle) {
        let status_msg = InstanceStatusMessage::new(instance_id, event);
        let payload_ptr = box_payload(status_msg);
        let envelope = FfiEnvelope::builder()
            .sender_id("launcher-host")
            .target_instance_id("*")
            .topic(TOPIC_CORE_INSTANCE_STATUS)
            .type_id(<InstanceStatusMessage as TypedMessage>::TYPE_ID)
            .payload(payload_ptr)
            .destroy_payload(Some(default_destroy_payload))
            .clone_payload(Some(default_clone_payload::<InstanceStatusMessage>))
            .build();
        self.route_message(envelope);
    }

    /// Send a broker response on the given topic.
    fn send_broker_response(&self, response_topic: &str, result: &Result<String, String>) {
        if response_topic.is_empty() {
            return;
        }
        let payload = serde_json::json!({
            "ok": result.is_ok(),
            "message": result.as_ref().map(|s| s.as_str()).unwrap_or_else(|e| e.as_str()),
        });
        let payload_str = payload.to_string();
        let payload_ptr = box_payload(payload_str);
        let envelope = FfiEnvelope::builder()
            .sender_id("launcher-host")
            .target_instance_id("*")
            .topic(response_topic)
            .type_id(0)
            .payload(payload_ptr)
            .destroy_payload(Some(default_destroy_payload))
            .clone_payload(Some(default_clone_payload::<String>))
            .build();
        self.route_message(envelope);
    }

    /// Persist a dynamically loaded instance to the state file.
    fn persist_instance(&self, instance_id: &str, config_path: &str, instance_type: crate::instance::InstanceType) {
        let lifecycle = self
            .instances
            .lock()
            .ok()
            .and_then(|instances| {
                instances
                    .get(instance_id)
                    .and_then(|inst| inst.lifecycle.lock().ok().map(|g| g.as_str().to_string()))
            })
            .unwrap_or_else(|| "ready".to_string());
        let state_path = get_instances_state_path();
        let mut entries = read_instances_state(&state_path);
        entries.retain(|e| e.instance_id != instance_id);
        entries.push(PersistedInstance {
            instance_id: instance_id.to_string(),
            config_path: config_path.to_string(),
            instance_type: instance_type.as_str().to_string(),
            lifecycle,
        });
        write_instances_state(&state_path, &entries);
    }

    /// Remove an instance from the persistence state file.
    fn unpersist_instance(&self, instance_id: &str) {
        let state_path = get_instances_state_path();
        let mut entries = read_instances_state(&state_path);
        entries.retain(|e| e.instance_id != instance_id);
        write_instances_state(&state_path, &entries);
    }

    /// Load persisted instances from the state file on startup.
    ///
    /// Each persisted instance's `lifecycle` field determines whether it is
    /// auto-started (`running`) or only loaded into `Ready` state (`ready`).
    pub fn load_persisted_instances(&self) {
        let state_path = get_instances_state_path();
        let entries = read_instances_state(&state_path);
        for entry in &entries {
            if let Ok(instances) = self.instances.lock() {
                if instances.contains_key(&entry.instance_id) {
                    continue;
                }
            }
            let instance_type = match <crate::instance::InstanceType as std::str::FromStr>::from_str(&entry.instance_type) {
                Ok(t) => t,
                Err(e) => {
                    debug!("Skipping persisted instance '{}': {}", entry.instance_id, e);
                    continue;
                }
            };
            let auto_start = entry.lifecycle == "running";
            match self.load_instance(entry.instance_id.clone(), &entry.config_path, instance_type, true, auto_start) {
                Ok(msg) => debug!("Loaded persisted instance: {}", msg),
                Err(e) => error!("Failed to load persisted instance '{}': {}", entry.instance_id, e),
            }
        }
    }

    pub fn run(&self) {
        self.gtk_app.run_with_args(&[] as &[&str]);
    }
}

/// Validate that a config path is within allowed directories.
fn validate_config_path(config_path: &str) -> Result<(), String> {
    let path = std::path::Path::new(config_path);
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("Config path '{}' cannot be resolved: {}", config_path, e))?;

    let cwd = std::env::current_dir().unwrap_or_default();
    let config_dir = dirs::config_dir().unwrap_or_default().join("smearor");

    if canonical.starts_with(&cwd) || canonical.starts_with(&config_dir) {
        Ok(())
    } else {
        Err(format!("Config path '{}' is outside allowed directories (current dir and ~/.config/smearor/)", config_path))
    }
}

/// Validate that an instance ID contains only safe characters.
fn validate_instance_id(instance_id: &str) -> Result<(), String> {
    if instance_id.is_empty() {
        return Err("Instance ID must not be empty".to_string());
    }
    for ch in instance_id.chars() {
        if !ch.is_alphanumeric() && ch != '-' && ch != '_' {
            return Err(format!(
                "Instance ID '{}' contains invalid character '{}'. Only alphanumeric, hyphen, and underscore are allowed.",
                instance_id, ch
            ));
        }
    }
    Ok(())
}

impl Drop for LauncherHost {
    fn drop(&mut self) {
        // Service cleanup is handled by ServiceManager's own Drop when the
        // last Arc reference is released. Clones of LauncherHost must not
        // unload services while other clones are still using them.
    }
}
