use crate::config::AppLauncherConfig;
use crate::desktop_entry::DesktopEntry;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaRaw;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::debug;
use tracing::error;

pub struct AppLauncherWidget {
    pub meta: PluginMeta,
    pub config: AppLauncherConfig,
    pub app_name: String,
    pub icon_name: String,
    pub core_context: Option<FfiCoreContext>,
    pub led_indicator: Arc<RwLock<Option<gtk4::Box>>>,
}

impl AppLauncherWidget {
    pub fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionError> {
        debug!("AppLauncherWidget config: {config:?}");
        let meta_raw: PluginMetaRaw =
            serde_json::from_value(config.config.clone()).map_err(|e| PluginConstructionError::FailedToParseMetaData(e.to_string().into()))?;
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
