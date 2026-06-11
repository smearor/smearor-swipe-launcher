use crate::config::AppLauncherConfig;
use crate::desktop_entry::DesktopEntry;
use gtk4::prelude::WidgetExt;
use smearor_app_launcher_model::DesktopFileCommandMessage;
use smearor_app_launcher_model::DesktopFileStatus;
use smearor_app_launcher_model::DesktopFileStatusMessage;
use smearor_app_launcher_model::TOPIC_STATUS;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::PluginMetaRaw;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::debug;
use tracing::error;
use tracing::info;

pub struct AppLauncherWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: AppLauncherConfig,
    pub app_name: String,
    pub icon_name: String,
    pub led_indicator: Arc<RwLock<Option<gtk4::Box>>>,
}

impl AppLauncherWidget {
    pub fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionError> {
        debug!("AppLauncherWidget config: {config:?}");
        let meta_raw = PluginMetaRaw::try_from(&config)?;
        let config = AppLauncherConfig::parse(&config.config).map_err(|e| PluginConstructionError::FailedToParseWidgetConfig(e.to_string().into()))?;
        let mut app_name = meta_raw.display_name.to_string();
        let mut icon_name = meta_raw.icon_name.unwrap_or_default().to_string();

        // Parse `.desktop` file
        if let Some(entry) = DesktopEntry::parse(&config.desktop_file_path) {
            app_name = entry.name;
            icon_name = entry.icon;
        } else {
            error!("Could not load .desktop file at: {}", config.desktop_file_path);
        }

        if icon_name.is_empty() {
            icon_name = "system-run".to_string(); // fallback
        }

        Ok(AppLauncherWidget {
            meta: PluginMeta::new(meta_raw.id, app_name.clone(), Some(icon_name.clone())),
            config,
            app_name,
            icon_name,
            core_context,
            led_indicator: Arc::new(RwLock::new(None)),
        })
    }
}

impl MessageHandler<FfiEnvelopePayload<DesktopFileStatusMessage>> for AppLauncherWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<DesktopFileStatusMessage>) {
        if message.desktop_file != self.config.desktop_file_path {
            return;
        }
        info!("AppLauncher Widget {} status updated for {}: {:?}", self.meta.id, message.desktop_file, message.status);
        if let Ok(guard) = self.led_indicator.read() {
            if let Some(led) = guard.as_ref() {
                match message.status {
                    DesktopFileStatus::Running => {
                        led.remove_css_class("led-unlit");
                        led.add_css_class("led-lit");
                    }
                    DesktopFileStatus::Stopped => {
                        led.remove_css_class("led-lit");
                        led.add_css_class("led-unlit");
                    }
                }
            }
        }
    }

    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_STATUS
    }
}

impl MessageBroadcaster<DesktopFileCommandMessage> for AppLauncherWidget {}

impl PluginMetaGetter for AppLauncherWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for AppLauncherWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}
