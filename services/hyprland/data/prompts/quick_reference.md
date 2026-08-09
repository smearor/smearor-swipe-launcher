# Hyprland MCP Quick Reference

Current state: `{{active_window_class}}` on workspace `{{active_workspace_id}}`, fullscreen=`{{is_fullscreen}}`, layout=`{{keyboard_layout}}`

## Resources

| URI                             | Description      |
|---------------------------------|------------------|
| `hyprland://state`              | compositor state |
| `hyprland://active-window`      | focused window   |
| `hyprland://workspace-snapshot` | all workspaces   |
| `hyprland://workspaces`         | workspace list   |
| `hyprland://monitors`           | monitor events   |
| `hyprland://window-status`      | window events    |
| `hyprland://workspace-status`   | workspace events |
| `hyprland://group-status`       | group events     |
| `hyprland://layer-status`       | layer events     |
| `hyprland://system-status`      | system events    |

## Window Tools

| Tool                                    | Description                |
|-----------------------------------------|----------------------------|
| `hyprland_window_kill_active`           | kill focused window        |
| `hyprland_window_close`                 | close window by addr/title |
| `hyprland_window_focus`                 | focus window by addr/title |
| `hyprland_window_focus_master`          | focus master window        |
| `hyprland_window_focus_urgent_or_last`  | focus urgent/last          |
| `hyprland_window_focus_current_or_last` | focus current/last         |
| `hyprland_window_cycle`                 | cycle focus next/prev      |
| `hyprland_window_swap`                  | swap with next/prev        |
| `hyprland_window_swap_with_master`      | swap with master           |
| `hyprland_window_center`                | center active window       |
| `hyprland_window_move`                  | move active by direction   |
| `hyprland_window_move_pixel`            | move by pixel offset       |
| `hyprland_window_resize_active`         | resize by direction+amount |
| `hyprland_window_resize_pixel`          | resize by pixel dims       |
| `hyprland_window_move_into_group`       | move into group            |
| `hyprland_window_change_group_active`   | change group active        |
| `hyprland_window_move_cursor`           | move cursor to position    |
| `hyprland_window_move_cursor_to_corner` | move cursor to corner      |
| `hyprland_window_move_focus`            | move focus by direction    |
| `hyprland_window_exec`                  | execute a command          |
| `hyprland_window_pass`                  | pass key/mouse event       |

## Workspace Tools

| Tool                                            | Description                      |
|-------------------------------------------------|----------------------------------|
| `hyprland_workspace_move`                       | move to workspace                |
| `hyprland_workspace_move_focused_window`        | move focused window to workspace |
| `hyprland_workspace_move_focused_window_silent` | move focused window silently     |
| `hyprland_workspace_move_to_silent`             | move to workspace silently       |
| `hyprland_workspace_rename`                     | rename workspace                 |
| `hyprland_workspace_swap_active`                | swap active workspaces           |
| `hyprland_workspace_toggle_special`             | toggle special workspace         |
| `hyprland_workspace_move_current_to_monitor`    | move workspace to monitor        |

## Toggle Tools

| Tool                              | Description                |
|-----------------------------------|----------------------------|
| `hyprland_toggle_fullscreen`      | toggle fullscreen          |
| `hyprland_toggle_dpms`            | toggle display power       |
| `hyprland_toggle_fake_fullscreen` | toggle fake fullscreen     |
| `hyprland_toggle_group`           | toggle window group        |
| `hyprland_toggle_opaque`          | toggle opacity             |
| `hyprland_toggle_pin`             | toggle pin (always on top) |
| `hyprland_toggle_pseudo`          | toggle pseudo tiling       |
| `hyprland_toggle_split`           | toggle split orientation   |

## System Tools

| Tool                                    | Description            |
|-----------------------------------------|------------------------|
| `hyprland_system_add_master`            | add master slot        |
| `hyprland_system_remove_master`         | remove master slot     |
| `hyprland_system_orientation`           | set layout orientation |
| `hyprland_system_bring_active_to_top`   | bring active to top    |
| `hyprland_system_exit`                  | exit Hyprland          |
| `hyprland_system_force_renderer_reload` | reload renderer        |
| `hyprland_system_custom`                | custom dispatch        |
| `hyprland_system_global`                | global dispatch        |
| `hyprland_system_lock_groups`           | lock/unlock groups     |
| `hyprland_system_move_out_of_group`     | move out of group      |
| `hyprland_system_set_cursor`            | set cursor theme+size  |
| `hyprland_system_pass`                  | pass key event         |

## Control Tools

| Tool                             | Description            |
|----------------------------------|------------------------|
| `hyprland_ctl_kill`              | kill by address        |
| `hyprland_ctl_notify`            | send notification      |
| `hyprland_ctl_output_create`     | create virtual output  |
| `hyprland_ctl_output_remove`     | remove virtual output  |
| `hyprland_ctl_plugin_load`       | load plugin            |
| `hyprland_ctl_plugin_unload`     | unload plugin          |
| `hyprland_ctl_reload`            | reload config          |
| `hyprland_ctl_set_cursor`        | set cursor (ctl)       |
| `hyprland_ctl_set_error`         | set error state        |
| `hyprland_ctl_set_prop`          | set window property    |
| `hyprland_ctl_switch_xkb_layout` | switch keyboard layout |

## Compositor Tools

| Tool                                   | Description                          |
|----------------------------------------|--------------------------------------|
| `hyprland_compositor_create_workspace` | create workspace relative to another |
| `hyprland_compositor_switch_workspace` | switch to workspace by ID            |

## Prompts

| Prompt                     | Description                |
|----------------------------|----------------------------|
| `hyprland_overview`        | full capability guide      |
| `hyprland_window_guide`    | window management guide    |
| `hyprland_workspace_guide` | workspace management guide |
