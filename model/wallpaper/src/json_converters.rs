use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use stabby::option::Option as StabbyOption;

use crate::MonitorProcess;
use crate::WallpaperCommandAction;
use crate::WallpaperCommandMessage;
use crate::WallpaperStatusMessage;
use crate::WallpaperThemeInfo;
use crate::WallpaperType;

fn parse_wallpaper_command_action(value: &serde_json::Value) -> WallpaperCommandAction {
    match value.as_str() {
        Some("StartSelected") => WallpaperCommandAction::StartSelected,
        Some("StopCurrent") => WallpaperCommandAction::StopCurrent,
        Some("Refresh") => WallpaperCommandAction::Refresh,
        _ => WallpaperCommandAction::SelectTheme,
    }
}

fn parse_wallpaper_type(value: &serde_json::Value) -> WallpaperType {
    match value.as_str() {
        Some("Video") => WallpaperType::Video,
        Some("Image") => WallpaperType::Image,
        _ => WallpaperType::Application,
    }
}

fn parse_theme_info(value: &serde_json::Value) -> WallpaperThemeInfo {
    let name = value.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let description = value.get("description").and_then(|v| v.as_str()).unwrap_or("");
    let preview_image_path = value.get("preview_image_path").and_then(|v| v.as_str()).unwrap_or("");
    let wallpaper_type = parse_wallpaper_type(value.get("wallpaper_type").unwrap_or(&serde_json::Value::Null));
    WallpaperThemeInfo::new(name, description, preview_image_path, wallpaper_type)
}

fn parse_monitor_process(value: &serde_json::Value) -> MonitorProcess {
    let monitor = value.get("monitor").and_then(|v| v.as_str()).unwrap_or("");
    let process_id = value.get("process_id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    MonitorProcess {
        monitor: monitor.into(),
        process_id,
    }
}

smearor_swipe_launcher_plugin_api::impl_json_convertible!(WallpaperCommandMessageConverter, WallpaperCommandMessage, |json: serde_json::Value| {
    let action = parse_wallpaper_command_action(json.get("action").unwrap_or(&serde_json::Value::Null));
    let theme_name = json.get("theme_name").and_then(|v| v.as_str()).unwrap_or("");
    WallpaperCommandMessage::new(action, theme_name)
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(WallpaperStatusMessageConverter, WallpaperStatusMessage, |json: serde_json::Value| {
    let current_theme = json
        .get("current_theme")
        .and_then(|v| if v.is_null() { None } else { v.as_str() })
        .map(stabby::string::String::from)
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None());

    let current_processes: stabby::vec::Vec<MonitorProcess> = {
        let mut v = stabby::vec::Vec::new();
        if let Some(arr) = json.get("current_processes").and_then(|v| v.as_array()) {
            for item in arr {
                v.push(parse_monitor_process(item));
            }
        }
        v
    };

    let selected_theme = json
        .get("selected_theme")
        .and_then(|v| if v.is_null() { None } else { v.as_str() })
        .map(stabby::string::String::from)
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None());

    let themes: stabby::vec::Vec<WallpaperThemeInfo> = {
        let mut v = stabby::vec::Vec::new();
        if let Some(arr) = json.get("themes").and_then(|v| v.as_array()) {
            for item in arr {
                v.push(parse_theme_info(item));
            }
        }
        v
    };

    let selected_theme_index = json.get("selected_theme_index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

    WallpaperStatusMessage {
        current_theme,
        current_processes,
        selected_theme,
        themes,
        selected_theme_index,
    }
});

/// Register all JSON converter implementations for wallpaper messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    WallpaperCommandMessageConverter::register_in_host(context);
    WallpaperStatusMessageConverter::register_in_host(context);
}
