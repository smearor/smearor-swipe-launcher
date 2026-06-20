use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::JsonConverterRegistry;
use smearor_swipe_launcher_plugin_api::JsonConvertible;
use stabby::option::Option as StabbyOption;

use crate::ExecDispatchMessage;
use crate::HyprlandDirection;
use crate::HyprlandDispatchMessage;
use crate::HyprlandFullscreenType;
use crate::HyprlandWorkspaceIdentifierKind;
use crate::HyprlandWorkspaceIdentifierWithSpecial;
use crate::KillActiveWindowDispatchMessage;
use crate::MoveFocusDispatchMessage;
use crate::ToggleFullscreenDispatchMessage;
use crate::WorkspaceDispatchMessage;

fn parse_direction(value: &serde_json::Value) -> HyprlandDirection {
    match value.as_str() {
        Some("Down") => HyprlandDirection::Down,
        Some("Left") => HyprlandDirection::Left,
        Some("Right") => HyprlandDirection::Right,
        _ => HyprlandDirection::Up,
    }
}

fn parse_fullscreen_type(value: &serde_json::Value) -> HyprlandFullscreenType {
    match value.as_str() {
        Some("Real") => HyprlandFullscreenType::Real,
        Some("Maximize") => HyprlandFullscreenType::Maximize,
        _ => HyprlandFullscreenType::NoParam,
    }
}

fn parse_workspace_identifier_kind(value: &serde_json::Value) -> HyprlandWorkspaceIdentifierKind {
    match value.as_str() {
        Some("Id") => HyprlandWorkspaceIdentifierKind::Id,
        Some("Relative") => HyprlandWorkspaceIdentifierKind::Relative,
        Some("RelativeMonitor") => HyprlandWorkspaceIdentifierKind::RelativeMonitor,
        Some("RelativeMonitorIncludingEmpty") => HyprlandWorkspaceIdentifierKind::RelativeMonitorIncludingEmpty,
        Some("RelativeOpen") => HyprlandWorkspaceIdentifierKind::RelativeOpen,
        Some("Previous") => HyprlandWorkspaceIdentifierKind::Previous,
        Some("Empty") => HyprlandWorkspaceIdentifierKind::Empty,
        Some("Name") => HyprlandWorkspaceIdentifierKind::Name,
        Some("Special") => HyprlandWorkspaceIdentifierKind::Special,
        _ => HyprlandWorkspaceIdentifierKind::Previous,
    }
}

fn parse_stabby_string(value: &serde_json::Value) -> StabbyOption<stabby::string::String> {
    value
        .as_str()
        .map(|text| stabby::string::String::from(text))
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None())
}

fn parse_workspace_identifier(value: &serde_json::Value) -> HyprlandWorkspaceIdentifierWithSpecial {
    HyprlandWorkspaceIdentifierWithSpecial {
        kind: parse_workspace_identifier_kind(value.get("kind").unwrap_or(&serde_json::Value::Null)),
        id: value.get("id").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        name: parse_stabby_string(value.get("name").unwrap_or(&serde_json::Value::Null)),
        special_name: parse_stabby_string(value.get("special_name").unwrap_or(&serde_json::Value::Null)),
    }
}

smearor_swipe_launcher_plugin_api::impl_json_convertible!(WorkspaceDispatchMessageConverter, WorkspaceDispatchMessage, |json: serde_json::Value| {
    WorkspaceDispatchMessage {
        identifier: parse_workspace_identifier(json.get("identifier").unwrap_or(&serde_json::Value::Null)),
    }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(ExecDispatchMessageConverter, ExecDispatchMessage, |json: serde_json::Value| {
    ExecDispatchMessage {
        command: json.get("command").and_then(|v| v.as_str()).unwrap_or("").to_string(),
    }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(
    KillActiveWindowDispatchMessageConverter,
    KillActiveWindowDispatchMessage,
    |_json: serde_json::Value| { KillActiveWindowDispatchMessage }
);

smearor_swipe_launcher_plugin_api::impl_json_convertible!(MoveFocusDispatchMessageConverter, MoveFocusDispatchMessage, |json: serde_json::Value| {
    MoveFocusDispatchMessage {
        direction: parse_direction(json.get("direction").unwrap_or(&serde_json::Value::Null)),
    }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(
    ToggleFullscreenDispatchMessageConverter,
    ToggleFullscreenDispatchMessage,
    |json: serde_json::Value| {
        ToggleFullscreenDispatchMessage {
            fullscreen_type: parse_fullscreen_type(json.get("fullscreen_type").unwrap_or(&serde_json::Value::Null)),
        }
    }
);

smearor_swipe_launcher_plugin_api::impl_json_convertible!(HyprlandDispatchMessageConverter, HyprlandDispatchMessage, |json: serde_json::Value| {
    use crate::HyprlandDispatchActionKind;

    let kind = match json.get("kind").and_then(|v| v.as_str()) {
        Some("KillActiveWindow") => HyprlandDispatchActionKind::KillActiveWindow,
        Some("MoveFocus") => HyprlandDispatchActionKind::MoveFocus,
        Some("ToggleFullscreen") => HyprlandDispatchActionKind::ToggleFullscreen,
        Some("Workspace") => HyprlandDispatchActionKind::Workspace,
        _ => HyprlandDispatchActionKind::Exec,
    };
    let exec = if kind == HyprlandDispatchActionKind::Exec {
        json.get("command")
            .and_then(|v| v.as_str())
            .map(|command| crate::ExecDispatchMessageStabby {
                command: stabby::string::String::from(command),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let workspace = if kind == HyprlandDispatchActionKind::Workspace {
        json.get("identifier")
            .map(|value| crate::WorkspaceDispatchMessageStabby {
                identifier: parse_workspace_identifier(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let move_focus = if kind == HyprlandDispatchActionKind::MoveFocus {
        json.get("direction")
            .map(|value| crate::MoveFocusDispatchMessageStabby {
                direction: parse_direction(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let toggle_fullscreen = if kind == HyprlandDispatchActionKind::ToggleFullscreen {
        json.get("fullscreen_type")
            .map(|value| crate::ToggleFullscreenDispatchMessageStabby {
                fullscreen_type: parse_fullscreen_type(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    HyprlandDispatchMessage {
        kind,
        exec,
        kill_active_window: StabbyOption::None(),
        move_focus,
        toggle_fullscreen,
        workspace,
    }
});

/// Register all JSON converter implementations for Hyprland messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    HyprlandDispatchMessageConverter::register_in_host(context);
}

/// Register all JSON converter implementations for Hyprland messages directly in a registry.
///
/// Call this once during host application startup (e.g. inside `AreaManager::new`).
pub fn register_json_converters_in_registry(registry: &JsonConverterRegistry) {
    HyprlandDispatchMessageConverter::register_json_converter(registry);
}
