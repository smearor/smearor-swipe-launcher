use crate::config::launcher::SwipeLauncherConfig;
use crate::config::services::ServicesConfig;
use crate::config::watcher::ConfigWatcher;
use crate::context::initialize_global_json_converter_registry;
use crate::css::CssWatcher;
use crate::css::create_css_provider;
use crate::display::AreaSize;
use crate::instance::LauncherInstance;
use crate::mcp::McpResponseTracker;
use crate::service::ServiceManager;
use async_channel::unbounded;
use gtk4::Application;
use gtk4::gdk::Display;
use gtk4::gdk::Monitor;
use gtk4::gio;
use gtk4::gio::prelude::*;
use gtk4::glib::MainContext;
use gtk4::prelude::*;
use smearor_model_mcp::McpRegistry;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::JsonConverterRegistry;
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

mod broker;
mod button_indexes;
mod device_render_info;
mod instance_lifecycle;
mod macropad;
mod rgba_pixels;
mod topic_action;

use smearor_swipe_launcher_plugin_api::default_clone_payload;
use smearor_swipe_launcher_plugin_api::default_destroy_payload;

pub use topic_action::TopicAction;

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

    pub fn run(&self) {
        self.gtk_app.run_with_args(&[] as &[&str]);
    }
}

impl Drop for LauncherHost {
    fn drop(&mut self) {
        // Service cleanup is handled by ServiceManager's own Drop when the
        // last Arc reference is released. Clones of LauncherHost must not
        // unload services while other clones are still using them.
    }
}
