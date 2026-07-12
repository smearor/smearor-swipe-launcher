use crate::config::launcher::SwipeLauncherConfig;
use crate::config::services::ServicesConfig;
use crate::context::GLOBAL_JSON_CONVERTER_REGISTRY;
use crate::context::initialize_global_json_converter_registry;
use crate::css_provider::create_css_provider;
use crate::display::AreaSize;
use crate::instance::LauncherInstance;
use crate::json_converter::JsonConverterRegistry;
use crate::mcp_registry::McpRegistry;
use crate::mcp_response_tracker::McpResponseTracker;
use crate::messages::try_convert_string_to_typed_envelope;
use crate::service_manager::ServiceManager;
use async_channel::unbounded;
use gtk4::Application;
use gtk4::gdk::Display;
use gtk4::gdk::Monitor;
use gtk4::gio;
use gtk4::gio::prelude::*;
use gtk4::glib::MainContext;
use gtk4::prelude::*;
use smearor_model_compositor::TOPIC_CREATE_WORKSPACE;
use smearor_model_compositor::TOPIC_SWITCH_WORKSPACE;
use smearor_model_compositor::TOPIC_WORKSPACE_CHANGED;
use smearor_model_compositor::TOPIC_WORKSPACE_LIFECYCLE;
use smearor_model_compositor::TOPIC_WORKSPACE_SNAPSHOT;
use smearor_model_compositor::TOPIC_WORKSPACE_SNAPSHOT_REQUEST;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::InvokeToolResponse;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
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
    pub(crate) hotplug_last_event: Arc<Mutex<Option<Instant>>>,
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
            hotplug_last_event: Arc::new(Mutex::new(None)),
        }
    }

    pub fn create_instance(&self, instance_id: String, config: SwipeLauncherConfig) {
        let instance = LauncherInstance::new(config, instance_id.clone(), self.broker_sender.clone());
        instance.load_plugins();
        if let Ok(mut instances) = self.instances.lock() {
            instances.insert(instance_id, instance);
        }
    }

    pub fn load_services(&self, services_config: &ServicesConfig) {
        for service_entry in &services_config.services {
            trace!("Loading service {}", service_entry.id);
            let service_config = services_config.plugin_config(&service_entry.id);
            trace!("Service config: {service_config:?}");
            if let Err(e) = self.service_manager.load_service(&service_entry, service_config) {
                error!("Failed to load service {}: {}", service_entry.id, e);
            }
        }
        debug!("Successfully loaded {} services", self.service_manager.services.len());
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

            create_css_provider();

            let instances = if let Ok(instances) = self_clone.instances.lock() {
                instances.values().map(|i| (i.instance_id.clone(), i.build_window(app))).collect::<Vec<_>>()
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

        // Global MCP registration messages are routed to the shared registry.
        if topic == smearor_model_mcp::TOPIC_MCP_REGISTER_TOOL {
            MessageHandler::<FfiEnvelopePayload<RegisterToolMessage>>::handle_envelope_message(&self.mcp_registry, &envelope);
            debug!("Plugin registered a new MCP tool, list_changed notification deferred to SDK runtime");
            return;
        }
        if topic == smearor_model_mcp::TOPIC_MCP_REGISTER_RESOURCE {
            MessageHandler::<FfiEnvelopePayload<RegisterResourceMessage>>::handle_envelope_message(&self.mcp_registry, &envelope);
            debug!("Plugin registered a new MCP resource, list_changed notification deferred to SDK runtime");
            return;
        }

        // Global MCP invocation responses complete the pending response trackers.
        if topic == smearor_model_mcp::TOPIC_MCP_TOOL_RESPONSE {
            let response = unsafe { &*(envelope.payload as *const InvokeToolResponse) };
            let result = if response.error.is_empty() {
                Ok(response.result.to_string())
            } else {
                Err(response.error.to_string())
            };
            self.mcp_response_tracker.resolve(&response.correlation_id.to_string(), result);
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

        // Route service.* topics to the shared ServiceManager, except for known
        // outbound topics that services broadcast to widgets.
        if topic.starts_with("service.")
            && !topic.ends_with(".status")
            && !topic.ends_with(".scan_results")
            && !topic.ends_with(".vpn_profiles")
            && !topic.contains(".response.")
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

        // MCP invocation requests must also reach shared services (e.g. sysinfo)
        // that register tools and resources. Plugins inside instances get the same
        // message via the broadcast below.
        if topic.starts_with("mcp.invoke.") {
            let service_count = self.service_manager.services.len();
            let service_ids: Vec<String> = self.service_manager.services.iter().map(|s| s.key().to_string()).collect();
            debug!(
                "routing mcp.invoke topic={} ServiceManager ptr={:p} count={} ids={:?}",
                topic,
                self.service_manager.as_ref(),
                service_count,
                service_ids
            );
            for service_id in service_ids {
                if let Some(service) = self.service_manager.services.get(&service_id) {
                    debug!("sending mcp.invoke to service {}", service_id);
                    unsafe {
                        service.on_message(envelope.clone());
                    }
                } else {
                    debug!("service {} disappeared after listing", service_id);
                }
            }
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

        // Broadcast to all instances (used by shared services for status updates)
        if target == "*" || (target.is_empty() && topic.ends_with(".status")) {
            if let Ok(instances) = self.instances.lock() {
                for instance in instances.values() {
                    instance.handle_message(envelope.clone());
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
                }
            }
            return;
        }

        // Broadcast workspace events from compositor services to all instances.
        if target.is_empty() && (topic == TOPIC_WORKSPACE_CHANGED || topic == TOPIC_WORKSPACE_SNAPSHOT || topic == TOPIC_WORKSPACE_LIFECYCLE) {
            if let Ok(instances) = self.instances.lock() {
                for instance in instances.values() {
                    instance.handle_message(envelope.clone());
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

        if let Ok(instances) = self.instances.lock() {
            if let Some(instance) = instances.get(&target_instance) {
                instance.handle_message(envelope);
            } else {
                debug!("Unknown target instance '{}', dropping message", target_instance);
            }
        }
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
