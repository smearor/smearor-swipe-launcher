use crate::config::launcher::SwipeLauncherConfig;
use crate::instance::InstanceType;
use crate::instance::PersistedInstance;
use crate::instance::get_instances_state_path;
use crate::instance::read_instances_state;
use crate::instance::write_instances_state;
use gtk4::prelude::*;
use smearor_model_instance_control::InstanceStatusMessage;
use smearor_model_instance_control::InstanceStopMessage;
use smearor_model_instance_control::LauncherInstanceLifecycle;
use smearor_model_instance_control::TOPIC_CORE_INSTANCE_STATUS;
use smearor_model_instance_control::TOPIC_CORE_INSTANCE_STOP;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::box_payload;
use tracing::debug;
use tracing::error;

use smearor_swipe_launcher_plugin_api::default_clone_payload;
use smearor_swipe_launcher_plugin_api::default_destroy_payload;

use super::TopicAction;

impl super::LauncherHost {
    /// Dynamically load a new launcher instance from a config file path.
    ///
    /// Loads the instance into `Ready` state. If `persist` is true, the instance
    /// is written to the state file. If `auto_start` is true and the config's
    /// `auto_start` field is true, the instance is automatically started.
    pub fn load_instance(
        &self,
        instance_id: String,
        config_path: &str,
        instance_type: InstanceType,
        persist: bool,
        auto_start: bool,
    ) -> Result<String, String> {
        crate::config::discovery::validate_config_path(config_path)?;
        crate::instance::validate_instance_id(&instance_id)?;

        let config_content = std::fs::read_to_string(config_path).map_err(|e| format!("Failed to read config file '{}': {}", config_path, e))?;
        let mut config: SwipeLauncherConfig = toml::from_str(&config_content).map_err(|e| format!("Failed to parse config '{}': {}", config_path, e))?;

        // 1. Resolve top-level includes (shared config fragments from external files)
        config
            .resolve_top_level_includes(std::path::Path::new(config_path))
            .map_err(|e| format!("{e}"))?;

        // 2. Resolve per-area includes (external TOML files referenced by area configs)
        config.resolve_includes(std::path::Path::new(config_path)).map_err(|e| format!("{e}"))?;

        // 3. Resolve global defaults for plugin configs (template expansion)
        config.resolve_defaults();

        // 4. Validate that the merged config defines at least one area.
        config.validate().map_err(|e| format!("{e}"))?;

        if let Ok(instances) = self.instances.lock() {
            if instances.contains_key(&instance_id) {
                return Err(format!("Instance '{}' already exists", instance_id));
            }
        }

        // Register config file and includes for hot-reload watching.
        let include_paths = config.collect_include_paths(std::path::Path::new(config_path));
        self.config_watcher.add_config(std::path::Path::new(config_path), &instance_id, &include_paths);

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
                // If the activate handler already built the window and set
                // lifecycle to Running, skip the idle callback entirely.
                let already_running = {
                    let Ok(instances) = self_clone.instances.lock() else { return };
                    let Some(instance) = instances.get(&instance_id_clone) else { return };
                    instance.lifecycle.lock().map(|g| *g == LauncherInstanceLifecycle::Running).unwrap_or(false)
                };
                if already_running {
                    debug!("GTK instance '{}' already running (via activate), skipping idle callback", instance_id_clone);
                    return;
                }

                let build_result = {
                    let Ok(instances) = self_clone.instances.lock() else { return };
                    let Some(instance) = instances.get(&instance_id_clone) else { return };
                    instance.build_window(&self_clone.gtk_app)
                };
                match build_result {
                    Ok(window) => {
                        if let Ok(instances) = self_clone.instances.lock() {
                            if let Some(instance) = instances.get(&instance_id_clone) {
                                if let Ok(mut window_guard) = instance.window.lock() {
                                    *window_guard = Some(window);
                                }
                            }
                        }
                        debug!("Started GTK instance '{}'", instance_id_clone);
                        self_clone.finalize_instance_start(&instance_id_clone);
                    }
                    Err(e) => {
                        error!("Failed to build window for instance '{}': {}", instance_id_clone, e);
                        if let Ok(instances) = self_clone.instances.lock() {
                            if let Some(instance) = instances.get(&instance_id_clone) {
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
            self.finalize_instance_start(instance_id);
        }

        Ok(format!("Instance '{}' started", instance_id))
    }

    /// Finalize instance start: set lifecycle to `Running`, recalculate sizes,
    /// update persisted state, broadcast status, and spawn auto-stop TTL timer.
    ///
    /// Called after the window or headless areas have been successfully built.
    pub(super) fn finalize_instance_start(&self, instance_id: &str) {
        let auto_stop_ttl = {
            let Ok(instances) = self.instances.lock() else { return };
            let Some(instance) = instances.get(instance_id) else { return };
            if let Ok(mut lifecycle) = instance.lifecycle.lock() {
                *lifecycle = LauncherInstanceLifecycle::Running;
            }
            instance.config.launcher.auto_stop_ttl
        };

        self.calculate_coordinated_sizes();
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
    }

    /// Subscribe to `auto_start_topic` and `auto_stop_topic` for event-driven lifecycle control.
    ///
    /// Called during `load_instance`. The subscriptions are topic-based — any message
    /// sent to the configured topic triggers the corresponding lifecycle action.
    pub(crate) fn subscribe_instance_topics(&self, instance_id: &str, config: &SwipeLauncherConfig) {
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
    pub(crate) fn unsubscribe_instance_topics(&self, instance_id: &str) {
        if let Ok(mut registry) = self.topic_instance_registry.lock() {
            registry.retain(|_, (id, _)| id != instance_id);
        }
    }

    /// Update the lifecycle field in the persisted state file without reloading the instance.
    pub(crate) fn update_persisted_lifecycle(&self, instance_id: &str, lifecycle: &str) {
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
            // Destroy window and clear areas synchronously to prevent duplicate windows on rapid stop/start.
            let instance_id_for_closure = instance_id_owned.clone();
            {
                let instances = self.instances.lock().map_err(|e| format!("Failed to lock instances: {}", e))?;
                if let Some(instance) = instances.get(&instance_id_for_closure) {
                    // Clear area manager first — releases references to main_container (child of window).
                    if let Ok(area_manager) = instance.area_manager.lock() {
                        area_manager.remove_all_areas_keep_plugins();
                        area_manager.clear_main_container();
                    }
                    // Disconnect close-request handler.
                    if let Some(handler_id) = instance.close_handler_id.lock().ok().and_then(|mut g| g.take()) {
                        if let Ok(window_guard) = instance.window.lock() {
                            if let Some(ref window) = *window_guard {
                                window.disconnect(handler_id);
                            }
                        }
                    }
                    // Remove and destroy window.
                    if let Ok(mut window_guard) = instance.window.lock() {
                        if let Some(window) = window_guard.take() {
                            window.set_visible(false);
                            self.gtk_app.remove_window(&window);
                        }
                    }
                    debug!("Stopped GTK instance '{}'", instance_id_for_closure);
                }
            }
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
                        area_manager.clear_main_container();
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
    pub(crate) fn broadcast_instance_status(&self, instance_id: &str, event: LauncherInstanceLifecycle) {
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
    pub(crate) fn send_broker_response(&self, response_topic: &str, result: &Result<String, String>) {
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
    pub(crate) fn persist_instance(&self, instance_id: &str, config_path: &str, instance_type: crate::instance::InstanceType) {
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
    pub(crate) fn unpersist_instance(&self, instance_id: &str) {
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
}
