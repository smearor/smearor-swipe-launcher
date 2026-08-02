//! MCP prompt definitions and resolution helpers.

use rust_mcp_sdk::schema::ContentBlock;
use rust_mcp_sdk::schema::GetPromptResult;
use rust_mcp_sdk::schema::Prompt;
use rust_mcp_sdk::schema::PromptArgument;
use rust_mcp_sdk::schema::PromptMessage;
use rust_mcp_sdk::schema::Role;
use rust_mcp_sdk::schema::TextContent;
use std::collections::BTreeMap;

/// Built-in prompt definition exposed by the MCP server.
pub struct PromptDefinition {
    pub name: String,
    pub description: String,
    pub arguments_schema: serde_json::Value,
    pub handler: PromptHandler,
}

/// Prompt handler signature.
pub type PromptHandler = Box<dyn Fn(&str, Option<&BTreeMap<String, String>>) -> Result<String, String> + Send + Sync>;

/// Build the list of core prompts available from the MCP server.
pub fn core_prompts() -> Vec<PromptDefinition> {
    vec![
        PromptDefinition {
            name: "launcher_overview".to_string(),
            description: "Returns a system message describing the launcher and all available MCP capabilities.".to_string(),
            arguments_schema: serde_json::json!({"type": "object", "properties": {}}),
            handler: Box::new(|_name, _args| {
                Ok("You are interacting with the Smearor Swipe Launcher. Use 'list_all_areas' to discover available areas, 'open_area' to open them, and 'send_message' to communicate with widgets and services. Resources and tools are dynamically registered by plugins.".to_string())
            }),
        },
        PromptDefinition {
            name: "area_control_help".to_string(),
            description: "Returns instructions for controlling a specific area.".to_string(),
            arguments_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "area_id": { "type": "string", "description": "The area to get control instructions for" }
                },
                "required": ["area_id"]
            }),
            handler: Box::new(|_name, args| {
                let area_id = args.and_then(|a| a.get("area_id").cloned()).unwrap_or_else(|| "<area_id>".to_string());
                Ok(format!(
                    "To control the area '{area_id}', use the following tools:\n\
                     - open_area: open the area by ID\n\
                     - close_area: close the area\n\
                     - toggle_area: toggle visibility\n\
                     - focus_area: set keyboard focus\n\
                     - get_area_config: retrieve the area's configuration as JSON\n"
                ))
            }),
        },
        PromptDefinition {
            name: "broker_message_guide".to_string(),
            description: "Returns a guide for using send_message with the central message broker.".to_string(),
            arguments_schema: serde_json::json!({"type": "object", "properties": {}}),
            handler: Box::new(|_name, _args| {
                Ok("The central message broker allows publishing JSON payloads to named topics.\n\
                   Use the 'send_message' tool with parameters:\n\
                   - topic: the broker topic name (string)\n\
                   - payload: a JSON object to publish\n\
                   - target_instance_id: optional target widget/service instance ID\n\
                   Widgets and services subscribe to topics and react to incoming messages."
                    .to_string())
            }),
        },
        PromptDefinition {
            name: "tool_shortcut_guide".to_string(),
            description: "Returns a shortcut map for common user requests to avoid unnecessary tool discovery.".to_string(),
            arguments_schema: serde_json::json!({"type": "object", "properties": {}}),
            handler: Box::new(|_name, _args| {
                Ok("Common user requests and their direct tool shortcuts:\n\
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
                   prompts/list or tools/list when the user's request does not match any shortcut."
                    .to_string())
            }),
        },
    ]
}

/// Convert a serde_json::Value arguments schema to SDK PromptArgument list.
pub fn schema_to_prompt_arguments(schema: &serde_json::Value) -> Vec<PromptArgument> {
    let Some(props) = schema.get("properties").and_then(|p| p.as_object()) else {
        return vec![];
    };
    let required: Vec<String> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    props
        .iter()
        .map(|(name, value)| {
            let description = value.get("description").and_then(|d| d.as_str()).map(String::from);
            let is_required = required.iter().any(|r| r == name);
            PromptArgument {
                description,
                name: name.clone(),
                required: if is_required { Some(true) } else { None },
                title: None,
            }
        })
        .collect()
}

/// Resolve a core prompt by name and return a GetPromptResult for the SDK.
pub fn get_prompt_sdk(prompts: &[PromptDefinition], name: &str, arguments: &Option<BTreeMap<String, String>>) -> Result<GetPromptResult, String> {
    let Some(prompt) = prompts.iter().find(|p| p.name == name) else {
        return Err(format!("Prompt {name} not found"));
    };
    let content = (prompt.handler)(name, arguments.as_ref())?;
    Ok(GetPromptResult {
        description: Some(prompt.description.clone()),
        messages: vec![PromptMessage {
            content: ContentBlock::TextContent(TextContent::new(content, None, None)),
            role: Role::User,
        }],
        meta: None,
    })
}

/// Convert a PromptDefinition to an SDK Prompt for listing.
pub fn prompt_to_sdk(prompt: &PromptDefinition) -> Prompt {
    Prompt {
        name: prompt.name.clone(),
        description: Some(prompt.description.clone()),
        arguments: schema_to_prompt_arguments(&prompt.arguments_schema),
        icons: vec![],
        meta: None,
        title: None,
    }
}
