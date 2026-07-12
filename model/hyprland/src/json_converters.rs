use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::JsonConverterRegistry;
use smearor_swipe_launcher_plugin_api::JsonConvertible;
use stabby::option::Option as StabbyOption;

use crate::CloseWindowDispatchMessage;
use crate::CycleWindowDispatchMessage;
use crate::ExecDispatchMessage;
use crate::FocusMonitorDispatchMessage;
use crate::FocusWindowDispatchMessage;
use crate::HyprlandColor;
use crate::HyprlandCorner;
use crate::HyprlandCycleDirection;
use crate::HyprlandDirection;
use crate::HyprlandDispatchMessage;
use crate::HyprlandFocusMasterParam;
use crate::HyprlandFullscreenType;
use crate::HyprlandLockType;
use crate::HyprlandMonitorIdentifier;
use crate::HyprlandMonitorIdentifierKind;
use crate::HyprlandNotifyIcon;
use crate::HyprlandOutputBackend;
use crate::HyprlandPosition;
use crate::HyprlandPositionKind;
use crate::HyprlandPropType;
use crate::HyprlandPropTypeKind;
use crate::HyprlandSwapWithMasterParam;
use crate::HyprlandSwitchXkbLayoutCmd;
use crate::HyprlandSwitchXkbLayoutCmdKind;
use crate::HyprlandWindowIdentifier;
use crate::HyprlandWindowMove;
use crate::HyprlandWindowMoveKind;
use crate::HyprlandWindowSwitchDirection;
use crate::HyprlandWorkspaceIdentifier;
use crate::HyprlandWorkspaceIdentifierKind;
use crate::HyprlandWorkspaceIdentifierWithSpecial;
use crate::HyprlandWorkspaceOptions;
use crate::KillActiveWindowDispatchMessage;
use crate::KillCommandMessage;
use crate::MoveCursorDispatchMessage;
use crate::MoveCursorToCornerDispatchMessage;
use crate::MoveFocusDispatchMessage;
use crate::MoveToWorkspaceDispatchMessage;
use crate::NotifyCommandMessage;
use crate::OutputCreateCommandMessage;
use crate::OutputRemoveCommandMessage;
use crate::PluginLoadCommandMessage;
use crate::PluginUnloadCommandMessage;
use crate::ReloadCommandMessage;
use crate::SetCursorCommandMessage;
use crate::SetErrorCommandMessage;
use crate::SetPropCommandMessage;
use crate::SwapWindowDispatchMessage;
use crate::SwitchXkbLayoutCommandMessage;
use crate::ToggleFloatingDispatchMessage;
use crate::ToggleFullscreenDispatchMessage;
use crate::ToggleSpecialWorkspaceDispatchMessage;
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

fn parse_cycle_direction(value: &serde_json::Value) -> HyprlandCycleDirection {
    match value.as_str() {
        Some("Previous") => HyprlandCycleDirection::Previous,
        _ => HyprlandCycleDirection::Next,
    }
}

fn parse_corner(value: &serde_json::Value) -> HyprlandCorner {
    match value.as_str() {
        Some("BottomRight") => HyprlandCorner::BottomRight,
        Some("TopRight") => HyprlandCorner::TopRight,
        Some("TopLeft") => HyprlandCorner::TopLeft,
        _ => HyprlandCorner::BottomLeft,
    }
}

fn parse_window_identifier(value: &serde_json::Value) -> HyprlandWindowIdentifier {
    let kind = value.get("kind").and_then(|v| v.as_str()).unwrap_or("ProcessId");
    match kind {
        "Address" => HyprlandWindowIdentifier::Address(stabby::string::String::from(value.get("address").and_then(|v| v.as_str()).unwrap_or(""))),
        "ClassRegularExpression" => {
            HyprlandWindowIdentifier::ClassRegularExpression(stabby::string::String::from(value.get("class_regex").and_then(|v| v.as_str()).unwrap_or("")))
        }
        "Title" => HyprlandWindowIdentifier::Title(stabby::string::String::from(value.get("title").and_then(|v| v.as_str()).unwrap_or(""))),
        _ => HyprlandWindowIdentifier::ProcessId(value.get("process_id").and_then(|v| v.as_i64()).unwrap_or(0) as u32),
    }
}

fn parse_monitor_identifier_kind(value: &serde_json::Value) -> HyprlandMonitorIdentifierKind {
    match value.as_str() {
        Some("Direction") => HyprlandMonitorIdentifierKind::Direction,
        Some("Id") => HyprlandMonitorIdentifierKind::Id,
        Some("Name") => HyprlandMonitorIdentifierKind::Name,
        Some("Relative") => HyprlandMonitorIdentifierKind::Relative,
        _ => HyprlandMonitorIdentifierKind::Current,
    }
}

fn parse_monitor_identifier(value: &serde_json::Value) -> HyprlandMonitorIdentifier {
    HyprlandMonitorIdentifier {
        kind: parse_monitor_identifier_kind(value.get("kind").unwrap_or(&serde_json::Value::Null)),
        direction: parse_direction(value.get("direction").unwrap_or(&serde_json::Value::Null)),
        id: value.get("id").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        name: parse_stabby_string(value.get("name").unwrap_or(&serde_json::Value::Null)),
        relative: value.get("relative").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
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

fn parse_workspace_identifier_no_special(value: &serde_json::Value) -> HyprlandWorkspaceIdentifier {
    let kind = value.get("kind").and_then(|v| v.as_str()).unwrap_or("Previous");
    match kind {
        "Id" => HyprlandWorkspaceIdentifier::Id(value.get("id").and_then(|v| v.as_i64()).unwrap_or(0) as i32),
        "Relative" => HyprlandWorkspaceIdentifier::Relative(value.get("relative").and_then(|v| v.as_i64()).unwrap_or(0) as i32),
        "RelativeMonitor" => HyprlandWorkspaceIdentifier::RelativeMonitor(value.get("relative").and_then(|v| v.as_i64()).unwrap_or(0) as i32),
        "RelativeMonitorIncludingEmpty" => {
            HyprlandWorkspaceIdentifier::RelativeMonitorIncludingEmpty(value.get("relative").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        }
        "RelativeOpen" => HyprlandWorkspaceIdentifier::RelativeOpen(value.get("relative").and_then(|v| v.as_i64()).unwrap_or(0) as i32),
        "Empty" => HyprlandWorkspaceIdentifier::Empty(),
        "Name" => HyprlandWorkspaceIdentifier::Name(stabby::string::String::from(value.get("name").and_then(|v| v.as_str()).unwrap_or(""))),
        _ => HyprlandWorkspaceIdentifier::Previous(),
    }
}

fn parse_workspace_options(value: &serde_json::Value) -> HyprlandWorkspaceOptions {
    match value.as_str() {
        Some("AllFloat") => HyprlandWorkspaceOptions::AllFloat,
        _ => HyprlandWorkspaceOptions::AllPseudo,
    }
}

fn parse_lock_type(value: &serde_json::Value) -> HyprlandLockType {
    match value.as_str() {
        Some("Unlock") => HyprlandLockType::Unlock,
        Some("ToggleLock") => HyprlandLockType::ToggleLock,
        _ => HyprlandLockType::Lock,
    }
}

fn parse_window_switch_direction(value: &serde_json::Value) -> HyprlandWindowSwitchDirection {
    match value.as_str() {
        Some("Forward") => HyprlandWindowSwitchDirection::Forward,
        _ => HyprlandWindowSwitchDirection::Back,
    }
}

fn parse_swap_with_master_param(value: &serde_json::Value) -> HyprlandSwapWithMasterParam {
    match value.as_str() {
        Some("Child") => HyprlandSwapWithMasterParam::Child,
        Some("Auto") => HyprlandSwapWithMasterParam::Auto,
        _ => HyprlandSwapWithMasterParam::Master,
    }
}

fn parse_focus_master_param(value: &serde_json::Value) -> HyprlandFocusMasterParam {
    match value.as_str() {
        Some("Auto") => HyprlandFocusMasterParam::Auto,
        _ => HyprlandFocusMasterParam::Master,
    }
}

fn parse_position(value: &serde_json::Value) -> HyprlandPosition {
    HyprlandPosition {
        kind: match value.get("kind").and_then(|v| v.as_str()) {
            Some("Exact") => HyprlandPositionKind::Exact,
            _ => HyprlandPositionKind::Delta,
        },
        x: value.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i16,
        y: value.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i16,
    }
}

fn parse_window_move(value: &serde_json::Value) -> HyprlandWindowMove {
    let kind = match value.get("kind").and_then(|v| v.as_str()) {
        Some("Monitor") => HyprlandWindowMoveKind::Monitor,
        _ => HyprlandWindowMoveKind::Direction,
    };
    HyprlandWindowMove {
        kind,
        direction: parse_direction(value.get("direction").unwrap_or(&serde_json::Value::Null)),
        monitor: parse_monitor_identifier(value.get("monitor").unwrap_or(&serde_json::Value::Null)),
    }
}

fn parse_notify_icon(value: &serde_json::Value) -> HyprlandNotifyIcon {
    match value.as_str() {
        Some("Info") => HyprlandNotifyIcon::Info,
        Some("Hint") => HyprlandNotifyIcon::Hint,
        Some("Error") => HyprlandNotifyIcon::Error,
        Some("Confused") => HyprlandNotifyIcon::Confused,
        Some("Ok") => HyprlandNotifyIcon::Ok,
        Some("NoIcon") => HyprlandNotifyIcon::NoIcon,
        _ => HyprlandNotifyIcon::Warning,
    }
}

fn parse_color(value: &serde_json::Value) -> HyprlandColor {
    HyprlandColor {
        red: value.get("red").and_then(|v| v.as_i64()).unwrap_or(0) as u8,
        green: value.get("green").and_then(|v| v.as_i64()).unwrap_or(0) as u8,
        blue: value.get("blue").and_then(|v| v.as_i64()).unwrap_or(0) as u8,
        alpha: value.get("alpha").and_then(|v| v.as_i64()).unwrap_or(255) as u8,
    }
}

fn parse_output_backend(value: &serde_json::Value) -> HyprlandOutputBackend {
    match value.as_str() {
        Some("X11") => HyprlandOutputBackend::X11,
        Some("Headless") => HyprlandOutputBackend::Headless,
        Some("Auto") => HyprlandOutputBackend::Auto,
        _ => HyprlandOutputBackend::Wayland,
    }
}

fn parse_switch_xkb_layout_cmd(value: &serde_json::Value) -> HyprlandSwitchXkbLayoutCmd {
    let kind = match value.get("kind").and_then(|v| v.as_str()) {
        Some("Previous") => HyprlandSwitchXkbLayoutCmdKind::Previous,
        Some("Id") => HyprlandSwitchXkbLayoutCmdKind::Id,
        _ => HyprlandSwitchXkbLayoutCmdKind::Next,
    };
    HyprlandSwitchXkbLayoutCmd {
        kind,
        id: value.get("id").and_then(|v| v.as_i64()).unwrap_or(0) as u8,
    }
}

fn parse_prop_type(value: &serde_json::Value) -> HyprlandPropType {
    let kind = match value.get("kind").and_then(|v| v.as_str()) {
        Some("Rounding") => HyprlandPropTypeKind::Rounding,
        Some("ForceNoBlur") => HyprlandPropTypeKind::ForceNoBlur,
        Some("ForceOpaque") => HyprlandPropTypeKind::ForceOpaque,
        Some("ForceOpaqueOverriden") => HyprlandPropTypeKind::ForceOpaqueOverriden,
        Some("ForceAllowsInput") => HyprlandPropTypeKind::ForceAllowsInput,
        Some("ForceNoAnims") => HyprlandPropTypeKind::ForceNoAnims,
        Some("ForceNoBorder") => HyprlandPropTypeKind::ForceNoBorder,
        Some("ForceNoShadow") => HyprlandPropTypeKind::ForceNoShadow,
        Some("WindowDanceCompat") => HyprlandPropTypeKind::WindowDanceCompat,
        Some("NoMaxSize") => HyprlandPropTypeKind::NoMaxSize,
        Some("DimAround") => HyprlandPropTypeKind::DimAround,
        Some("AlphaOverride") => HyprlandPropTypeKind::AlphaOverride,
        Some("Alpha") => HyprlandPropTypeKind::Alpha,
        Some("AlphaInactiveOverride") => HyprlandPropTypeKind::AlphaInactiveOverride,
        Some("AlphaInactive") => HyprlandPropTypeKind::AlphaInactive,
        Some("ActiveBorderColor") => HyprlandPropTypeKind::ActiveBorderColor,
        Some("InactiveBorderColor") => HyprlandPropTypeKind::InactiveBorderColor,
        _ => HyprlandPropTypeKind::AnimationStyle,
    };
    HyprlandPropType {
        kind,
        animation_style: parse_stabby_string(value.get("animation_style").unwrap_or(&serde_json::Value::Null)),
        rounding: value.get("rounding").and_then(|v| v.as_i64()).unwrap_or(0),
        value_bool: value.get("value_bool").and_then(|v| v.as_bool()).unwrap_or(false),
        value_float: value.get("value_float").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
        color: parse_color(value.get("color").unwrap_or(&serde_json::Value::Null)),
        locked: value.get("locked").and_then(|v| v.as_bool()).unwrap_or(false),
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
    MoveToWorkspaceDispatchMessageConverter,
    MoveToWorkspaceDispatchMessage,
    |json: serde_json::Value| {
        MoveToWorkspaceDispatchMessage {
            identifier: parse_workspace_identifier(json.get("identifier").unwrap_or(&serde_json::Value::Null)),
        }
    }
);

smearor_swipe_launcher_plugin_api::impl_json_convertible!(ToggleFloatingDispatchMessageConverter, ToggleFloatingDispatchMessage, |_json: serde_json::Value| {
    ToggleFloatingDispatchMessage
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

smearor_swipe_launcher_plugin_api::impl_json_convertible!(CloseWindowDispatchMessageConverter, CloseWindowDispatchMessage, |json: serde_json::Value| {
    CloseWindowDispatchMessage {
        window_identifier: parse_window_identifier(json.get("window_identifier").unwrap_or(&serde_json::Value::Null)),
    }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(FocusWindowDispatchMessageConverter, FocusWindowDispatchMessage, |json: serde_json::Value| {
    FocusWindowDispatchMessage {
        window_identifier: parse_window_identifier(json.get("window_identifier").unwrap_or(&serde_json::Value::Null)),
    }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(FocusMonitorDispatchMessageConverter, FocusMonitorDispatchMessage, |json: serde_json::Value| {
    FocusMonitorDispatchMessage {
        monitor_identifier: parse_monitor_identifier(json.get("monitor_identifier").unwrap_or(&serde_json::Value::Null)),
    }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(CycleWindowDispatchMessageConverter, CycleWindowDispatchMessage, |json: serde_json::Value| {
    CycleWindowDispatchMessage {
        cycle_direction: parse_cycle_direction(json.get("cycle_direction").unwrap_or(&serde_json::Value::Null)),
    }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(SwapWindowDispatchMessageConverter, SwapWindowDispatchMessage, |json: serde_json::Value| {
    SwapWindowDispatchMessage {
        cycle_direction: parse_cycle_direction(json.get("cycle_direction").unwrap_or(&serde_json::Value::Null)),
    }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(
    ToggleSpecialWorkspaceDispatchMessageConverter,
    ToggleSpecialWorkspaceDispatchMessage,
    |json: serde_json::Value| {
        ToggleSpecialWorkspaceDispatchMessage {
            workspace_name: json.get("workspace_name").and_then(|v| v.as_str()).map(|s| s.to_string()),
        }
    }
);

smearor_swipe_launcher_plugin_api::impl_json_convertible!(MoveCursorDispatchMessageConverter, MoveCursorDispatchMessage, |json: serde_json::Value| {
    MoveCursorDispatchMessage {
        x: json.get("x").and_then(|v| v.as_i64()).unwrap_or(0),
        y: json.get("y").and_then(|v| v.as_i64()).unwrap_or(0),
    }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(
    MoveCursorToCornerDispatchMessageConverter,
    MoveCursorToCornerDispatchMessage,
    |json: serde_json::Value| {
        MoveCursorToCornerDispatchMessage {
            corner: parse_corner(json.get("corner").unwrap_or(&serde_json::Value::Null)),
        }
    }
);

smearor_swipe_launcher_plugin_api::impl_json_convertible!(KillCommandMessageConverter, KillCommandMessage, |_json: serde_json::Value| { KillCommandMessage });

smearor_swipe_launcher_plugin_api::impl_json_convertible!(ReloadCommandMessageConverter, ReloadCommandMessage, |_json: serde_json::Value| {
    ReloadCommandMessage
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(NotifyCommandMessageConverter, NotifyCommandMessage, |json: serde_json::Value| {
    NotifyCommandMessage {
        icon: parse_notify_icon(json.get("icon").unwrap_or(&serde_json::Value::Null)),
        time_ms: json.get("time_ms").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        color: parse_color(json.get("color").unwrap_or(&serde_json::Value::Null)),
        message: json.get("message").and_then(|v| v.as_str()).unwrap_or("").to_string(),
    }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(OutputCreateCommandMessageConverter, OutputCreateCommandMessage, |json: serde_json::Value| {
    OutputCreateCommandMessage {
        backend: parse_output_backend(json.get("backend").unwrap_or(&serde_json::Value::Null)),
    }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(OutputRemoveCommandMessageConverter, OutputRemoveCommandMessage, |json: serde_json::Value| {
    OutputRemoveCommandMessage {
        name: json.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
    }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(PluginLoadCommandMessageConverter, PluginLoadCommandMessage, |json: serde_json::Value| {
    PluginLoadCommandMessage {
        path: json.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string(),
    }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(PluginUnloadCommandMessageConverter, PluginUnloadCommandMessage, |json: serde_json::Value| {
    PluginUnloadCommandMessage {
        name: json.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
    }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(SetCursorCommandMessageConverter, SetCursorCommandMessage, |json: serde_json::Value| {
    SetCursorCommandMessage {
        theme: json.get("theme").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        size: json.get("size").and_then(|v| v.as_u64()).unwrap_or(0) as u16,
    }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(SetErrorCommandMessageConverter, SetErrorCommandMessage, |json: serde_json::Value| {
    SetErrorCommandMessage {
        color: parse_color(json.get("color").unwrap_or(&serde_json::Value::Null)),
        message: json.get("message").and_then(|v| v.as_str()).unwrap_or("").to_string(),
    }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(SetPropCommandMessageConverter, SetPropCommandMessage, |json: serde_json::Value| {
    SetPropCommandMessage {
        identifier: json.get("identifier").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        prop: parse_prop_type(json.get("prop").unwrap_or(&serde_json::Value::Null)),
        lock: json.get("lock").and_then(|v| v.as_bool()).unwrap_or(false),
    }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(SwitchXkbLayoutCommandMessageConverter, SwitchXkbLayoutCommandMessage, |json: serde_json::Value| {
    SwitchXkbLayoutCommandMessage {
        device: json.get("device").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        cmd: parse_switch_xkb_layout_cmd(json.get("cmd").unwrap_or(&serde_json::Value::Null)),
    }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(HyprlandDispatchMessageConverter, HyprlandDispatchMessage, |json: serde_json::Value| {
    use crate::HyprlandDispatchActionKind;

    let kind = match json.get("kind").and_then(|v| v.as_str()) {
        Some("AddMaster") => HyprlandDispatchActionKind::AddMaster,
        Some("BringActiveToTop") => HyprlandDispatchActionKind::BringActiveToTop,
        Some("CenterWindow") => HyprlandDispatchActionKind::CenterWindow,
        Some("ChangeGroupActive") => HyprlandDispatchActionKind::ChangeGroupActive,
        Some("ChangeSplitRatio") => HyprlandDispatchActionKind::ChangeSplitRatio,
        Some("CloseWindow") => HyprlandDispatchActionKind::CloseWindow,
        Some("Custom") => HyprlandDispatchActionKind::Custom,
        Some("CycleWindow") => HyprlandDispatchActionKind::CycleWindow,
        Some("Exit") => HyprlandDispatchActionKind::Exit,
        Some("FocusCurrentOrLast") => HyprlandDispatchActionKind::FocusCurrentOrLast,
        Some("FocusMaster") => HyprlandDispatchActionKind::FocusMaster,
        Some("FocusMonitor") => HyprlandDispatchActionKind::FocusMonitor,
        Some("FocusUrgentOrLast") => HyprlandDispatchActionKind::FocusUrgentOrLast,
        Some("FocusWindow") => HyprlandDispatchActionKind::FocusWindow,
        Some("ForceRendererReload") => HyprlandDispatchActionKind::ForceRendererReload,
        Some("Global") => HyprlandDispatchActionKind::Global,
        Some("KillActiveWindow") => HyprlandDispatchActionKind::KillActiveWindow,
        Some("LockGroups") => HyprlandDispatchActionKind::LockGroups,
        Some("MoveActive") => HyprlandDispatchActionKind::MoveActive,
        Some("MoveCursor") => HyprlandDispatchActionKind::MoveCursor,
        Some("MoveCursorToCorner") => HyprlandDispatchActionKind::MoveCursorToCorner,
        Some("MoveCurrentWorkspaceToMonitor") => HyprlandDispatchActionKind::MoveCurrentWorkspaceToMonitor,
        Some("MoveFocusedWindowToWorkspace") => HyprlandDispatchActionKind::MoveFocusedWindowToWorkspace,
        Some("MoveFocusedWindowToWorkspaceSilent") => HyprlandDispatchActionKind::MoveFocusedWindowToWorkspaceSilent,
        Some("MoveFocus") => HyprlandDispatchActionKind::MoveFocus,
        Some("MoveIntoGroup") => HyprlandDispatchActionKind::MoveIntoGroup,
        Some("MoveOutOfGroup") => HyprlandDispatchActionKind::MoveOutOfGroup,
        Some("MoveToWorkspace") => HyprlandDispatchActionKind::MoveToWorkspace,
        Some("MoveToWorkspaceSilent") => HyprlandDispatchActionKind::MoveToWorkspaceSilent,
        Some("MoveWindow") => HyprlandDispatchActionKind::MoveWindow,
        Some("MoveWindowPixel") => HyprlandDispatchActionKind::MoveWindowPixel,
        Some("OrientationBottom") => HyprlandDispatchActionKind::OrientationBottom,
        Some("OrientationCenter") => HyprlandDispatchActionKind::OrientationCenter,
        Some("OrientationLeft") => HyprlandDispatchActionKind::OrientationLeft,
        Some("OrientationNext") => HyprlandDispatchActionKind::OrientationNext,
        Some("OrientationPrev") => HyprlandDispatchActionKind::OrientationPrev,
        Some("OrientationRight") => HyprlandDispatchActionKind::OrientationRight,
        Some("OrientationTop") => HyprlandDispatchActionKind::OrientationTop,
        Some("Pass") => HyprlandDispatchActionKind::Pass,
        Some("RemoveMaster") => HyprlandDispatchActionKind::RemoveMaster,
        Some("RenameWorkspace") => HyprlandDispatchActionKind::RenameWorkspace,
        Some("ResizeActive") => HyprlandDispatchActionKind::ResizeActive,
        Some("ResizeWindowPixel") => HyprlandDispatchActionKind::ResizeWindowPixel,
        Some("SetCursor") => HyprlandDispatchActionKind::SetCursor,
        Some("SwapActiveWorkspaces") => HyprlandDispatchActionKind::SwapActiveWorkspaces,
        Some("SwapWindow") => HyprlandDispatchActionKind::SwapWindow,
        Some("SwapWithMaster") => HyprlandDispatchActionKind::SwapWithMaster,
        Some("ToggleDpms") => HyprlandDispatchActionKind::ToggleDpms,
        Some("ToggleFakeFullscreen") => HyprlandDispatchActionKind::ToggleFakeFullscreen,
        Some("ToggleFloating") => HyprlandDispatchActionKind::ToggleFloating,
        Some("ToggleFullscreen") => HyprlandDispatchActionKind::ToggleFullscreen,
        Some("ToggleGroup") => HyprlandDispatchActionKind::ToggleGroup,
        Some("ToggleOpaque") => HyprlandDispatchActionKind::ToggleOpaque,
        Some("TogglePin") => HyprlandDispatchActionKind::TogglePin,
        Some("TogglePseudo") => HyprlandDispatchActionKind::TogglePseudo,
        Some("ToggleSpecialWorkspace") => HyprlandDispatchActionKind::ToggleSpecialWorkspace,
        Some("ToggleSplit") => HyprlandDispatchActionKind::ToggleSplit,
        Some("Workspace") => HyprlandDispatchActionKind::Workspace,
        Some("WorkspaceOption") => HyprlandDispatchActionKind::WorkspaceOption,
        _ => HyprlandDispatchActionKind::Exec,
    };
    let add_master: StabbyOption<crate::AddMasterDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::AddMaster {
        StabbyOption::Some(crate::AddMasterDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let bring_active_to_top: StabbyOption<crate::BringActiveToTopDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::BringActiveToTop {
        StabbyOption::Some(crate::BringActiveToTopDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let center_window: StabbyOption<crate::CenterWindowDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::CenterWindow {
        StabbyOption::Some(crate::CenterWindowDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let change_group_active = if kind == HyprlandDispatchActionKind::ChangeGroupActive {
        json.get("direction")
            .map(|value| crate::ChangeGroupActiveDispatchMessageStabby {
                direction: parse_window_switch_direction(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let change_split_ratio = if kind == HyprlandDispatchActionKind::ChangeSplitRatio {
        json.get("ratio")
            .and_then(|v| v.as_f64())
            .map(|ratio| crate::ChangeSplitRatioDispatchMessageStabby { ratio: ratio as f32 })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let close_window = if kind == HyprlandDispatchActionKind::CloseWindow {
        json.get("window_identifier")
            .map(|value| crate::CloseWindowDispatchMessageStabby {
                window_identifier: parse_window_identifier(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let custom = if kind == HyprlandDispatchActionKind::Custom {
        json.get("name")
            .and_then(|v| v.as_str())
            .map(|name| crate::CustomDispatchMessageStabby {
                name: stabby::string::String::from(name),
                value: stabby::string::String::from(json.get("value").and_then(|v| v.as_str()).unwrap_or("")),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let cycle_window = if kind == HyprlandDispatchActionKind::CycleWindow {
        json.get("cycle_direction")
            .map(|value| crate::CycleWindowDispatchMessageStabby {
                cycle_direction: parse_cycle_direction(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
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
    let exit: StabbyOption<crate::ExitDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::Exit {
        StabbyOption::Some(crate::ExitDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let focus_current_or_last: StabbyOption<crate::FocusCurrentOrLastDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::FocusCurrentOrLast {
        StabbyOption::Some(crate::FocusCurrentOrLastDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let focus_master = if kind == HyprlandDispatchActionKind::FocusMaster {
        json.get("param")
            .map(|value| crate::FocusMasterDispatchMessageStabby {
                param: parse_focus_master_param(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let focus_monitor = if kind == HyprlandDispatchActionKind::FocusMonitor {
        json.get("monitor_identifier")
            .map(|value| crate::FocusMonitorDispatchMessageStabby {
                monitor_identifier: parse_monitor_identifier(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let focus_urgent_or_last: StabbyOption<crate::FocusUrgentOrLastDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::FocusUrgentOrLast {
        StabbyOption::Some(crate::FocusUrgentOrLastDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let focus_window = if kind == HyprlandDispatchActionKind::FocusWindow {
        json.get("window_identifier")
            .map(|value| crate::FocusWindowDispatchMessageStabby {
                window_identifier: parse_window_identifier(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let force_renderer_reload: StabbyOption<crate::ForceRendererReloadDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::ForceRendererReload {
        StabbyOption::Some(crate::ForceRendererReloadDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let global = if kind == HyprlandDispatchActionKind::Global {
        json.get("key")
            .and_then(|v| v.as_str())
            .map(|key| crate::GlobalDispatchMessageStabby {
                key: stabby::string::String::from(key),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let kill_active_window: StabbyOption<crate::KillActiveWindowDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::KillActiveWindow {
        StabbyOption::Some(crate::KillActiveWindowDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let lock_groups = if kind == HyprlandDispatchActionKind::LockGroups {
        json.get("lock_type")
            .map(|value| crate::LockGroupsDispatchMessageStabby {
                lock_type: parse_lock_type(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let move_active = if kind == HyprlandDispatchActionKind::MoveActive {
        json.get("position")
            .map(|value| crate::MoveActiveDispatchMessageStabby {
                position: parse_position(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let move_cursor = if kind == HyprlandDispatchActionKind::MoveCursor {
        StabbyOption::Some(crate::MoveCursorDispatchMessageStabby {
            x: json.get("x").and_then(|v| v.as_i64()).unwrap_or(0),
            y: json.get("y").and_then(|v| v.as_i64()).unwrap_or(0),
        })
    } else {
        StabbyOption::None()
    };
    let move_cursor_to_corner = if kind == HyprlandDispatchActionKind::MoveCursorToCorner {
        json.get("corner")
            .map(|value| crate::MoveCursorToCornerDispatchMessageStabby { corner: parse_corner(value) })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let move_current_workspace_to_monitor = if kind == HyprlandDispatchActionKind::MoveCurrentWorkspaceToMonitor {
        json.get("monitor_identifier")
            .map(|value| crate::MoveCurrentWorkspaceToMonitorDispatchMessageStabby {
                monitor_identifier: parse_monitor_identifier(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let move_focused_window_to_workspace = if kind == HyprlandDispatchActionKind::MoveFocusedWindowToWorkspace {
        json.get("identifier")
            .map(|value| crate::MoveFocusedWindowToWorkspaceDispatchMessageStabby {
                identifier: parse_workspace_identifier_no_special(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let move_focused_window_to_workspace_silent = if kind == HyprlandDispatchActionKind::MoveFocusedWindowToWorkspaceSilent {
        json.get("identifier")
            .map(|value| crate::MoveFocusedWindowToWorkspaceSilentDispatchMessageStabby {
                identifier: parse_workspace_identifier_no_special(value),
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
    let move_into_group = if kind == HyprlandDispatchActionKind::MoveIntoGroup {
        json.get("direction")
            .map(|value| crate::MoveIntoGroupDispatchMessageStabby {
                direction: parse_direction(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let move_out_of_group: StabbyOption<crate::MoveOutOfGroupDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::MoveOutOfGroup {
        StabbyOption::Some(crate::MoveOutOfGroupDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let move_to_workspace = if kind == HyprlandDispatchActionKind::MoveToWorkspace {
        json.get("identifier")
            .map(|value| crate::MoveToWorkspaceDispatchMessageStabby {
                identifier: parse_workspace_identifier(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let move_to_workspace_silent = if kind == HyprlandDispatchActionKind::MoveToWorkspaceSilent {
        StabbyOption::Some(crate::MoveToWorkspaceSilentDispatchMessageStabby {
            identifier: parse_workspace_identifier(json.get("identifier").unwrap_or(&serde_json::Value::Null)),
            window_identifier: json.get("window_identifier").map(|value| parse_window_identifier(value)).into(),
        })
    } else {
        StabbyOption::None()
    };
    let move_window = if kind == HyprlandDispatchActionKind::MoveWindow {
        json.get("window_move")
            .map(|value| crate::MoveWindowDispatchMessageStabby {
                window_move: parse_window_move(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let move_window_pixel = if kind == HyprlandDispatchActionKind::MoveWindowPixel {
        json.get("position")
            .map(|value| crate::MoveWindowPixelDispatchMessageStabby {
                position: parse_position(value),
                window_identifier: parse_window_identifier(json.get("window_identifier").unwrap_or(&serde_json::Value::Null)),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let orientation_bottom: StabbyOption<crate::OrientationBottomDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::OrientationBottom {
        StabbyOption::Some(crate::OrientationBottomDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let orientation_center: StabbyOption<crate::OrientationCenterDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::OrientationCenter {
        StabbyOption::Some(crate::OrientationCenterDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let orientation_left: StabbyOption<crate::OrientationLeftDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::OrientationLeft {
        StabbyOption::Some(crate::OrientationLeftDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let orientation_next: StabbyOption<crate::OrientationNextDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::OrientationNext {
        StabbyOption::Some(crate::OrientationNextDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let orientation_prev: StabbyOption<crate::OrientationPrevDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::OrientationPrev {
        StabbyOption::Some(crate::OrientationPrevDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let orientation_right: StabbyOption<crate::OrientationRightDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::OrientationRight {
        StabbyOption::Some(crate::OrientationRightDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let orientation_top: StabbyOption<crate::OrientationTopDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::OrientationTop {
        StabbyOption::Some(crate::OrientationTopDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let pass = if kind == HyprlandDispatchActionKind::Pass {
        json.get("window_identifier")
            .map(|value| crate::PassDispatchMessageStabby {
                window_identifier: parse_window_identifier(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let remove_master: StabbyOption<crate::RemoveMasterDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::RemoveMaster {
        StabbyOption::Some(crate::RemoveMasterDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let rename_workspace = if kind == HyprlandDispatchActionKind::RenameWorkspace {
        StabbyOption::Some(crate::RenameWorkspaceDispatchMessageStabby {
            workspace_id: json.get("workspace_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            new_name: json.get("new_name").and_then(|v| v.as_str()).map(stabby::string::String::from).into(),
        })
    } else {
        StabbyOption::None()
    };
    let resize_active = if kind == HyprlandDispatchActionKind::ResizeActive {
        json.get("position")
            .map(|value| crate::ResizeActiveDispatchMessageStabby {
                position: parse_position(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let resize_window_pixel = if kind == HyprlandDispatchActionKind::ResizeWindowPixel {
        json.get("position")
            .map(|value| crate::ResizeWindowPixelDispatchMessageStabby {
                position: parse_position(value),
                window_identifier: parse_window_identifier(json.get("window_identifier").unwrap_or(&serde_json::Value::Null)),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let set_cursor = if kind == HyprlandDispatchActionKind::SetCursor {
        json.get("theme")
            .and_then(|v| v.as_str())
            .map(|theme| crate::SetCursorDispatchMessageStabby {
                theme: stabby::string::String::from(theme),
                size: json.get("size").and_then(|v| v.as_u64()).unwrap_or(0) as u16,
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let swap_active_workspaces = if kind == HyprlandDispatchActionKind::SwapActiveWorkspaces {
        StabbyOption::Some(crate::SwapActiveWorkspacesDispatchMessageStabby {
            monitor_a: parse_monitor_identifier(json.get("monitor_a").unwrap_or(&serde_json::Value::Null)),
            monitor_b: parse_monitor_identifier(json.get("monitor_b").unwrap_or(&serde_json::Value::Null)),
        })
    } else {
        StabbyOption::None()
    };
    let swap_window = if kind == HyprlandDispatchActionKind::SwapWindow {
        json.get("cycle_direction")
            .map(|value| crate::SwapWindowDispatchMessageStabby {
                cycle_direction: parse_cycle_direction(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let swap_with_master = if kind == HyprlandDispatchActionKind::SwapWithMaster {
        json.get("param")
            .map(|value| crate::SwapWithMasterDispatchMessageStabby {
                param: parse_swap_with_master_param(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    let toggle_dpms = if kind == HyprlandDispatchActionKind::ToggleDpms {
        StabbyOption::Some(crate::ToggleDpmsDispatchMessageStabby {
            on: json.get("on").and_then(|v| v.as_bool()).unwrap_or(false),
            name: json.get("name").and_then(|v| v.as_str()).map(stabby::string::String::from).into(),
        })
    } else {
        StabbyOption::None()
    };
    let toggle_fake_fullscreen: StabbyOption<crate::ToggleFakeFullscreenDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::ToggleFakeFullscreen {
        StabbyOption::Some(crate::ToggleFakeFullscreenDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let toggle_floating: StabbyOption<crate::ToggleFloatingDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::ToggleFloating {
        StabbyOption::Some(crate::ToggleFloatingDispatchMessageStabby)
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
    let toggle_group: StabbyOption<crate::ToggleGroupDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::ToggleGroup {
        StabbyOption::Some(crate::ToggleGroupDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let toggle_opaque: StabbyOption<crate::ToggleOpaqueDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::ToggleOpaque {
        StabbyOption::Some(crate::ToggleOpaqueDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let toggle_pin: StabbyOption<crate::TogglePinDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::TogglePin {
        StabbyOption::Some(crate::TogglePinDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let toggle_pseudo: StabbyOption<crate::TogglePseudoDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::TogglePseudo {
        StabbyOption::Some(crate::TogglePseudoDispatchMessageStabby)
    } else {
        StabbyOption::None()
    };
    let toggle_special_workspace = if kind == HyprlandDispatchActionKind::ToggleSpecialWorkspace {
        StabbyOption::Some(crate::ToggleSpecialWorkspaceDispatchMessageStabby {
            workspace_name: parse_stabby_string(json.get("workspace_name").unwrap_or(&serde_json::Value::Null)),
        })
    } else {
        StabbyOption::None()
    };
    let toggle_split: StabbyOption<crate::ToggleSplitDispatchMessageStabby> = if kind == HyprlandDispatchActionKind::ToggleSplit {
        StabbyOption::Some(crate::ToggleSplitDispatchMessageStabby)
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
    let workspace_option = if kind == HyprlandDispatchActionKind::WorkspaceOption {
        json.get("option")
            .map(|value| crate::WorkspaceOptionDispatchMessageStabby {
                option: parse_workspace_options(value),
            })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    } else {
        StabbyOption::None()
    };
    HyprlandDispatchMessage {
        kind,
        add_master,
        bring_active_to_top,
        center_window,
        change_group_active,
        change_split_ratio,
        close_window,
        custom,
        cycle_window,
        exec,
        exit,
        focus_current_or_last,
        focus_master,
        focus_monitor,
        focus_urgent_or_last,
        focus_window,
        force_renderer_reload,
        global,
        kill_active_window,
        lock_groups,
        move_active,
        move_cursor,
        move_cursor_to_corner,
        move_current_workspace_to_monitor,
        move_focused_window_to_workspace,
        move_focused_window_to_workspace_silent,
        move_focus,
        move_into_group,
        move_out_of_group,
        move_to_workspace,
        move_to_workspace_silent,
        move_window,
        move_window_pixel,
        orientation_bottom,
        orientation_center,
        orientation_left,
        orientation_next,
        orientation_prev,
        orientation_right,
        orientation_top,
        pass,
        remove_master,
        rename_workspace,
        resize_active,
        resize_window_pixel,
        set_cursor,
        swap_active_workspaces,
        swap_window,
        swap_with_master,
        toggle_dpms,
        toggle_fake_fullscreen,
        toggle_floating,
        toggle_fullscreen,
        toggle_group,
        toggle_opaque,
        toggle_pin,
        toggle_pseudo,
        toggle_special_workspace,
        toggle_split,
        workspace,
        workspace_option,
    }
});

/// Register all JSON converter implementations for Hyprland messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    HyprlandDispatchMessageConverter::register_in_host(context);
    KillCommandMessageConverter::register_in_host(context);
    ReloadCommandMessageConverter::register_in_host(context);
    NotifyCommandMessageConverter::register_in_host(context);
    OutputCreateCommandMessageConverter::register_in_host(context);
    OutputRemoveCommandMessageConverter::register_in_host(context);
    PluginLoadCommandMessageConverter::register_in_host(context);
    PluginUnloadCommandMessageConverter::register_in_host(context);
    SetCursorCommandMessageConverter::register_in_host(context);
    SetErrorCommandMessageConverter::register_in_host(context);
    SetPropCommandMessageConverter::register_in_host(context);
    SwitchXkbLayoutCommandMessageConverter::register_in_host(context);
}

/// Register all JSON converter implementations for Hyprland messages directly in a registry.
///
/// Call this once during host application startup (e.g. inside `AreaManager::new`).
pub fn register_json_converters_in_registry(registry: &JsonConverterRegistry) {
    HyprlandDispatchMessageConverter::register_json_converter(registry);
    KillCommandMessageConverter::register_json_converter(registry);
    ReloadCommandMessageConverter::register_json_converter(registry);
    NotifyCommandMessageConverter::register_json_converter(registry);
    OutputCreateCommandMessageConverter::register_json_converter(registry);
    OutputRemoveCommandMessageConverter::register_json_converter(registry);
    PluginLoadCommandMessageConverter::register_json_converter(registry);
    PluginUnloadCommandMessageConverter::register_json_converter(registry);
    SetCursorCommandMessageConverter::register_json_converter(registry);
    SetErrorCommandMessageConverter::register_json_converter(registry);
    SetPropCommandMessageConverter::register_json_converter(registry);
    SwitchXkbLayoutCommandMessageConverter::register_json_converter(registry);
}
