use crate::command::ThemeCommand;
use crate::config::ThemeServiceConfig;
use crate::state::ThemeState;
use gtk4::CssProvider;
use gtk4::STYLE_PROVIDER_PRIORITY_USER;
use gtk4::style_context_add_provider_for_display;
use gtk4::style_context_remove_provider_for_display;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_personalization_model::TOPIC_STATUS as TOPIC_PERSONALIZATION_STATUS;
use smearor_swipe_launcher_plugin_api::AcceptTopic;
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
use smearor_theme_model::TOPIC_COMMAND;
use smearor_theme_model::TOPIC_STATUS;
use smearor_theme_model::Theme;
use smearor_theme_model::ThemeColorsStabby;
use smearor_theme_model::ThemeCommandAction;
use smearor_theme_model::ThemeCommandMessage;
use smearor_theme_model::ThemeInfo;
use smearor_theme_model::ThemeMode;
use smearor_theme_model::ThemeStatusMessage;
use smearor_wallpaper_model::WallpaperCommandMessage;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::debug;
use tracing::error;
use tracing::trace;

thread_local! {
    static ACTIVE_CSS_PROVIDERS: std::cell::RefCell<Vec<CssProvider>> = std::cell::RefCell::new(Vec::new());
}

#[allow(dead_code)]
pub struct ThemeService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: RwLock<ThemeServiceConfig>,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<ThemeCommand>,
    pub state: Arc<RwLock<ThemeState>>,
}

impl ThemeService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        smearor_theme_model::register_json_converters(core_context);

        let service_config: ThemeServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;

        let mut service_config = service_config;
        service_config.themes = service_config.load_or_discover_themes();

        let (command_sender, command_receiver) = tokio::sync::mpsc::unbounded_channel::<ThemeCommand>();
        let meta = PluginMeta::try_from(&config)?;
        let state = Arc::new(RwLock::new(ThemeState::default()));
        let state_clone = state.clone();
        let config_clone = service_config.clone();
        let meta_clone = meta.clone();
        let core_context_clone = core_context;
        let command_sender_clone = command_sender.clone();

        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    error!("Theme service: failed to create tokio runtime: {e}");
                    return;
                }
            };
            runtime.block_on(async move {
                run_command_loop(config_clone, command_receiver, meta_clone, core_context_clone, state_clone, command_sender_clone).await;
            });
        });

        {
            if let Ok(mut state_guard) = state.write() {
                state_guard.themes = service_config.themes.clone();
                if !service_config.default_theme.is_empty() {
                    if let Some(index) = service_config.themes.iter().position(|t| t.name == service_config.default_theme) {
                        state_guard.selected_theme_index = index;
                    }
                }
            }
        }

        let service = ThemeService {
            meta,
            core_context,
            config: RwLock::new(service_config),
            command_sender,
            state,
        };
        service.register_mcp_capabilities();
        Ok(service)
    }
}

impl MessageHandler<FfiEnvelopePayload<ThemeCommandMessage>> for ThemeService {
    fn handle_message(&self, message: FfiEnvelopePayload<ThemeCommandMessage>, _sender_id: &str) {
        let command = match message.action {
            ThemeCommandAction::SelectTheme => {
                let name = message.name.as_ref().map(|s| s.to_string()).unwrap_or_default();
                ThemeCommand::SelectTheme(name)
            }
            ThemeCommandAction::ApplySelected => ThemeCommand::ApplySelected,
            ThemeCommandAction::SelectAndApply => {
                let name = message.name.as_ref().map(|s| s.to_string()).unwrap_or_default();
                ThemeCommand::SelectAndApply(name)
            }
            ThemeCommandAction::Refresh => ThemeCommand::Refresh,
            ThemeCommandAction::AddTheme => {
                let theme_json = message.theme_json.as_ref().map(|s| s.to_string()).unwrap_or_default();
                match serde_json::from_str::<Theme>(&theme_json) {
                    Ok(theme) => ThemeCommand::AddTheme(theme),
                    Err(e) => {
                        error!("Theme service: failed to parse theme JSON: {e}");
                        return;
                    }
                }
            }
            ThemeCommandAction::RemoveTheme => {
                let name = message.name.as_ref().map(|s| s.to_string()).unwrap_or_default();
                ThemeCommand::RemoveTheme(name)
            }
        };
        let _ = self.command_sender.send(command);
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for ThemeService {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        let scheme = message.0.color_scheme;
        let _ = self.command_sender.send(ThemeCommand::ColorSchemeChanged(scheme));
    }
}

impl MessageBroadcaster for ThemeService {}

impl MessageTopicBroadcaster<ThemeStatusMessage> for ThemeService {}

impl PluginMetaGetter for ThemeService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for ThemeService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl AcceptTopic<FfiEnvelope> for ThemeService {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_COMMAND
            || topic == TOPIC_PERSONALIZATION_STATUS
            || topic == smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL
            || topic == smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE
            || topic == smearor_model_mcp::TOPIC_MCP_INVOKE_PROMPT
    }
}

impl ServicePlugin for ThemeService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                trace!("theme: on_message topic={} type_id={}", envelope.topic, envelope.type_id);
                if envelope.type_id == FfiEnvelopePayload::<ThemeCommandMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<ThemeCommandMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<PersonalizationStatusMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<PersonalizationStatusMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeResourceMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<smearor_model_mcp::InvokePromptMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<smearor_model_mcp::InvokePromptMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

async fn run_command_loop(
    mut config: ThemeServiceConfig,
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<ThemeCommand>,
    meta: PluginMeta,
    core_context: Option<FfiCoreContext>,
    state: Arc<RwLock<ThemeState>>,
    command_sender: tokio::sync::mpsc::UnboundedSender<ThemeCommand>,
) {
    if config.auto_apply && !config.default_theme.is_empty() {
        apply_selected_theme(&meta, &core_context, &config, &state, &command_sender);
    }

    broadcast_status(&meta, &core_context, &state);

    while let Some(command) = command_receiver.recv().await {
        match command {
            ThemeCommand::SelectTheme(name) => {
                select_theme(&state, &name);
                broadcast_status(&meta, &core_context, &state);
            }
            ThemeCommand::ApplySelected => {
                apply_selected_theme(&meta, &core_context, &config, &state, &command_sender);
                broadcast_status(&meta, &core_context, &state);
            }
            ThemeCommand::SelectAndApply(name) => {
                select_theme(&state, &name);
                apply_selected_theme(&meta, &core_context, &config, &state, &command_sender);
                broadcast_status(&meta, &core_context, &state);
            }
            ThemeCommand::Refresh => {
                broadcast_status(&meta, &core_context, &state);
            }
            ThemeCommand::AddTheme(theme) => {
                add_theme_to_config(&config.config_path, &theme);
                config.themes = config.load_or_discover_themes();
                if let Ok(mut state_guard) = state.write() {
                    state_guard.themes = config.themes.clone();
                }
                broadcast_status(&meta, &core_context, &state);
            }
            ThemeCommand::RemoveTheme(name) => {
                remove_theme_from_config(&config.config_path, &name);
                config.themes = config.load_or_discover_themes();
                if let Ok(mut state_guard) = state.write() {
                    state_guard.themes = config.themes.clone();
                    if state_guard.current_theme.as_deref() == Some(&name) {
                        state_guard.current_theme = None;
                    }
                }
                broadcast_status(&meta, &core_context, &state);
            }
            ThemeCommand::ColorSchemeChanged(scheme) => {
                let needs_reapply = {
                    if let Ok(state_guard) = state.read() {
                        state_guard.system_color_scheme != scheme
                            && state_guard
                                .themes
                                .get(state_guard.selected_theme_index)
                                .map(|t| t.mode == ThemeMode::System)
                                .unwrap_or(false)
                    } else {
                        false
                    }
                };
                if needs_reapply {
                    if let Ok(mut state_guard) = state.write() {
                        state_guard.system_color_scheme = scheme;
                    }
                    apply_selected_theme(&meta, &core_context, &config, &state, &command_sender);
                    broadcast_status(&meta, &core_context, &state);
                }
            }
        }
    }
}

fn select_theme(state: &Arc<RwLock<ThemeState>>, name: &str) {
    if let Ok(mut state_guard) = state.write() {
        if let Some(index) = state_guard.themes.iter().position(|t| t.name == name) {
            state_guard.selected_theme_index = index;
            debug!("Theme service: selected theme '{}'", name);
        } else {
            debug!("Theme service: theme '{}' not found", name);
        }
    }
}

fn apply_selected_theme(
    _meta: &PluginMeta,
    _core_context: &Option<FfiCoreContext>,
    _config: &ThemeServiceConfig,
    state: &Arc<RwLock<ThemeState>>,
    _command_sender: &tokio::sync::mpsc::UnboundedSender<ThemeCommand>,
) {
    let (theme, system_scheme) = {
        let state_guard = match state.read() {
            Ok(g) => g,
            Err(e) => {
                error!("Theme service: state lock poisoned: {e}");
                return;
            }
        };
        let theme = match state_guard.themes.get(state_guard.selected_theme_index) {
            Some(t) => t.clone(),
            None => {
                debug!("Theme service: no theme at index {}", state_guard.selected_theme_index);
                return;
            }
        };
        (theme, state_guard.system_color_scheme.clone())
    };

    let effective_mode = theme.mode.resolve(system_scheme);

    let css_files: &[String] = match effective_mode {
        ThemeMode::Dark => &theme.css_files_dark,
        ThemeMode::Light => {
            if theme.css_files_light.is_empty() {
                &theme.css_files_dark
            } else {
                &theme.css_files_light
            }
        }
        ThemeMode::System => &theme.css_files_dark,
    };

    let colors_css = theme.colors.to_css(effective_mode);

    let theme_name = theme.name.clone();
    let wallpaper_theme = theme.wallpaper_theme.clone();
    let file_paths: Vec<String> = css_files.iter().map(|p| shellexpand::tilde(p).into_owned()).collect();

    debug!(
        "Theme service: applying theme '{}' (mode: {:?}, effective: {:?}, css files: {}, wallpaper: {:?})",
        theme_name,
        theme.mode,
        effective_mode,
        file_paths.len(),
        wallpaper_theme
    );

    let provider_count = 1 + file_paths.len();

    glib::idle_add_once(move || {
        let display = match gtk4::gdk::Display::default() {
            Some(d) => d,
            None => {
                error!("Theme service: no GDK display found");
                return;
            }
        };

        ACTIVE_CSS_PROVIDERS.with(|providers| {
            let mut providers = providers.borrow_mut();
            for provider in providers.drain(..) {
                style_context_remove_provider_for_display(&display, &provider);
            }
        });

        let mut new_providers = Vec::with_capacity(provider_count);

        let var_provider = CssProvider::new();
        var_provider.load_from_string(&colors_css);
        style_context_add_provider_for_display(&display, &var_provider, STYLE_PROVIDER_PRIORITY_USER + 2);
        new_providers.push(var_provider);

        for file_path in &file_paths {
            let file_provider = CssProvider::new();
            if let Err(e) = std::fs::metadata(file_path) {
                debug!("Theme service: CSS file not found '{}': {}", file_path, e);
                continue;
            }
            file_provider.load_from_path(file_path);
            style_context_add_provider_for_display(&display, &file_provider, STYLE_PROVIDER_PRIORITY_USER + 2);
            new_providers.push(file_provider);
        }

        ACTIVE_CSS_PROVIDERS.with(|providers| {
            *providers.borrow_mut() = new_providers;
        });

        debug!("Theme service: applied {} CSS provider(s) on GTK main thread", provider_count);
    });

    if let Ok(mut state_guard) = state.write() {
        state_guard.effective_mode = effective_mode;
        state_guard.current_theme = Some(theme_name);
        state_guard.applied_provider_count = provider_count;
    }

    if let Some(wallpaper_name) = wallpaper_theme {
        if let Some(core_context) = _core_context {
            send_wallpaper_coupling(core_context, _meta, &wallpaper_name);
        }
    }
}

fn send_wallpaper_coupling(core_context: &FfiCoreContext, meta: &PluginMeta, wallpaper_theme_name: &str) {
    let select_command = WallpaperCommandMessage::select_theme(wallpaper_theme_name);
    let payload_ptr = box_payload(select_command);
    let envelope = FfiEnvelope::builder()
        .sender_id(meta.id.clone())
        .target_instance_id("")
        .topic(<WallpaperCommandMessage as MessageTopic>::topic())
        .type_id(WallpaperCommandMessage::TYPE_ID)
        .payload(payload_ptr)
        .destroy_payload(Some(default_destroy_payload))
        .clone_payload(Some(default_clone_payload::<WallpaperCommandMessage>))
        .build();
    core_context.send_message(envelope);

    let start_command = WallpaperCommandMessage::start_selected();
    let payload_ptr = box_payload(start_command);
    let envelope = FfiEnvelope::builder()
        .sender_id(meta.id.clone())
        .target_instance_id("")
        .topic(<WallpaperCommandMessage as MessageTopic>::topic())
        .type_id(WallpaperCommandMessage::TYPE_ID)
        .payload(payload_ptr)
        .destroy_payload(Some(default_destroy_payload))
        .clone_payload(Some(default_clone_payload::<WallpaperCommandMessage>))
        .build();
    core_context.send_message(envelope);

    debug!("Theme service: sent wallpaper coupling for '{}'", wallpaper_theme_name);
}

fn add_theme_to_config(config_path: &str, theme: &Theme) {
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
    debug!("Theme service: added theme '{}' to {}", theme.name, config_path);
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
    debug!("Theme service: removed theme '{}' from {}", name, config_path);
}

fn broadcast_status(meta: &PluginMeta, core_context: &Option<FfiCoreContext>, state: &Arc<RwLock<ThemeState>>) {
    let (selected_index, current_theme, effective_mode, themes) = match state.read() {
        Ok(s) => (s.selected_theme_index, s.current_theme.clone(), s.effective_mode, s.themes.clone()),
        Err(e) => {
            error!("Theme service: state lock poisoned: {e}");
            (0, None, ThemeMode::Dark, Vec::new())
        }
    };

    let theme_infos: stabby::vec::Vec<ThemeInfo> = {
        let mut infos = stabby::vec::Vec::with_capacity(themes.len());
        for theme in &themes {
            let colors_stabby = ThemeColorsStabby::from(&theme.colors);
            let info = ThemeInfo {
                name: theme.name.as_str().into(),
                description: theme.description.as_str().into(),
                preview_icon: theme.preview_icon.as_str().into(),
                preview_image_path: theme.preview_image_path.as_str().into(),
                colors: colors_stabby,
                mode: theme.mode,
                has_wallpaper: theme.wallpaper_theme.is_some(),
            };
            infos.push(info);
        }
        infos
    };

    let current_theme_stabby: stabby::option::Option<stabby::string::String> = current_theme.map(|name| name.into()).into();

    let msg = ThemeStatusMessage {
        themes: theme_infos,
        current_theme: current_theme_stabby,
        last_updated: chrono_now_iso().into(),
        selected_theme_index: selected_index as u32,
        effective_mode,
    };

    let payload_ptr = box_payload(msg.clone());
    let envelope = FfiEnvelope::builder()
        .sender_id(meta.id.clone())
        .target_instance_id("")
        .topic(TOPIC_STATUS)
        .type_id(ThemeStatusMessage::TYPE_ID)
        .payload(payload_ptr)
        .destroy_payload(Some(default_destroy_payload))
        .clone_payload(Some(default_clone_payload::<ThemeStatusMessage>))
        .build();
    if let Some(context) = core_context {
        context.send_message(envelope);
    }
}

fn chrono_now_iso() -> String {
    use std::time::SystemTime;
    let secs = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    format!("t:{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_state_default() {
        let state = ThemeState::default();
        assert!(state.current_theme.is_none());
        assert!(state.themes.is_empty());
        assert_eq!(state.selected_theme_index, 0);
    }
}
