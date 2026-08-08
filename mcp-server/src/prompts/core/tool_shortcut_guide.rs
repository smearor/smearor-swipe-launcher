use crate::prompts::creator::PromptDefinitionCreator;
use crate::prompts::creator::static_prompt_handler;
use crate::prompts::definition::PromptHandler;

/// Prompt returning a shortcut map for common user requests to avoid unnecessary tool discovery.
pub struct ToolShortcutGuidePrompt;

impl PromptDefinitionCreator for ToolShortcutGuidePrompt {
    fn prompt_name() -> &'static str {
        "tool_shortcut_guide"
    }
    fn prompt_description() -> &'static str {
        "Returns a shortcut map for common user requests to avoid unnecessary tool discovery."
    }
    fn prompt_arguments_schema() -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}})
    }
    fn prompt_handler() -> PromptHandler {
        static_prompt_handler(
            "Common user requests and their direct tool shortcuts:\n\
                   \n\
                   Audio:\n\
                   - 'Lauter' / 'Volume up' → audio_volume_up\n\
                   - 'Leiser' / 'Volume down' → audio_volume_down\n\
                   - 'Stumm' / 'Mute' → audio_toggle_mute\n\
                   \n\
                   MPRIS:\n\
                   - 'Pause' / 'Play' → mpris_toggle_play_pause\n\
                   - 'Nächster Titel' / 'Next track' → mpris_next_track\n\
                   - 'Vorheriger Titel' / 'Previous track' → mpris_previous_track\n\
                   \n\
                   Power:\n\
                   - 'Herunterfahren' / 'Shutdown' → system_power_action { action: 'shutdown' }\n\
                   - 'Neustart' / 'Reboot' → system_power_action { action: 'reboot' }\n\
                   - 'Sperren' / 'Lock' → system_power_action { action: 'lock' }\n\
                   \n\
                   Weather:\n\
                   - 'Wetter' / 'Weather' → weather_get_forecast\n\
                   - 'Wettervorhersage' / 'Forecast' → weather_get_forecast\n\
                   \n\
                   Network:\n\
                   - 'WLAN an' / 'WiFi on' → network_toggle_radio { technology: 'wifi', enabled: true }\n\
                   - 'WLAN aus' / 'WiFi off' → network_toggle_radio { technology: 'wifi', enabled: false }\n\
                   \n\
                   Sysinfo:\n\
                   - 'Systemstatus' / 'System health' → read resources sysinfo://cpu, sysinfo://memory, sysinfo://temperature-components\n\
                   \n\
                   Launcher:\n\
                   - 'Öffne <area>' / 'Open <area>' → open_area { area_id: '<area>' }\n\
                   - 'Schließe <area>' / 'Close <area>' → close_area { area_id: '<area>' }\n\
                   \n\
                   Use these shortcuts directly instead of listing all tools first. Only fall back to\n\
                   prompts/list or tools/list when the user's request does not match any shortcut.",
        )
    }
}
