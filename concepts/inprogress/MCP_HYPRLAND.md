# MCP Hyprland — Full Message-Broker Parity via MCP Tools, Resources & Prompts

## Motivation

The Hyprland service currently exposes ~80 data structures (dispatches, commands, status events, compositor events) via the internal Message Broker, but only
**3 MCP tools** (`hyprland_switch_workspace`, `hyprland_move_window`, `hyprland_toggle_floating`) and **2 MCP resources** (`hyprland://state`,
`hyprland://active-window`) are registered. This means external MCP clients (AI assistants, automation tools, remote control interfaces) can only access a tiny
fraction of the compositor's capabilities.

This concept defines a plan to achieve **full parity** between the Message Broker and MCP, so every dispatch, command, status event, and compositor event can be
initiated, observed, or queried via MCP tools, resources, and prompts.

No new crates are needed — all changes are in the existing `model/hyprland`, `services/hyprland`, and `mcp-server` crates.

### Critical Design Rules

1. **No manual JSON serialization or deserialization**: All MCP tool arguments and resource responses must use Rust structs with
   `#[derive(Serialize, Deserialize)]` (and `JsonSchema` for tool args). Manual `serde_json::json!()` construction or hand-written JSON strings are
   **forbidden**. Use `serde_json::to_string(&struct)` for serialization and `serde_json::from_str::<ArgsStruct>(&json)` for deserialization. Existing
   dispatch/command structs should be reused wherever possible instead of creating duplicate types.
2. **Prompts are outsourced**: Prompt templates are `.md` files in `services/hyprland/data/prompts/` and loaded via `include_str!()` at compile time. Use
   `smearor_model_mcp::render_template()` for placeholder substitution (e.g. `{{workspace_count}}`). See `services/weather/src/mcp/handler/prompt.rs` and
   `services/weather/data/prompts/` as reference implementation.
3. **`McpCapabilitiesRegistrator` must be implemented**: The `HyprlandService` must implement (or extend the existing) `McpCapabilitiesRegistrator` trait from
   `smearor_swipe_launcher_plugin_api`. The `register_mcp_capabilities()` method is the single source of truth for all MCP tool, resource, and prompt
   registrations. Every new tool, resource, and prompt must be registered there via `RegisterToolMessage`, `RegisterResourceMessage`, and
   `RegisterPromptMessage`.
4. **LLM-friendly documentation**: Every `RegisterToolMessage`, `RegisterResourceMessage`, and `RegisterPromptMessage` must carry a descriptive `name`, a
   detailed `description`, and (for prompts) a `memory_query` string. The description must be clear enough for an LLM to understand when and how to use the
   tool/resource/prompt without additional context. For prompts that benefit from memory recall, use
   `RegisterPromptMessage::with_memory(name, description, schema, memory_query, entity_filter)`.

---

## Crate Structure

| Crate Type     | Path                 | Responsibility                                                                       |
|----------------|----------------------|--------------------------------------------------------------------------------------|
| **Model**      | `model/hyprland/`    | MCP tool/resource/prompt enums, args structs, response structs                       |
| **Service**    | `services/hyprland/` | MCP tool handlers, resource handlers, prompt registration, capabilities registration |
| **MCP Server** | `mcp-server/`        | Prompt definitions for Hyprland compositor control                                   |

---

## Current State — Full Inventory

### 1. Window Dispatches

| Datenstruktur                       | Topic                              | Broker | MCP |
|-------------------------------------|------------------------------------|--------|-----|
| `CenterWindowDispatchMessage`       | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `ChangeGroupActiveDispatchMessage`  | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `ChangeSplitRatioDispatchMessage`   | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `CloseWindowDispatchMessage`        | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `CycleWindowDispatchMessage`        | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `ExecDispatchMessage`               | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `FocusCurrentOrLastDispatchMessage` | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `FocusMasterDispatchMessage`        | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `FocusMonitorDispatchMessage`       | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `FocusUrgentOrLastDispatchMessage`  | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `FocusWindowDispatchMessage`        | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `KillActiveWindowDispatchMessage`   | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `MoveActiveDispatchMessage`         | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `MoveCursorDispatchMessage`         | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `MoveCursorToCornerDispatchMessage` | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `MoveFocusDispatchMessage`          | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `MoveIntoGroupDispatchMessage`      | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `MoveWindowDispatchMessage`         | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `MoveWindowPixelDispatchMessage`    | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `ResizeActiveDispatchMessage`       | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `ResizeWindowPixelDispatchMessage`  | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `SwapWindowDispatchMessage`         | `service.hyprland.dispatch.window` | ✅     | ❌  |
| `SwapWithMasterDispatchMessage`     | `service.hyprland.dispatch.window` | ✅     | ❌  |

### 2. Workspace Dispatches

| Datenstruktur                                       | Topic                                 | Broker | MCP                              |
|-----------------------------------------------------|---------------------------------------|--------|----------------------------------|
| `WorkspaceDispatchMessage`                          | `service.hyprland.dispatch.workspace` | ✅     | ✅ (`hyprland_switch_workspace`) |
| `MoveToWorkspaceDispatchMessage`                    | `service.hyprland.dispatch.workspace` | ✅     | ✅ (`hyprland_move_window`)      |
| `MoveToWorkspaceSilentDispatchMessage`              | `service.hyprland.dispatch.workspace` | ✅     | ❌                               |
| `MoveFocusedWindowToWorkspaceDispatchMessage`       | `service.hyprland.dispatch.workspace` | ✅     | ❌                               |
| `MoveFocusedWindowToWorkspaceSilentDispatchMessage` | `service.hyprland.dispatch.workspace` | ✅     | ❌                               |
| `MoveCurrentWorkspaceToMonitorDispatchMessage`      | `service.hyprland.dispatch.workspace` | ✅     | ❌                               |
| `RenameWorkspaceDispatchMessage`                    | `service.hyprland.dispatch.workspace` | ✅     | ❌                               |
| `SwapActiveWorkspacesDispatchMessage`               | `service.hyprland.dispatch.workspace` | ✅     | ❌                               |
| `ToggleSpecialWorkspaceDispatchMessage`             | `service.hyprland.dispatch.workspace` | ✅     | ❌                               |
| `WorkspaceOptionDispatchMessage`                    | `service.hyprland.dispatch.workspace` | ✅     | ❌                               |

### 3. Toggle Dispatches

| Datenstruktur                         | Topic                              | Broker | MCP                             |
|---------------------------------------|------------------------------------|--------|---------------------------------|
| `ToggleFloatingDispatchMessage`       | `service.hyprland.dispatch.toggle` | ✅     | ✅ (`hyprland_toggle_floating`) |
| `ToggleFullscreenDispatchMessage`     | `service.hyprland.dispatch.toggle` | ✅     | ❌                              |
| `ToggleDpmsDispatchMessage`           | `service.hyprland.dispatch.toggle` | ✅     | ❌                              |
| `ToggleFakeFullscreenDispatchMessage` | `service.hyprland.dispatch.toggle` | ✅     | ❌                              |
| `ToggleGroupDispatchMessage`          | `service.hyprland.dispatch.toggle` | ✅     | ❌                              |
| `ToggleOpaqueDispatchMessage`         | `service.hyprland.dispatch.toggle` | ✅     | ❌                              |
| `TogglePinDispatchMessage`            | `service.hyprland.dispatch.toggle` | ✅     | ❌                              |
| `TogglePseudoDispatchMessage`         | `service.hyprland.dispatch.toggle` | ✅     | ❌                              |
| `ToggleSplitDispatchMessage`          | `service.hyprland.dispatch.toggle` | ✅     | ❌                              |

### 4. System Dispatches

| Datenstruktur                        | Topic                              | Broker | MCP |
|--------------------------------------|------------------------------------|--------|-----|
| `AddMasterDispatchMessage`           | `service.hyprland.dispatch.system` | ✅     | ❌  |
| `BringActiveToTopDispatchMessage`    | `service.hyprland.dispatch.system` | ✅     | ❌  |
| `CustomDispatchMessage`              | `service.hyprland.dispatch.system` | ✅     | ❌  |
| `ExitDispatchMessage`                | `service.hyprland.dispatch.system` | ✅     | ❌  |
| `ForceRendererReloadDispatchMessage` | `service.hyprland.dispatch.system` | ✅     | ❌  |
| `GlobalDispatchMessage`              | `service.hyprland.dispatch.system` | ✅     | ❌  |
| `LockGroupsDispatchMessage`          | `service.hyprland.dispatch.system` | ✅     | ❌  |
| `MoveOutOfGroupDispatchMessage`      | `service.hyprland.dispatch.system` | ✅     | ❌  |
| `OrientationBottomDispatchMessage`   | `service.hyprland.dispatch.system` | ✅     | ❌  |
| `OrientationCenterDispatchMessage`   | `service.hyprland.dispatch.system` | ✅     | ❌  |
| `OrientationLeftDispatchMessage`     | `service.hyprland.dispatch.system` | ✅     | ❌  |
| `OrientationNextDispatchMessage`     | `service.hyprland.dispatch.system` | ✅     | ❌  |
| `OrientationPrevDispatchMessage`     | `service.hyprland.dispatch.system` | ✅     | ❌  |
| `OrientationRightDispatchMessage`    | `service.hyprland.dispatch.system` | ✅     | ❌  |
| `OrientationTopDispatchMessage`      | `service.hyprland.dispatch.system` | ✅     | ❌  |
| `PassDispatchMessage`                | `service.hyprland.dispatch.system` | ✅     | ❌  |
| `RemoveMasterDispatchMessage`        | `service.hyprland.dispatch.system` | ✅     | ❌  |
| `SetCursorDispatchMessage`           | `service.hyprland.dispatch.system` | ✅     | ❌  |

### 5. Control Commands (CTL)

| Datenstruktur                   | Topic                  | Broker | MCP |
|---------------------------------|------------------------|--------|-----|
| `KillCommandMessage`            | `service.hyprland.ctl` | ✅     | ❌  |
| `NotifyCommandMessage`          | `service.hyprland.ctl` | ✅     | ❌  |
| `OutputCreateCommandMessage`    | `service.hyprland.ctl` | ✅     | ❌  |
| `OutputRemoveCommandMessage`    | `service.hyprland.ctl` | ✅     | ❌  |
| `PluginLoadCommandMessage`      | `service.hyprland.ctl` | ✅     | ❌  |
| `PluginUnloadCommandMessage`    | `service.hyprland.ctl` | ✅     | ❌  |
| `ReloadCommandMessage`          | `service.hyprland.ctl` | ✅     | ❌  |
| `SetCursorCommandMessage`       | `service.hyprland.ctl` | ✅     | ❌  |
| `SetErrorCommandMessage`        | `service.hyprland.ctl` | ✅     | ❌  |
| `SetPropCommandMessage`         | `service.hyprland.ctl` | ✅     | ❌  |
| `SwitchXkbLayoutCommandMessage` | `service.hyprland.ctl` | ✅     | ❌  |

### 6. Compositor Workspace Commands

| Datenstruktur                     | Topic                                   | Broker | MCP |
|-----------------------------------|-----------------------------------------|--------|-----|
| `SwitchWorkspaceMessage`          | `compositor.workspace.switch`           | ✅     | ❌  |
| `CreateWorkspaceMessage`          | `compositor.workspace.create`           | ✅     | ❌  |
| `WorkspaceSnapshotRequestMessage` | `compositor.workspace.snapshot.request` | ✅     | ❌  |

### 7. Status Events (Service → Widget, read-only broadcast)

#### 7.1 Window Status (`service.hyprland.window.status`)

| Datenstruktur                      | Topic                            | Broker         | MCP |
|------------------------------------|----------------------------------|----------------|-----|
| `ActiveWindowChangedStatusMessage` | `service.hyprland.window.status` | ✅ (broadcast) | ❌  |
| `WindowOpenedStatusMessage`        | `service.hyprland.window.status` | ✅ (broadcast) | ❌  |
| `WindowClosedStatusMessage`        | `service.hyprland.window.status` | ✅ (broadcast) | ❌  |
| `WindowMovedStatusMessage`         | `service.hyprland.window.status` | ✅ (broadcast) | ❌  |
| `FloatStateChangedStatusMessage`   | `service.hyprland.window.status` | ✅ (broadcast) | ❌  |
| `UrgentStateChangedStatusMessage`  | `service.hyprland.window.status` | ✅ (broadcast) | ❌  |
| `WindowTitleChangedStatusMessage`  | `service.hyprland.window.status` | ✅ (broadcast) | ❌  |
| `WindowPinnedStatusMessage`        | `service.hyprland.window.status` | ✅ (broadcast) | ❌  |

#### 7.2 Workspace Status (`service.hyprland.workspace.status`)

| Datenstruktur                         | Topic                               | Broker         | MCP |
|---------------------------------------|-------------------------------------|----------------|-----|
| `FullscreenStateChangedStatusMessage` | `service.hyprland.workspace.status` | ✅ (broadcast) | ❌  |
| `WorkspaceRenamedStatusMessage`       | `service.hyprland.workspace.status` | ✅ (broadcast) | ❌  |
| `SpecialRemovedStatusMessage`         | `service.hyprland.workspace.status` | ✅ (broadcast) | ❌  |
| `ChangedSpecialStatusMessage`         | `service.hyprland.workspace.status` | ✅ (broadcast) | ❌  |
| `SubMapChangedStatusMessage`          | `service.hyprland.workspace.status` | ✅ (broadcast) | ❌  |

#### 7.3 Group Status (`service.hyprland.group.status`)

| Datenstruktur                              | Topic                           | Broker         | MCP |
|--------------------------------------------|---------------------------------|----------------|-----|
| `GroupToggledStatusMessage`                | `service.hyprland.group.status` | ✅ (broadcast) | ❌  |
| `WindowMovedIntoGroupStatusMessage`        | `service.hyprland.group.status` | ✅ (broadcast) | ❌  |
| `WindowMovedOutOfGroupStatusMessage`       | `service.hyprland.group.status` | ✅ (broadcast) | ❌  |
| `IgnoreGroupLockStateChangedStatusMessage` | `service.hyprland.group.status` | ✅ (broadcast) | ❌  |
| `LockGroupsStateChangedStatusMessage`      | `service.hyprland.group.status` | ✅ (broadcast) | ❌  |

#### 7.4 Layer Status (`service.hyprland.layer.status`)

| Datenstruktur              | Topic                           | Broker         | MCP |
|----------------------------|---------------------------------|----------------|-----|
| `LayerOpenedStatusMessage` | `service.hyprland.layer.status` | ✅ (broadcast) | ❌  |
| `LayerClosedStatusMessage` | `service.hyprland.layer.status` | ✅ (broadcast) | ❌  |

#### 7.5 System Status (`service.hyprland.system.status`)

| Datenstruktur                        | Topic                            | Broker         | MCP |
|--------------------------------------|----------------------------------|----------------|-----|
| `KeyboardLayoutChangedStatusMessage` | `service.hyprland.system.status` | ✅ (broadcast) | ❌  |
| `ScreencastStatusMessage`            | `service.hyprland.system.status` | ✅ (broadcast) | ❌  |
| `ConfigReloadedStatusMessage`        | `service.hyprland.system.status` | ✅ (broadcast) | ❌  |

### 8. State Request / Response

| Datenstruktur                     | Topic                              | Broker         | MCP                              |
|-----------------------------------|------------------------------------|----------------|----------------------------------|
| `HyprlandStateRequestMessage`     | `service.hyprland.status.request`  | ✅             | ❌                               |
| `HyprlandStateMessage` (response) | `service.hyprland.status.response` | ✅ (broadcast) | ✅ (`hyprland://state` resource) |

### 9. Compositor Events (Service → Widget, broadcast)

| Datenstruktur              | Topic                            | Broker         | MCP |
|----------------------------|----------------------------------|----------------|-----|
| `WorkspaceChangedEvent`    | `compositor.workspace.changed`   | ✅ (broadcast) | ❌  |
| `WorkspaceLifecycleEvent`  | `compositor.workspace.lifecycle` | ✅ (broadcast) | ❌  |
| `WorkspaceSnapshotMessage` | `compositor.workspace.snapshot`  | ✅ (broadcast) | ❌  |
| `MonitorChangedEvent`      | `compositor.monitor.changed`     | ✅ (broadcast) | ❌  |

### 10. MCP Tools & Resources (current)

| MCP Tool / Resource                   | Function                 | Corresponding Broker Topic            |
|---------------------------------------|--------------------------|---------------------------------------|
| `hyprland_switch_workspace`           | Switch workspace         | `service.hyprland.dispatch.workspace` |
| `hyprland_move_window`                | Move window to workspace | `service.hyprland.dispatch.workspace` |
| `hyprland_toggle_floating`            | Toggle floating          | `service.hyprland.dispatch.toggle`    |
| `hyprland://state` (resource)         | Query Hyprland state     | `service.hyprland.status.response`    |
| `hyprland://active-window` (resource) | Query active window      | (derived from state)                  |

---

## Target State — MCP Parity Plan

### Design Principles

1. **MCP Tools** map to **initiating actions** (dispatches, commands) — one tool per dispatch/command kind
2. **MCP Resources** map to **queryable state** (state, snapshots, active window) — read-only
3. **MCP Prompts** map to **guided workflows** (multi-step compositor operations) — contextual templates
4. **Status events** are broadcast-only and do not get individual MCP tools — they are observable via resources that return the latest state
5. Tool naming convention: `hyprland_<category>_<action>` (e.g. `hyprland_window_kill_active`, `hyprland_toggle_fullscreen`)
6. Resource naming convention: `hyprland://<resource>` (e.g. `hyprland://workspace-snapshot`)
7. Prompt naming convention: `hyprland_<workflow>` (e.g. `hyprland_workspace_management`)

### MCP Tools — New Tools to Add

#### Window Dispatch Tools

| MCP Tool Name                           | Dispatch Kind        | Args Struct             | Description                             |
|-----------------------------------------|----------------------|-------------------------|-----------------------------------------|
| `hyprland_window_center`                | `CenterWindow`       | `WindowIdentifierArgs`  | Center the active window                |
| `hyprland_window_change_group_active`   | `ChangeGroupActive`  | `ChangeGroupActiveArgs` | Change active window in group           |
| `hyprland_window_change_split_ratio`    | `ChangeSplitRatio`   | `ChangeSplitRatioArgs`  | Change split ratio                      |
| `hyprland_window_close`                 | `CloseWindow`        | `WindowIdentifierArgs`  | Close a window                          |
| `hyprland_window_cycle`                 | `CycleWindow`        | `CycleWindowArgs`       | Cycle to next/previous window           |
| `hyprland_window_exec`                  | `Exec`               | `ExecArgs`              | Execute a command via Hyprland dispatch |
| `hyprland_window_focus_current_or_last` | `FocusCurrentOrLast` | (none)                  | Focus current or last window            |
| `hyprland_window_focus_master`          | `FocusMaster`        | `FocusMasterArgs`       | Focus master window                     |
| `hyprland_window_focus_monitor`         | `FocusMonitor`       | `MonitorIdentifierArgs` | Focus a specific monitor                |
| `hyprland_window_focus_urgent_or_last`  | `FocusUrgentOrLast`  | (none)                  | Focus urgent or last window             |
| `hyprland_window_focus_window`          | `FocusWindow`        | `WindowIdentifierArgs`  | Focus a specific window                 |
| `hyprland_window_kill_active`           | `KillActiveWindow`   | (none)                  | Kill the active window                  |
| `hyprland_window_move_active`           | `MoveActive`         | `MoveActiveArgs`        | Move active window                      |
| `hyprland_window_move_cursor`           | `MoveCursor`         | `MoveCursorArgs`        | Move cursor to position                 |
| `hyprland_window_move_cursor_to_corner` | `MoveCursorToCorner` | `CornerArgs`            | Move cursor to corner                   |
| `hyprland_window_move_focus`            | `MoveFocus`          | `DirectionArgs`         | Move focus in direction                 |
| `hyprland_window_move_into_group`       | `MoveIntoGroup`      | (none)                  | Move active window into group           |
| `hyprland_window_move_window`           | `MoveWindow`         | `MoveWindowArgs`        | Move window                             |
| `hyprland_window_move_window_pixel`     | `MoveWindowPixel`    | `MoveWindowPixelArgs`   | Move window by pixel delta              |
| `hyprland_window_resize_active`         | `ResizeActive`       | `ResizeActiveArgs`      | Resize active window                    |
| `hyprland_window_resize_window_pixel`   | `ResizeWindowPixel`  | `ResizeWindowPixelArgs` | Resize window by pixel delta            |
| `hyprland_window_swap`                  | `SwapWindow`         | `DirectionArgs`         | Swap window in direction                |
| `hyprland_window_swap_with_master`      | `SwapWithMaster`     | `SwapWithMasterArgs`    | Swap active window with master          |

#### Workspace Dispatch Tools

| MCP Tool Name                                   | Dispatch Kind                        | Args Struct                | Description                             |
|-------------------------------------------------|--------------------------------------|----------------------------|-----------------------------------------|
| `hyprland_workspace_move_current_to_monitor`    | `MoveCurrentWorkspaceToMonitor`      | `MonitorIdentifierArgs`    | Move current workspace to monitor       |
| `hyprland_workspace_move_focused_window`        | `MoveFocusedWindowToWorkspace`       | `WorkspaceIdentifierArgs`  | Move focused window to workspace        |
| `hyprland_workspace_move_focused_window_silent` | `MoveFocusedWindowToWorkspaceSilent` | `WorkspaceIdentifierArgs`  | Move focused window silently            |
| `hyprland_workspace_move_to_workspace_silent`   | `MoveToWorkspaceSilent`              | `WorkspaceIdentifierArgs`  | Move active window silently             |
| `hyprland_workspace_rename`                     | `RenameWorkspace`                    | `RenameWorkspaceArgs`      | Rename a workspace                      |
| `hyprland_workspace_swap_active`                | `SwapActiveWorkspaces`               | `SwapActiveWorkspacesArgs` | Swap active workspaces between monitors |
| `hyprland_workspace_toggle_special`             | `ToggleSpecialWorkspace`             | `SpecialWorkspaceArgs`     | Toggle special workspace                |
| `hyprland_workspace_option`                     | `WorkspaceOption`                    | `WorkspaceOptionArgs`      | Set workspace option                    |

#### Toggle Dispatch Tools

| MCP Tool Name                     | Dispatch Kind          | Args Struct          | Description                          |
|-----------------------------------|------------------------|----------------------|--------------------------------------|
| `hyprland_toggle_fullscreen`      | `ToggleFullscreen`     | `FullscreenTypeArgs` | Toggle fullscreen (maximize or real) |
| `hyprland_toggle_dpms`            | `ToggleDpms`           | `ToggleDpmsArgs`     | Toggle DPMS (monitor power)          |
| `hyprland_toggle_fake_fullscreen` | `ToggleFakeFullscreen` | (none)               | Toggle fake fullscreen               |
| `hyprland_toggle_group`           | `ToggleGroup`          | (none)               | Toggle window group                  |
| `hyprland_toggle_opaque`          | `ToggleOpaque`         | (none)               | Toggle opaque                        |
| `hyprland_toggle_pin`             | `TogglePin`            | (none)               | Toggle pin                           |
| `hyprland_toggle_pseudo`          | `TogglePseudo`         | (none)               | Toggle pseudo tiling                 |
| `hyprland_toggle_split`           | `ToggleSplit`          | (none)               | Toggle split                         |

#### System Dispatch Tools

| MCP Tool Name                           | Dispatch Kind         | Args Struct          | Description                      |
|-----------------------------------------|-----------------------|----------------------|----------------------------------|
| `hyprland_system_add_master`            | `AddMaster`           | (none)               | Add master to layout             |
| `hyprland_system_bring_active_to_top`   | `BringActiveToTop`    | (none)               | Bring active window to top       |
| `hyprland_system_custom`                | `Custom`              | `CustomDispatchArgs` | Execute custom Hyprland dispatch |
| `hyprland_system_exit`                  | `Exit`                | (none)               | Exit Hyprland                    |
| `hyprland_system_force_renderer_reload` | `ForceRendererReload` | (none)               | Force renderer reload            |
| `hyprland_system_global`                | `Global`              | `GlobalDispatchArgs` | Execute global keybinding        |
| `hyprland_system_lock_groups`           | `LockGroups`          | `LockGroupsArgs`     | Lock/unlock/toggle group locks   |
| `hyprland_system_move_out_of_group`     | `MoveOutOfGroup`      | (none)               | Move active window out of group  |
| `hyprland_system_orientation`           | `Orientation*`        | `OrientationArgs`    | Set window orientation           |
| `hyprland_system_pass`                  | `Pass`                | `PassArgs`           | Pass key event                   |
| `hyprland_system_remove_master`         | `RemoveMaster`        | (none)               | Remove master from layout        |
| `hyprland_system_set_cursor`            | `SetCursor`           | `SetCursorArgs`      | Set cursor shape                 |

#### Control Command Tools

| MCP Tool Name                    | Command                         | Args Struct           | Description                |
|----------------------------------|---------------------------------|-----------------------|----------------------------|
| `hyprland_ctl_kill`              | `KillCommandMessage`            | (none)                | Enter kill mode            |
| `hyprland_ctl_notify`            | `NotifyCommandMessage`          | `NotifyArgs`          | Send Hyprland notification |
| `hyprland_ctl_output_create`     | `OutputCreateCommandMessage`    | `OutputCreateArgs`    | Create virtual output      |
| `hyprland_ctl_output_remove`     | `OutputRemoveCommandMessage`    | `OutputRemoveArgs`    | Remove virtual output      |
| `hyprland_ctl_plugin_load`       | `PluginLoadCommandMessage`      | `PluginLoadArgs`      | Load Hyprland plugin       |
| `hyprland_ctl_plugin_unload`     | `PluginUnloadCommandMessage`    | `PluginUnloadArgs`    | Unload Hyprland plugin     |
| `hyprland_ctl_reload`            | `ReloadCommandMessage`          | (none)                | Reload Hyprland config     |
| `hyprland_ctl_set_cursor`        | `SetCursorCommandMessage`       | `SetCursorCtlArgs`    | Set cursor (ctl variant)   |
| `hyprland_ctl_set_error`         | `SetErrorCommandMessage`        | `SetErrorArgs`        | Set error status           |
| `hyprland_ctl_set_prop`          | `SetPropCommandMessage`         | `SetPropArgs`         | Set window property        |
| `hyprland_ctl_switch_xkb_layout` | `SwitchXkbLayoutCommandMessage` | `SwitchXkbLayoutArgs` | Switch keyboard layout     |

#### Compositor Workspace Tools

| MCP Tool Name                          | Message                  | Args Struct                     | Description                         |
|----------------------------------------|--------------------------|---------------------------------|-------------------------------------|
| `hyprland_compositor_create_workspace` | `CreateWorkspaceMessage` | `CreateWorkspaceArgs`           | Create a new workspace              |
| `hyprland_compositor_switch_workspace` | `SwitchWorkspaceMessage` | `SwitchWorkspaceCompositorArgs` | Switch workspace (compositor-level) |

### MCP Resources — New Resources to Add

| MCP Resource URI                 | Source                     | Description                                       |
|----------------------------------|----------------------------|---------------------------------------------------|
| `hyprland://workspace-snapshot`  | `WorkspaceSnapshotMessage` | Full snapshot of all workspaces and their windows |
| `hyprland://workspace-changed`   | `WorkspaceChangedEvent`    | Latest workspace change event                     |
| `hyprland://workspace-lifecycle` | `WorkspaceLifecycleEvent`  | Latest workspace lifecycle event                  |
| `hyprland://monitor-changed`     | `MonitorChangedEvent`      | Latest monitor change event                       |
| `hyprland://window-status`       | `WindowEvent` (latest)     | Latest window status event                        |
| `hyprland://workspace-status`    | `WorkspaceEvent` (latest)  | Latest workspace status event                     |
| `hyprland://group-status`        | `GroupEvent` (latest)      | Latest group status event                         |
| `hyprland://layer-status`        | `LayerEvent` (latest)      | Latest layer status event                         |
| `hyprland://system-status`       | `SystemEvent` (latest)     | Latest system status event                        |

### MCP Prompts — New Prompts to Add

Prompts are outsourced to template files in `services/hyprland/data/prompts/` and loaded via `include_str!()`. Placeholder substitution uses
`smearor_model_mcp::render_template()`. See `services/weather/src/mcp/handler/prompt.rs` as reference.

| MCP Prompt Name                 | Description                                                                             | Memory Query                               | Entity Filter              | Template File                                            |
|---------------------------------|-----------------------------------------------------------------------------------------|--------------------------------------------|----------------------------|----------------------------------------------------------|
| `hyprland_workspace_management` | Guide for workspace operations (create, switch, move windows, rename)                   | `Hyprland workspace layout preference`     | `workspace,monitor`        | `services/hyprland/data/prompts/workspace_management.md` |
| `hyprland_window_management`    | Guide for window operations (focus, move, resize, close, toggle floating/fullscreen)    | `Hyprland window layout preference`        | `window,workspace`         | `services/hyprland/data/prompts/window_management.md`    |
| `hyprland_monitor_management`   | Guide for monitor operations (focus, create/remove virtual outputs, move workspaces)    | `Hyprland monitor setup preference`        | `monitor,workspace`        | `services/hyprland/data/prompts/monitor_management.md`   |
| `hyprland_group_management`     | Guide for window group operations (toggle, move into/out, lock)                         | `Hyprland window group preference`         | `window,group`             | `services/hyprland/data/prompts/group_management.md`     |
| `hyprland_system_control`       | Guide for system-level operations (exit, reload, plugins, cursor, orientation)          | `Hyprland system configuration preference` | `system,plugin`            | `services/hyprland/data/prompts/system_control.md`       |
| `hyprland_status_overview`      | Guide for querying compositor state (active window, workspace snapshot, monitor status) | `Hyprland compositor state`                | `workspace,window,monitor` | `services/hyprland/data/prompts/status_overview.md`      |

All prompts use `RegisterPromptMessage::with_memory()` to enable SemanticMemory recall and EntityStore filtering.

### MCP Guides — Overview Prompts

In addition to the workflow-specific prompts above, one or more **overview guides** provide a comprehensive summary of all available Hyprland MCP capabilities.
These guides help an LLM quickly understand the full scope of available tools, resources, and prompts.

| MCP Prompt Name            | Description                                                                                             | Memory Query                                             | Entity Filter                           | Template File                                       |
|----------------------------|---------------------------------------------------------------------------------------------------------|----------------------------------------------------------|-----------------------------------------|-----------------------------------------------------|
| `hyprland_overview`        | Comprehensive overview of all Hyprland MCP tools, resources, and prompts with usage examples            | `Hyprland compositor configuration and usage preference` | `workspace,window,monitor,group,system` | `services/hyprland/data/prompts/overview.md`        |
| `hyprland_quick_reference` | Quick reference card listing all tool names, resource URIs, and prompt names with one-line descriptions | `Hyprland tool usage preference`                         | ``                                      | `services/hyprland/data/prompts/quick_reference.md` |

The overview guide template (`overview.md`) should contain:

```markdown
# Hyprland MCP Capabilities Overview

You have access to the following Hyprland compositor controls via MCP:

## Tools ({{tool_count}})

### Window Management
{{window_tools_list}}

### Workspace Management
{{workspace_tools_list}}

### Toggle States
{{toggle_tools_list}}

### System
{{system_tools_list}}

### Control Commands
{{ctl_tools_list}}

## Resources ({{resource_count}})
{{resources_list}}

## Prompts ({{prompt_count}})
{{prompts_list}}

## Quick Start
1. Query `hyprland://state` to get current compositor state
2. Query `hyprland://workspace-snapshot` to see all workspaces
3. Use the appropriate tool to perform the requested action
4. Query state again to confirm the result
```

The quick reference template (`quick_reference.md`) should contain a compact table of all tool names and one-line descriptions, suitable for injection into an
LLM context window.

---

## Implementation Plan

### Phase 1: Model Crate — Args Structs & Tools/Resources/Prompts Enums

**Dependencies**: None (extends existing `model/hyprland`)

**Tasks**:

1. Create `model/hyprland/src/mcp/tools.rs` — extend `HyprlandMcpTools` enum with all new tool variants
2. Create `model/hyprland/src/mcp/requests.rs` — add all new args structs with `JsonSchema` derive
3. Create `model/hyprland/src/mcp/resources.rs` — extend `HyprlandMcpResources` enum with all new resource variants
4. Create `model/hyprland/src/mcp/prompts.rs` — new `HyprlandMcpPrompts` enum with `AsRef<str>`, `FromStr`, `Display`
5. Create `model/hyprland/src/mcp/responses.rs` — add response structs for workspace snapshot, monitor changed, etc.
6. Export all new types from `model/hyprland/src/lib.rs`

**Args struct organization** — group by category in separate files under `model/hyprland/src/mcp/args/`:

- `args/window.rs` — window dispatch args (identifier, cycle, move, resize, swap, etc.)
- `args/workspace.rs` — workspace dispatch args (identifier, rename, swap, special, option)
- `args/toggle.rs` — toggle dispatch args (fullscreen type, dpms)
- `args/system.rs` — system dispatch args (custom, global, lock, orientation, pass, cursor)
- `args/ctl.rs` — control command args (notify, output, plugin, prop, xkb)
- `args/compositor.rs` — compositor workspace args (create, switch)

Each args struct derives `Clone, Debug, Default, Serialize, Deserialize, JsonSchema`.

**Exit Criteria**: `cargo build -p smearor_hyprland_model` succeeds. All enums and args structs compile.

### Phase 2: Service Crate — MCP Tool Handlers

**Dependencies**: Phase 1

**Tasks**:

1. Extend `services/hyprland/src/mcp/handler/tools.rs` — add `match` arms for all new `HyprlandMcpTools` variants
2. Each tool handler:
    - Parse args from `message.0.arguments` via `serde_json::from_str` with explicit error handling
    - Construct the appropriate dispatch/command message
    - Send via `self.command_sender.send(HyprlandCommand::...)`
    - Return `InvokeToolResponse::success` or `InvokeToolResponse::error`
3. Split tool handler into category modules to keep file sizes manageable:
    - `services/hyprland/src/mcp/handler/tools/window.rs`
    - `services/hyprland/src/mcp/handler/tools/workspace.rs`
    - `services/hyprland/src/mcp/handler/tools/toggle.rs`
    - `services/hyprland/src/mcp/handler/tools/system.rs`
    - `services/hyprland/src/mcp/handler/tools/ctl.rs`
    - `services/hyprland/src/mcp/handler/tools/compositor.rs`
    - `services/hyprland/src/mcp/handler/tools.rs` — dispatches to sub-modules
4. Extend `services/hyprland/src/mcp/capabilities.rs` — register all new tools via `RegisterToolMessage`
5. Use `schemars::schema_for!` for JSON schema generation of each args struct

**Exit Criteria**: `cargo build -p smearor_hyprland_service` succeeds. All tools registered and handler compiles.

### Phase 3: Service Crate — MCP Resource Handlers

**Dependencies**: Phase 1

**Tasks**:

1. Extend `services/hyprland/src/mcp/handler/resources.rs` — add `match` arms for all new `HyprlandMcpResources` variants
2. Cache latest status events in `HyprlandService` shared state for resource queries:
    - `latest_window_event: Arc<Mutex<Option<WindowEvent>>>`
    - `latest_workspace_event: Arc<Mutex<Option<WorkspaceEvent>>>`
    - `latest_group_event: Arc<Mutex<Option<GroupEvent>>>`
    - `latest_layer_event: Arc<Mutex<Option<LayerEvent>>>`
    - `latest_system_event: Arc<Mutex<Option<SystemEvent>>>`
    - `latest_workspace_changed: Arc<Mutex<Option<WorkspaceChangedEvent>>>`
    - `latest_workspace_lifecycle: Arc<Mutex<Option<WorkspaceLifecycleEvent>>>`
    - `latest_monitor_changed: Arc<Mutex<Option<MonitorChangedEvent>>>`
3. Update event listener to store latest events in shared state (brief mutex hold, no I/O under lock)
4. Implement `WorkspaceSnapshotRequestMessage` handler that returns full snapshot as MCP resource
5. Extend `services/hyprland/src/mcp/capabilities.rs` — register all new resources via `RegisterResourceMessage`

**Exit Criteria**: All resources registered. Resource queries return current state.

### Phase 4: Service Crate — Prompt Templates & Handler

**Dependencies**: Phase 1

**Tasks**:

1. Create prompt template files in `services/hyprland/data/prompts/`:
    - `workspace_management.md`
    - `window_management.md`
    - `monitor_management.md`
    - `group_management.md`
    - `system_control.md`
    - `status_overview.md`
    - `overview.md` — comprehensive guide with all tools/resources/prompts
    - `quick_reference.md` — compact one-line-per-tool reference card
2. Each prompt template file contains:
    - System context about Hyprland compositor
    - Available MCP tools for the workflow (with descriptions)
    - Step-by-step guidance for common operations
    - Example tool invocations
    - `{{placeholder}}` variables for runtime substitution (e.g. `{{workspace_count}}`, `{{active_window_class}}`)
3. Create `services/hyprland/src/mcp/handler/prompt.rs` — implement `MessageHandler<FfiEnvelopePayload<InvokePromptMessage>>`:
    - Parse prompt name via `HyprlandMcpPrompts::from_str()`
    - Load template via `include_str!("../../../data/prompts/<name>.md")`
    - Render with `smearor_model_mcp::render_template()` using runtime values from service state
    - Return `InvokePromptResponse::success()` with `PromptMessage::new("system", &content)`
    - Use `debug!` for logging prompt invocations
4. Reference implementation: `services/weather/src/mcp/handler/prompt.rs`

**Prompt handler pattern** (following weather service):

```rust
impl MessageHandler<FfiEnvelopePayload<InvokePromptMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokePromptMessage>, sender_id: &str) {
        let prompt_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        debug!("hyprland: InvokePromptMessage name={} sender_id={}", prompt_name, sender_id);
        let prompt = match HyprlandMcpPrompts::from_str(&prompt_name) {
            Ok(prompt) => prompt,
            Err(e) => {
                self.send_response(InvokePromptResponse::from(InvokePromptError::new(e, &correlation_id)), sender_id);
                return;
            }
        };
        let response = match prompt {
            HyprlandMcpPrompts::WorkspaceManagement => {
                let content = render_template(
                    include_str!("../../../data/prompts/workspace_management.md"),
                    &[("workspace_count", &self.workspace_count().to_string())],
                );
                let messages = vec![PromptMessage::new("system", &content)];
                InvokePromptResponse::success(&correlation_id, messages)
            }
            // ... other prompt variants
        };
        self.send_response(response, sender_id);
    }
}
```

**Prompt content example** (`workspace_management.md`):

```markdown
You are managing Hyprland workspaces. Current workspace count: {{workspace_count}}.

Available tools:
- `hyprland_switch_workspace` — Switch to a workspace by ID
- `hyprland_workspace_move_focused_window` — Move focused window to a workspace
- `hyprland_compositor_create_workspace` — Create a new workspace
- `hyprland_workspace_rename` — Rename a workspace
- `hyprland_workspace_swap_active` — Swap active workspaces between monitors
- `hyprland_workspace_toggle_special` — Toggle special workspace

Query state first:
- `hyprland://workspace-snapshot` — Get full workspace snapshot

Steps:
1. Query the workspace snapshot to understand current layout
2. Perform the requested workspace operation
3. Confirm the result by querying the snapshot again
```

**Exit Criteria**: All prompts registered. Prompt invocation returns rendered template content.

### Phase 5: McpCapabilitiesRegistrator — Full Registration

**Dependencies**: Phase 2, Phase 3, Phase 4

**Tasks**:

1. Extend `services/hyprland/src/mcp/capabilities.rs` — implement `McpCapabilitiesRegistrator` for `HyprlandService` with all tools, resources, and prompts
   registered in `register_mcp_capabilities()`
2. **Tool registration** — for each tool, create a `RegisterToolMessage::new(name, description, input_schema)` where:
    - `name` follows `hyprland_<category>_<action>` convention
    - `description` is detailed enough for an LLM to understand when and how to use the tool (include parameter semantics, valid values, and expected outcome)
    - `input_schema` is generated via `serde_json::to_string(&schema_for!(ArgsStruct))`
3. **Resource registration** — for each resource, create a `RegisterResourceMessage::new(uri, name, description, mime_type)` where:
    - `uri` follows `hyprland://<resource>` convention
    - `name` is a short human-readable title
    - `description` explains what data the resource returns and when to use it
    - `mime_type` is `"application/json"`
4. **Prompt registration** — for each prompt, create a `RegisterPromptMessage::with_memory(name, description, arguments_schema, memory_query, entity_filter)`
   where:
    - `name` follows `hyprland_<workflow>` convention
    - `description` explains the workflow the prompt guides
    - `arguments_schema` is generated via `schemars::schema_for!()` or `NoArgs` schema
    - `memory_query` is a natural language query for SemanticMemory recall
    - `entity_filter` is a comma-separated list of relevant entity names
5. Broadcast all registration messages via `broadcaster.broadcast_message_to_topic()`
6. Ensure the MCP server discovers and exposes all registered capabilities
7. Add `schemars` dependency to `services/hyprland/Cargo.toml` if not already present

**Registration example** (following weather service pattern):

```rust
impl McpCapabilitiesRegistrator for HyprlandService {
    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        // --- Resources ---
        let state_resource = RegisterResourceMessage::new(
            "hyprland://state",
            "Hyprland State",
            "Current Hyprland compositor state: active window (class, title, workspace), fullscreen status, keyboard layout, submap. Use this to understand the current compositor state before performing actions.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(state_resource);

        let snapshot_resource = RegisterResourceMessage::new(
            "hyprland://workspace-snapshot",
            "Workspace Snapshot",
            "Full snapshot of all workspaces, their IDs, names, and windows. Use this to understand the complete workspace layout before performing workspace operations.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(snapshot_resource);

        // --- Tools ---
        let kill_active_schema = serde_json::to_string(&schema_for!(NoArgs)).unwrap_or_default();
        let kill_active_tool = RegisterToolMessage::new(
            "hyprland_window_kill_active",
            "Kill (force-close) the currently focused window. Use this when the user wants to close the active window forcefully.",
            &kill_active_schema,
        );
        broadcaster.broadcast_message_to_topic(kill_active_tool);

        let close_window_schema = serde_json::to_string(&schema_for!(WindowIdentifierArgs)).unwrap_or_default();
        let close_window_tool = RegisterToolMessage::new(
            "hyprland_window_close",
            "Close a specific window by address or title. Use this when the user wants to close a specific window that is not the active window.",
            &close_window_schema,
        );
        broadcaster.broadcast_message_to_topic(close_window_tool);

        // --- Prompts ---
        let workspace_management_prompt = RegisterPromptMessage::with_memory(
            "hyprland_workspace_management",
            "Returns a system prompt with workspace management instructions, available tools, and current workspace count. Guides the LLM through workspace creation, switching, moving windows, and renaming.",
            &no_args_schema,
            "Hyprland workspace layout preference",
            "workspace,monitor",
        );
        broadcaster.broadcast_message_to_topic(workspace_management_prompt);

        let overview_prompt = RegisterPromptMessage::with_memory(
            "hyprland_overview",
            "Comprehensive overview of all Hyprland MCP tools, resources, and prompts. Use this when the LLM needs to understand the full scope of available compositor controls.",
            &no_args_schema,
            "Hyprland compositor configuration and usage preference",
            "workspace,window,monitor,group,system",
        );
        broadcaster.broadcast_message_to_topic(overview_prompt);
    }
}
```

**Exit Criteria**: `McpCapabilitiesRegistrator` registers all tools, resources, and prompts. MCP server lists all Hyprland capabilities. External MCP clients
can discover and invoke them.

### Phase 6: Testing & Verification

**Dependencies**: Phase 5

**Tasks**:

1. **Tool invocation tests** — verify each MCP tool correctly dispatches to the Hyprland service:
    - Window tools: kill, close, focus, move, resize, swap, cycle, center
    - Workspace tools: switch, move, rename, swap, toggle special, create
    - Toggle tools: fullscreen, dpms, group, opaque, pin, pseudo, split, fake fullscreen
    - System tools: add/remove master, orientation, exit, reload, custom, global, lock groups
    - CTL tools: kill, notify, output create/remove, plugin load/unload, set cursor, set prop, switch xkb
2. **Resource query tests** — verify each MCP resource returns correct data:
    - State, active window, workspace snapshot, monitor changed
    - Latest window/workspace/group/layer/system status events
3. **Prompt invocation tests** — verify each prompt returns correct guidance content
4. **Error handling tests** — verify invalid args produce `InvokeToolResponse::error` with descriptive messages
5. **Graceful degradation** — verify tools respond correctly when Hyprland is not running
6. **No `unwrap()` or `expect()`** in any new code path

**Exit Criteria**: All tests pass. No panics in production code paths.

### Phase 7: Documentation

**Dependencies**: Phase 6

**Tasks**:

1. Update `book/src/features/hyprland.md` with MCP tools/resources/prompts reference
2. Create `book/src/architecture/hyprland-mcp.md` with architecture diagram
3. Update `README.md` feature list
4. Add MCP tools reference table to book

**Exit Criteria**: `mdbook build` succeeds. Documentation covers all tools, resources, and prompts.

---

## Dependencies

| Crate               | New Dependencies                                                               |
|---------------------|--------------------------------------------------------------------------------|
| `model/hyprland`    | `schemars` (for JSON schema generation), `smearor-model-mcp` (already present) |
| `services/hyprland` | `schemars` (for schema generation in capabilities), no other new deps          |
| `mcp-server`        | No new dependencies                                                            |

---

## Testing Checklist

- [ ] Each MCP tool dispatches the correct Hyprland command
- [ ] Each MCP resource returns current state
- [ ] Each MCP prompt returns correct guidance content
- [ ] Invalid tool arguments produce descriptive error responses
- [ ] Missing required parameters produce error responses (not silent defaults)
- [ ] Tools work when Hyprland is running
- [ ] Tools return graceful error when Hyprland is not running
- [ ] No `unwrap()` or `expect()` in production code paths
- [ ] **No manual JSON serialization** — all args and responses use structs with `Serialize`/`Deserialize` derives
- [ ] **No manual JSON deserialization** — all argument parsing uses `serde_json::from_str::<ArgsStruct>()` with explicit error handling
- [ ] All args structs derive `Default` for fallback deserialization
- [ ] All args structs derive `JsonSchema` for schema generation
- [ ] Tool names follow `hyprland_<category>_<action>` convention
- [ ] Resource URIs follow `hyprland://<resource>` convention
- [ ] Prompt names follow `hyprland_<workflow>` convention
- [ ] Prompt templates are in `services/hyprland/data/prompts/` and loaded via `include_str!()`
- [ ] Prompt templates use `render_template()` for placeholder substitution
- [ ] `McpCapabilitiesRegistrator` implemented with all tools, resources, and prompts registered
- [ ] Each `RegisterToolMessage` has LLM-friendly `description` (when and how to use)
- [ ] Each `RegisterResourceMessage` has LLM-friendly `description` (what data it returns)
- [ ] Each `RegisterPromptMessage` uses `with_memory()` with `memory_query` and `entity_filter`
- [ ] Overview guide prompt covers all available tools, resources, and prompts
- [ ] Quick reference prompt provides compact one-line-per-tool listing
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes
- [ ] `cargo build -p smearor_hyprland_model` succeeds
- [ ] `cargo build -p smearor_hyprland_service` succeeds

---

## Future Enhancements

- **Event subscription via MCP**: Allow MCP clients to subscribe to real-time status events (requires MCP server streaming support)
- **Batch dispatch**: A single MCP tool that executes multiple dispatches atomically
- **Workspace rules engine**: Declarative workspace rules (auto-assign apps to workspaces) exposed via MCP
- **Keybinding simulation**: MCP tool to simulate keybindings via `hyprland_system_global`
- **Monitor layout presets**: MCP prompt + tools for saving/restoring monitor layouts
- **Window rules management**: MCP tools for dynamic window rule creation/removal
