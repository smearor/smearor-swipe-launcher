# Concept: GNOME Shell Extension — Strut Orchestrator

This document describes the concept for a **GNOME Shell Extension** that acts as an **orchestrator** for the Smearor Swipe Launcher under GNOME/Wayland. The
extension compensates for two fundamental GNOME limitations:

1. **GNOME does not implement the Layer Shell protocol** — no edge anchoring, no exclusive zones.
2. **GNOME/Wayland forbids client-side window positioning** — applications cannot move or resize their own windows. Only the compositor (Mutter) or code running
   inside the Shell process can position windows.

The extension solves both problems by running **inside the GNOME Shell process**, where it has access to `Main.layoutManager` (for strut reservation) and
`MetaWindow` APIs (for forced window placement). It manages **up to 4 independent strut zones** — one per screen edge — and positions each registered launcher
window at its corresponding edge.

The extension is a **pure JavaScript artifact**. It is deliberately kept separate from the Rust codebase. The only coupling is a **D-Bus interface** through
which Rust launcher instances register themselves.

---

## 1. Problem Statement

### 1.1 Layer Shell Is Not Supported by GNOME

The launcher relies on the **Wayland Layer Shell protocol** (`wlr-layer-shell-unstable-v1`) for edge anchoring and exclusive zone reservation. This protocol is
implemented by wlroots-based compositors (Sway, Hyprland, River, Labwc) and by KDE Plasma's KWin.

**GNOME's Mutter compositor does not implement the Layer Shell protocol.** This is a deliberate upstream decision — GNOME's design philosophy reserves
shell-level surfaces for GNOME Shell itself, not for client applications.

As a result, when the launcher runs under GNOME/Wayland:

- **No edge anchoring:** The `gtk4_layer_shell` calls (`set_anchor`, `set_layer`) silently fail or are ignored. The window appears as a regular floating window,
  not docked to any edge.
- **No exclusive zone:** `set_exclusive_zone` has no effect. Maximised windows cover the full screen, overlapping the launcher.
- **No panel-like behaviour:** The launcher cannot behave as a system panel that reduces the usable workarea.

### 1.2 Client-Side Window Positioning Is Forbidden Under Wayland

Under the Wayland protocol, **clients cannot position their own windows**. The compositor decides where windows appear. GTK's `window.set_position()` or
`window.move()` calls are ignored under Wayland — they only work on X11.

This means the Rust launcher binary **cannot** place its own window at a screen edge, even if it knows the exact coordinates. Only code running inside the
compositor (or inside GNOME Shell, which has privileged access to Mutter's `MetaWindow` API) can move windows.

### 1.3 The Impact

The launcher's primary use case is a **touch-optimised ribbon on all four edges of a large tabletop touchscreen**. Without edge anchoring, exclusive zones, and
window positioning, the launcher is completely unusable on GNOME desktops.

### 1.4 Why Not Patch Mutter or Use a Different Compositor?

- **Patching Mutter** is not feasible for end users and would break on every GNOME update.
- **Telling users to switch compositors** is hostile and defeats the purpose of a cross-compositor launcher.
- **GNOME Shell Extensions** are the officially supported mechanism for modifying GNOME Shell behaviour. They run inside the Shell process and have access to
  `Main.layoutManager` (struts/workarea) and `global.display.list_all_windows()` / `MetaWindow` methods (window positioning).

---

## 2. Solution Overview

The GNOME Shell Extension acts as an **orchestrator** with two responsibilities:

1. **Strut management:** Create up to 4 invisible `St.Widget` actors (one per edge) registered with `affectsStruts: true`, so maximised windows avoid the
   reserved regions.
2. **Window placement:** Find each launcher's `MetaWindow` by PID and force-position it at the correct edge using `move_resize_frame()`, `make_above()`, and
   `stick()`.

The extension and the Rust binaries are **completely decoupled**. The only communication channel is **D-Bus**.

```
┌────────────────────────────────────────────────────────────────────────┐
│                        GNOME Shell (Mutter)                            │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                    GNOME Extension (Orchestrator)                │  │
│  │                                                                  │  │
│  │  Strut-Map: [                                                    │  │
│  │    "top"    => St.Widget (Height: 80px, affectsStruts: true)     │  │
│  │    "bottom" => St.Widget (Height: 80px, affectsStruts: true)     │  │
│  │    "left"   => St.Widget (Width:  80px, affectsStruts: true)     │  │
│  │    "right"  => St.Widget (Width:  80px, affectsStruts: true)     │  │
│  │  ]                                                               │  │
│  │                                                                  │  │
│  │  Window Positioning Logic (via MetaWindow API):                  │  │
│  │    • Top Window    ──► move_resize_frame(x, y_top, w, size)      │  │
│  │    • Bottom Window ──► move_resize_frame(x, y_bot, w, size)      │  │
│  │    • Left Window   ──► move_resize_frame(x_left, y, size, h)     │  │
│  │    • Right Window  ──► move_resize_frame(x_right, y, size, h)    │  │
│  │                                                                  │  │
│  │  Per-window flags: make_above() + stick()                        │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                  ▲                                     │
│                           D-Bus  │  RegisterLauncher(PID, Edge, Size)  │
│                                  │  UpdateLauncher(InstanceID, Size)   │
│                                  │  UnregisterLauncher(InstanceID)     │
│                                  │                                     │
│ ┌──────────────────────┐ ┌───────┴──────────────┐ ┌──────────────────┐ │
│ │ Rust Launcher Top    │ │ Rust Launcher Bottom │ │ Rust Launcher ...│ │
│ │ (Rotation: 180°)     │ │ (Rotation: 0°)       │ │ (Rotation: 90/270)│ │
│ │ PID: 12345           │ │ PID: 12346           │ │ PID: 12347/12348  │ │
│ └──────────────────────┘ └──────────────────────┘ └──────────────────┘ │
└────────────────────────────────────────────────────────────────────────┘
```

### 2.1 Separation of Concerns

| Component                 | Language   | Location                                               | Responsibility                                                        |
|---------------------------|------------|--------------------------------------------------------|-----------------------------------------------------------------------|
| **GNOME Shell Extension** | JavaScript | `~/.local/share/gnome-shell/extensions/`               | Reserve struts, position windows via MetaWindow API, expose D-Bus API |
| **Rust Launcher Binary**  | Rust       | System `PATH` (e.g. `/usr/bin/smearor-swipe-launcher`) | Render UI, manage plugins, call D-Bus to register/unregister          |

The extension **never imports or calls** any Rust code. The Rust binary **never imports or calls** any JavaScript code. The Rust binary does **not** attempt to
position its own window — it delegates that entirely to the extension.

---

## 3. Extension Structure

The extension follows the **GNOME 45+ ESM module format**.

### 3.1 Directory Layout

```
~/.local/share/gnome-shell/extensions/launcher-strut@smearor.local/
├── metadata.json      # Extension metadata and GNOME version compatibility
├── extension.js       # Main logic: strut management + window positioning + D-Bus server
└── schemas/           # (Optional) GSettings schema for persistent state
    └── org.gnome.shell.extensions.launcher-strut.gschema.xml
```

### 3.2 `metadata.json`

```json
{
  "uuid": "launcher-strut@smearor.local",
  "name": "Launcher Strut Orchestrator",
  "description": "Reserves exclusive screen space and positions Smearor Swipe Launcher instances at screen edges",
  "shell-version": ["45", "46", "47", "48"],
  "settings-schema": "org.gnome.shell.extensions.launcher-strut"
}
```

- **`uuid`**: Unique identifier. The `@smearor.local` suffix indicates a locally installed extension.
- **`shell-version`**: Compatible with GNOME 45 through 48 (ESM module era).

---

## 4. D-Bus Interface — Multi-Instance Registration

Each launcher instance registers itself at startup via D-Bus. The extension manages a map of active instances, each identified by a unique `instance_id` string.

### 4.1 D-Bus Bus Name and Object Path

| Property        | Value                                       |
|-----------------|---------------------------------------------|
| **Bus name**    | `org.gnome.Shell.Extensions.LauncherStrut`  |
| **Object path** | `/org/gnome/Shell/Extensions/LauncherStrut` |
| **Interface**   | `org.gnome.Shell.Extensions.LauncherStrut`  |

### 4.2 Methods

| Method                   | Signature                                                | Description                                                                                                 |
|--------------------------|----------------------------------------------------------|-------------------------------------------------------------------------------------------------------------|
| **`RegisterLauncher`**   | `(u pid, s edge, u size, u monitor)` → `(s instance_id)` | Register a launcher instance. Creates strut, finds window by PID, positions it. Returns unique instance ID. |
| **`UpdateLauncher`**     | `(s instance_id, u size)` → `()`                         | Update the strut size for an existing instance. Re-positions the window.                                    |
| **`UnregisterLauncher`** | `(s instance_id)` → `()`                                 | Unregister an instance. Removes the strut and releases the reserved space.                                  |
| **`GetInstances`**       | `() → (aa{sv})`                                          | Returns an array of dicts with all active instances (pid, edge, size, monitor).                             |

### 4.3 D-Bus Introspection XML

```xml
<node>
  <interface name="org.gnome.Shell.Extensions.LauncherStrut">
    <method name="RegisterLauncher">
      <arg type="u" name="pid" direction="in"/>
      <arg type="s" name="edge" direction="in"/>
      <arg type="u" name="size" direction="in"/>
      <arg type="u" name="monitor" direction="in"/>
      <arg type="s" name="instance_id" direction="out"/>
    </method>
    <method name="UpdateLauncher">
      <arg type="s" name="instance_id" direction="in"/>
      <arg type="u" name="size" direction="in"/>
    </method>
    <method name="UnregisterLauncher">
      <arg type="s" name="instance_id" direction="in"/>
    </method>
    <method name="GetInstances">
      <arg type="aa{sv}" name="instances" direction="out"/>
    </method>
  </interface>
</node>
```

### 4.4 Edge Values

The `edge` parameter is a string with one of these values:

| Edge value | Launcher rotation | Strut orientation   | Window geometry                               |
|------------|-------------------|---------------------|-----------------------------------------------|
| `"top"`    | 180°              | Horizontal (height) | Full monitor width, `size` px tall, at top    |
| `"bottom"` | 0°                | Horizontal (height) | Full monitor width, `size` px tall, at bottom |
| `"left"`   | 90°               | Vertical (width)    | `size` px wide, full monitor height, at left  |
| `"right"`  | 270°              | Vertical (width)    | `size` px wide, full monitor height, at right |

---

## 5. Extension Implementation — `extension.js`

The extension manages a `Map` of active launcher instances. Each instance has its own strut actor. The extension uses `MetaWindow` APIs to find and position
each launcher's window.

```javascript
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';
import Meta from 'gi://Meta';

const DBUS_INTERFACE_XML = `
<node>
  <interface name="org.gnome.Shell.Extensions.LauncherStrut">
    <method name="RegisterLauncher">
      <arg type="u" name="pid" direction="in"/>
      <arg type="s" name="edge" direction="in"/>
      <arg type="u" name="size" direction="in"/>
      <arg type="u" name="monitor" direction="in"/>
      <arg type="s" name="instance_id" direction="out"/>
    </method>
    <method name="UpdateLauncher">
      <arg type="s" name="instance_id" direction="in"/>
      <arg type="u" name="size" direction="in"/>
    </method>
    <method name="UnregisterLauncher">
      <arg type="s" name="instance_id" direction="in"/>
    </method>
    <method name="GetInstances">
      <arg type="aa{sv}" name="instances" direction="out"/>
    </method>
  </interface>
</node>
`;

export default class LauncherStrutExtension extends Extension {
    enable() {
        // Map<instance_id, { pid, edge, size, monitorIndex, strutActor }>
        this._instances = new Map();

        // D-Bus server
        this._dbusImpl = Gio.DBusExportedObject.wrapJSObject(DBUS_INTERFACE_XML, this);
        this._dbusImpl.export(
            Gio.DBus.session,
            '/org/gnome/Shell/Extensions/LauncherStrut'
        );
        Gio.DBus.session.own_name(
            'org.gnome.Shell.Extensions.LauncherStrut',
            Gio.BusNameOwnerFlags.NONE,
            null,
            null
        );
    }

    disable() {
        if (this._dbusImpl) {
            this._dbusImpl.unexport();
            this._dbusImpl = null;
        }
        for (const [instanceId, instance] of this._instances) {
            this._removeStrut(instance.strutActor);
        }
        this._instances.clear();
    }

    // --- D-Bus method handlers ---

    RegisterLauncher(pid, edge, size, monitorIndex) {
        const instanceId = `${edge}_${pid}`;

        // Remove existing instance with same ID if re-registering
        if (this._instances.has(instanceId)) {
            this._cleanupInstance(instanceId);
        }

        // 1. Create strut widget for this edge
        const strutActor = this._createStrut(edge, size, monitorIndex);

        // 2. Store instance
        this._instances.set(instanceId, {
            pid,
            edge,
            size,
            monitorIndex,
            strutActor
        });

        // 3. Position the launcher window (with retry, since the window
        //    may not be fully created yet when the D-Bus call arrives)
        this._positionWindowWithRetry(pid, edge, size, monitorIndex, instanceId);

        return [instanceId];
    }

    UpdateLauncher(instanceId, size) {
        const instance = this._instances.get(instanceId);
        if (!instance) return;

        instance.size = size;
        this._updateStrut(instance.strutActor, instance.edge, size, instance.monitorIndex);
        this._positionWindowWithRetry(
            instance.pid, instance.edge, size, instance.monitorIndex, instanceId
        );
    }

    UnregisterLauncher(instanceId) {
        this._cleanupInstance(instanceId);
    }

    GetInstances() {
        const result = [];
        for (const [id, inst] of this._instances) {
            result.push({
                id: GLib.Variant.new_string(id),
                pid: GLib.Variant.new_uint32(inst.pid),
                edge: GLib.Variant.new_string(inst.edge),
                size: GLib.Variant.new_uint32(inst.size),
                monitor: GLib.Variant.new_uint32(inst.monitorIndex)
            });
        }
        return [result];
    }

    // --- Strut management ---

    _createStrut(edge, size, monitorIndex) {
        const strutActor = new St.Widget({
            name: `launcher-strut-${edge}`,
            visible: true,
            opacity: 0
        });

        Main.layoutManager.addChrome(strutActor, {
            affectsStruts: true,
            trackFullscreen: true
        });

        this._updateStrut(strutActor, edge, size, monitorIndex);
        return strutActor;
    }

    _updateStrut(strutActor, edge, size, monitorIndex) {
        const monitor = Main.layoutManager.monitors[monitorIndex]
            || Main.layoutManager.primaryMonitor;
        if (!monitor) return;

        switch (edge) {
            case 'top':
                strutActor.set_size(monitor.width, size);
                strutActor.set_position(monitor.x, monitor.y);
                break;
            case 'bottom':
                strutActor.set_size(monitor.width, size);
                strutActor.set_position(monitor.x, monitor.y + monitor.height - size);
                break;
            case 'left':
                strutActor.set_size(size, monitor.height);
                strutActor.set_position(monitor.x, monitor.y);
                break;
            case 'right':
                strutActor.set_size(size, monitor.height);
                strutActor.set_position(monitor.x + monitor.width - size, monitor.y);
                break;
        }
    }

    _removeStrut(strutActor) {
        if (strutActor) {
            Main.layoutManager.removeChrome(strutActor);
            strutActor.destroy();
        }
    }

    _cleanupInstance(instanceId) {
        const instance = this._instances.get(instanceId);
        if (!instance) return;
        this._removeStrut(instance.strutActor);
        this._instances.delete(instanceId);
    }

    // --- Window positioning via MetaWindow API ---

    _positionWindowWithRetry(pid, edge, size, monitorIndex, instanceId, retries = 10) {
        const success = this._positionWindow(pid, edge, size, monitorIndex);
        if (!success && retries > 0) {
            // The window may not be mapped yet. Retry after 100ms.
            GLib.timeout_add(GLib.PRIORITY_DEFAULT, 100, () => {
                // Only retry if the instance still exists (not unregistered)
                if (this._instances.has(instanceId)) {
                    this._positionWindowWithRetry(pid, edge, size, monitorIndex, instanceId, retries - 1);
                }
                return GLib.SOURCE_REMOVE;
            });
        }
    }

    _positionWindow(pid, edge, size, monitorIndex) {
        const monitor = Main.layoutManager.monitors[monitorIndex]
            || Main.layoutManager.primaryMonitor;
        if (!monitor) return false;

        // Find the MetaWindow belonging to this PID
        const windows = global.display.list_all_windows();
        const metaWindow = windows.find(w => w.get_pid() === pid);

        if (!metaWindow) return false;

        // Calculate geometry for the edge
        let x = monitor.x;
        let y = monitor.y;
        let w = monitor.width;
        let h = monitor.height;

        switch (edge) {
            case 'top':
                h = size;
                break;
            case 'bottom':
                y = monitor.y + monitor.height - size;
                h = size;
                break;
            case 'left':
                w = size;
                break;
            case 'right':
                x = monitor.x + monitor.width - size;
                w = size;
                break;
        }

        // Unmaximise first — move_resize_frame is ignored on maximised windows
        if (metaWindow.get_maximized() !== Meta.MaximizeFlags.NONE) {
            metaWindow.unmaximize(Meta.MaximizeFlags.BOTH);
        }

        // Force position and size
        metaWindow.move_resize_frame(true, x, y, w, h);

        // Keep above other windows (equivalent to Layer Shell TOP layer)
        metaWindow.make_above();

        // Show on all workspaces (equivalent to Layer Shell panel behaviour)
        metaWindow.stick();

        return true;
    }
}
```

### 5.1 Key API Calls Explained

| API call                                                     | Purpose                                                                                      |
|--------------------------------------------------------------|----------------------------------------------------------------------------------------------|
| `Main.layoutManager.addChrome(actor, {affectsStruts: true})` | Registers the invisible widget so GNOME subtracts its area from the workarea.                |
| `global.display.list_all_windows()`                          | Returns all `MetaWindow` objects known to Mutter.                                            |
| `metaWindow.get_pid()`                                       | Returns the PID of the process that created the window.                                      |
| `metaWindow.move_resize_frame(user_op, x, y, w, h)`          | Force-moves and resizes the window. `user_op = true` marks it as a user-initiated operation. |
| `metaWindow.unmaximize(flags)`                               | Unmaximises the window so `move_resize_frame` takes effect.                                  |
| `metaWindow.make_above()`                                    | Sets the window above normal windows (Layer Shell TOP equivalent).                           |
| `metaWindow.stick()`                                         | Shows the window on all workspaces (panel behaviour).                                        |

### 5.2 Retry Logic for Window Discovery

When a launcher instance calls `RegisterLauncher` via D-Bus, its GTK window may not yet be mapped in Mutter's window list. The extension retries
`_positionWindow` up to 10 times with 100ms intervals. This handles the race condition between the GTK window being created and the D-Bus registration arriving.

---

## 6. Decoupling of UI Rotation and Strut Geometry

A critical design principle: **the extension handles geometry, the Rust binary handles visual rotation.**

### 6.1 Extension Responsibility — Geometric Placement

The extension positions the window at the correct screen edge and reserves the strut:

- **top:** Full monitor width at the top, `size` pixels tall.
- **bottom:** Full monitor width at the bottom, `size` pixels tall.
- **left:** Full monitor height at the left, `size` pixels wide.
- **right:** Full monitor height at the right, `size` pixels wide.

The extension does **not** know or care about the visual content rotation. It only cares about where the window rectangle sits on the monitor.

### 6.2 Rust Binary Responsibility — Visual UI Rotation

The Rust binary renders its UI with the correct rotation for the user sitting at that edge:

- **0° (bottom):** Text and icons are upright when viewed from the bottom edge.
- **90° (left):** Content is rotated 90° counter-clockwise for a user at the left edge.
- **180° (top):** Content is rotated 180° for a user at the top edge.
- **270° (right):** Content is rotated 90° clockwise for a user at the right edge.

The `RotationWidget` from `smearor-wrot-rotation` handles this visual transformation inside the GTK widget tree. The extension never touches this — it only
positions the outer window rectangle.

### 6.3 Mapping Table

| Rotation (Rust config) | Edge (D-Bus `edge` parameter) | Extension places window at | Rust rotates UI content by  |
|------------------------|-------------------------------|----------------------------|-----------------------------|
| 0°                     | `"bottom"`                    | Bottom edge                | 0° (upright from bottom)    |
| 90°                    | `"left"`                      | Left edge                  | 90° CCW (upright from left) |
| 180°                   | `"top"`                       | Top edge                   | 180° (upright from top)     |
| 270°                   | `"right"`                     | Right edge                 | 90° CW (upright from right) |

---

## 7. Multi-Instance Tabletop Scenario

The primary use case is a **65" 4K tabletop touchscreen** with launcher ribbons on all four edges. Each edge has its own launcher instance with its own config,
rotation, and plugin set.

### 7.1 Startup Flow

1. **Autostart or systemd user service** launches 4 launcher instances, each with a different config file:

```bash
smearor-swipe-launcher --config config-bottom.toml &
smearor-swipe-launcher --config config-left.toml &
smearor-swipe-launcher --config config-top.toml &
smearor-swipe-launcher --config config-right.toml &
```

2. **Each instance** opens its GTK window (which appears at an arbitrary position under Wayland) and immediately calls D-Bus:

```
RegisterLauncher(pid=12345, edge="bottom", size=80, monitor=0)  → instance_id="bottom_12345"
RegisterLauncher(pid=12346, edge="left",    size=80, monitor=0)  → instance_id="left_12346"
RegisterLauncher(pid=12347, edge="top",    size=80, monitor=0)  → instance_id="top_12347"
RegisterLauncher(pid=12348, edge="right",   size=80, monitor=0)  → instance_id="right_12348"
```

3. **The extension:**
    - Creates 4 `St.Widget` struts (top, bottom, left, right), each with `affectsStruts: true`.
    - GNOME's workarea shrinks: maximised windows now occupy only the central rectangle, leaving 80px free on all four edges.
    - Finds each launcher's `MetaWindow` by PID and positions it exactly in the reserved strut zone:
        - Bottom window: `move_resize_frame(monitor.x, monitor.y + monitor.height - 80, monitor.width, 80)`
        - Top window: `move_resize_frame(monitor.x, monitor.y, monitor.width, 80)`
        - Left window: `move_resize_frame(monitor.x, monitor.y, 80, monitor.height)`
        - Right window: `move_resize_frame(monitor.x + monitor.width - 80, monitor.y, 80, monitor.height)`
    - Calls `make_above()` and `stick()` on each window.

4. **Result:** Four launcher ribbons docked at all four edges, each rotated for its respective viewer, with maximised applications confined to the central
   workarea.

### 7.2 Visual Result

```
┌──────────────────────────────────────────────────────────┐
│  [Launcher Top — Rotation 180°]                     80px │
├──────────────────────────────────────────────────────────┤
│                                                          │
│ 80px  [  Maximised Application / Desktop Workarea  ] 80px│
│                                                          │
├──────────────────────────────────────────────────────────┤
│  [Launcher Bottom — Rotation 0°]                    80px │
└──────────────────────────────────────────────────────────┘
         ↑ Left (90°)              Right (270°) ↑
```

### 7.3 Dynamic Size Updates

If a launcher instance changes its height (e.g., layout profile switch, area expand/collapse), it calls:

```
UpdateLauncher("bottom_12345", 120)
```

The extension:

1. Resizes the bottom strut to 120px (workarea shrinks further).
2. Re-positions the bottom launcher window to the new 120px height.

### 7.4 Clean Shutdown

When a launcher instance exits, it calls:

```
UnregisterLauncher("bottom_12345")
```

The extension:

1. Removes the bottom strut (workarea expands).
2. Deletes the instance from its map.

If the launcher process is killed without calling `UnregisterLauncher`, the extension detects the window closure via a `window-removed` signal on `display` and
cleans up automatically.

---

## 8. Rust Binary Integration

The Rust launcher binary does **not** contain any GNOME Shell Extension code. It does **not** attempt to position its own window. It interacts with the
extension exclusively via D-Bus.

### 8.1 Detection: Am I Running Under GNOME?

At startup, the launcher checks whether the session is GNOME:

```rust
fn is_gnome_session() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|v| v.to_lowercase().contains("gnome"))
        .unwrap_or(false)
}
```

### 8.2 D-Bus Proxy

Using the `zbus` crate (already a workspace dependency):

```rust
#[zbus::proxy(
    interface = "org.gnome.Shell.Extensions.LauncherStrut",
    default_service = "org.gnome.Shell.Extensions.LauncherStrut",
    default_path = "/org/gnome/Shell/Extensions/LauncherStrut"
)]
trait LauncherStrut {
    async fn register_launcher(
        &self,
        pid: u32,
        edge: &str,
        size: u32,
        monitor: u32,
    ) -> zbus::Result<String>;

    async fn update_launcher(&self, instance_id: &str, size: u32) -> zbus::Result<()>;
    async fn unregister_launcher(&self, instance_id: &str) -> zbus::Result<()>;
    async fn get_instances(&self) -> zbus::Result<Vec<HashMap<String, zbus::zvariant::OwnedValue>>>;
}
```

### 8.3 Startup Sequence Under GNOME

1. **Launcher starts.** Detects GNOME session.
2. **Skips Layer Shell initialisation.** Does not call `window.init_layer_shell()`.
3. **Opens GTK window.** The window appears at an arbitrary position (Wayland default). The launcher does **not** attempt to move it.
4. **Connects to D-Bus.** Obtains a `LauncherStrutProxy` via `zbus`.
5. **Registers itself.** Calls `RegisterLauncher` with its own PID (`std::process::id()`), the edge derived from `rotation`, the `exclusive_zone` value as
   `size`, and the `monitor` index.
6. **Extension positions the window.** The extension finds the window by PID and snaps it to the correct edge.
7. **On size change** (layout profile switch, area expand/collapse): Calls `UpdateLauncher` with the new size.
8. **On shutdown:** Calls `UnregisterLauncher` before exiting.

### 8.4 Rotation-to-Edge Mapping in Rust

```rust
fn rotation_to_edge(rotation_degrees: f32) -> &'static str {
    match rotation_degrees as i32 {
        0 => "bottom",
        90 => "left",
        180 => "top",
        270 => "right",
        _ => "bottom",
    }
}
```

### 8.5 No Window Positioning Code in Rust

Under GNOME, the Rust binary **must not** call any of:

- `window.set_position()`
- `window.move()`
- `window.set_default_size()` with edge-calculated coordinates
- `window.set_keep_above()`

All of these are either ignored under Wayland or conflict with the extension's `MetaWindow` positioning. The binary simply creates the window with its desired
**size** (width for left/right, height for top/bottom) and lets the extension handle placement.

---

## 9. Configuration Integration

### 9.1 New Config Section: `[gnome_strut]`

```toml
[gnome_strut]
# Whether to use the GNOME Shell Extension under GNOME.
# "auto" = auto-detect GNOME and use the extension.
# false = do not use the extension (Layer Shell mode or fallback).
enabled = "auto"

# D-Bus bus name of the extension
# dbus_name = "org.gnome.Shell.Extensions.LauncherStrut"
```

### 9.2 Relationship to Existing `[launcher]` Config

The existing `[launcher]` section remains the source of truth. Under GNOME, these values are translated to D-Bus calls:

| Config field     | D-Bus parameter                                      |
|------------------|------------------------------------------------------|
| `rotation = 0`   | `edge = "bottom"`                                    |
| `rotation = 90`  | `edge = "left"`                                      |
| `rotation = 180` | `edge = "top"`                                       |
| `rotation = 270` | `edge = "right"`                                     |
| `exclusive_zone` | `size` (height for top/bottom, width for left/right) |
| `monitor`        | `monitor`                                            |
| `max_width`      | Not used under GNOME (window spans full edge)        |

### 9.3 Example Configs for 4-Edge Tabletop

**`config-bottom.toml`:**

```toml
[launcher]
rotation = 0
exclusive_zone = 80
monitor = 0
```

**`config-left.toml`:**

```toml
[launcher]
rotation = 90
exclusive_zone = 80
monitor = 0
```

**`config-top.toml`:**

```toml
[launcher]
rotation = 180
exclusive_zone = 80
monitor = 0
```

**`config-right.toml`:**

```toml
[launcher]
rotation = 270
exclusive_zone = 80
monitor = 0
```

---

## 10. Installation & Packaging

### 10.1 Extension Installation

```
~/.local/share/gnome-shell/extensions/launcher-strut@smearor.local/
```

For system-wide installation:

```
/usr/share/gnome-shell/extensions/launcher-strut@smearor.local/
```

### 10.2 Activation

```bash
gnome-extensions enable launcher-strut@smearor.local
gnome-extensions info launcher-strut@smearor.local
# Under Wayland: log out and back in once for GNOME to discover the extension
```

### 10.3 Rust Binary Installation

```bash
cargo install --path smearor-swipe-launcher
```

This places `smearor-swipe-launcher` in `~/.cargo/bin/` or `/usr/bin/`. The extension does not need to know the binary's path — it only reacts to D-Bus calls.

### 10.4 Combined Distribution Package

For distribution (`.deb`, `.rpm`, AUR):

1. Compiled Rust binary → `/usr/bin/smearor-swipe-launcher`
2. Extension directory → `/usr/share/gnome-shell/extensions/launcher-strut@smearor.local/`
3. Post-install script: `gnome-extensions enable launcher-strut@smearor.local`
4. `.desktop` autostart entries for multi-instance tabletop mode
5. Systemd user service files (optional, for managed multi-instance startup)

---

## 11. Limitations & Trade-offs

### 11.1 No True Layer Shell Semantics

| Feature                        | Layer Shell | GNOME Strut Extension            |
|--------------------------------|-------------|----------------------------------|
| Edge anchoring                 | Yes         | Yes (via MetaWindow positioning) |
| Exclusive zone                 | Yes         | Yes (via `affectsStruts`)        |
| Layer (background/top/overlay) | Yes         | Partial (always `make_above`)    |
| Keyboard interactivity mode    | Yes         | No (GNOME handles focus)         |
| Cross-compositor               | Yes         | GNOME-only                       |
| Client-side positioning        | N/A         | No (extension positions windows) |

### 11.2 Window Appears in Alt-Tab

Windows positioned by the extension still appear in the Alt-Tab switcher. This can be mitigated by:

- Setting the window type hint to `WINDOW_TYPE_DOCK` or `WINDOW_TYPE_TOOLBAR` in the Rust binary (GTK supports this via `GtkWindow::set_type_hint`).
- The extension can additionally call `metaWindow.set_skip_taskbar(true)` to hide it from the taskbar.

### 11.3 User Can Still Move Windows

Even after the extension positions a window, a user might drag it away from the edge with the mouse. The extension can counter this by:

- Listening to the `position-changed` signal on each positioned `MetaWindow`.
- Re-snapping the window to its edge if it is moved outside a tolerance threshold.

### 11.4 Extension Must Be Enabled

If the user disables the extension, struts disappear and windows are no longer repositioned. The Rust binary should detect D-Bus call failures and log a
warning. The window will remain at whatever position it last had.

### 11.5 GNOME Shell Restart

When GNOME Shell restarts, the extension re-enables, but all D-Bus connections are lost and all `MetaWindow` references are stale. The Rust binaries must:

1. Detect the D-Bus disconnection.
2. Wait for the extension to re-register on the bus (retry with backoff).
3. Re-register via `RegisterLauncher` with the same parameters.

The extension's `enable()` starts with an empty instance map, so all launchers must re-register.

### 11.6 Window Discovery Race Condition

The launcher's GTK window may not be mapped in Mutter's window list at the moment `RegisterLauncher` is called. The extension handles this with a retry loop
(100ms intervals, up to 10 attempts). If the window still cannot be found after 1 second, the strut is created but the window is not positioned. The launcher
should call `RegisterLauncher` after its window is fully realised (e.g., after the `realize` signal).

---

## 12. Testing Strategy

### 12.1 Manual Testing

1. Install the extension to `~/.local/share/gnome-shell/extensions/launcher-strut@smearor.local/`.
2. Enable it: `gnome-extensions enable launcher-strut@smearor.local`.
3. Log out and back in.
4. Start a single launcher: `smearor-swipe-launcher`.
5. Verify: the window snaps to the bottom edge; maximise a window — it should not cover the launcher.
6. Start 4 launchers with different configs — verify all 4 edges are occupied and the central workarea is reduced.
7. Disable the extension — verify struts disappear and windows are no longer repositioned.

### 12.2 D-Bus Testing

```bash
# Register a launcher (PID 12345, bottom edge, 80px, monitor 0)
gdbus call --session \
  --dest org.gnome.Shell.Extensions.LauncherStrut \
  --object-path /org/gnome/Shell/Extensions/LauncherStrut \
  --method org.gnome.Shell.Extensions.LauncherStrut.RegisterLauncher \
  12345 "bottom" 80 0

# Update size to 120px
gdbus call --session \
  --dest org.gnome.Shell.Extensions.LauncherStrut \
  --object-path /org/gnome/Shell/Extensions/LauncherStrut \
  --method org.gnome.Shell.Extensions.LauncherStrut.UpdateLauncher \
  "bottom_12345" 120

# Unregister
gdbus call --session \
  --dest org.gnome.Shell.Extensions.LauncherStrut \
  --object-path /org/gnome/Shell/Extensions/LauncherStrut \
  --method org.gnome.Shell.Extensions.LauncherStrut.UnregisterLauncher \
  "bottom_12345"

# List all instances
gdbus call --session \
  --dest org.gnome.Shell.Extensions.LauncherStrut \
  --object-path /org/gnome/Shell/Extensions/LauncherStrut \
  --method org.gnome.Shell.Extensions.LauncherStrut.GetInstances
```

### 12.3 Automated Testing

- **Extension tests:** Use `gnome-shell-extensions-tool` to validate metadata and load the extension in a headless GNOME Shell instance.
- **Rust integration tests:** Mock the D-Bus interface using `zbus` test utilities and verify that the launcher sends correct `RegisterLauncher` calls with the
  right edge/size/monitor based on config values.

---

## 13. Future Extensions

### 13.1 GSettings for Persistent State

A GSettings schema can persist the last-known instance map so that struts survive GNOME Shell restarts without waiting for Rust binaries to re-register:

```xml
<schemalist>
  <schema id="org.gnome.shell.extensions.launcher-strut">
    <key name="instances" type="aa{sv}">
      <default>[]</default>
      <summary>Active launcher instances</summary>
    </key>
  </schema>
</schemalist>
```

On `enable()`, the extension reads from GSettings and pre-creates struts. When Rust binaries re-register, the extension merges the new registrations with the
persisted state.

### 13.2 Window Re-snapping

Listen to `position-changed` on each positioned `MetaWindow` and re-snap it to its edge if the user drags it away. This prevents accidental window displacement.

### 13.3 Animations

Animate strut size changes using `St.Widget` transitions, providing a smooth workarea resize instead of an instant jump when a launcher expands or collapses.

### 13.4 Monitor Hotplug

Listen to `Main.layoutManager`'s `monitors-changed` signal and reposition all launcher windows when monitors are added, removed, or their geometry changes.

---

## 14. Summary

| Aspect                 | Decision                                                                          |
|------------------------|-----------------------------------------------------------------------------------|
| **Problem**            | GNOME/Mutter: no Layer Shell + no client-side window positioning                  |
| **Solution**           | GNOME Shell Extension as orchestrator: struts + MetaWindow positioning            |
| **Multi-instance**     | Up to 4 independent strut zones (one per edge), managed by instance ID            |
| **Window positioning** | Extension finds window by PID, calls `move_resize_frame` + `make_above` + `stick` |
| **Coupling**           | D-Bus only — no shared code between extension and Rust binary                     |
| **Rotation split**     | Extension handles geometric placement; Rust handles visual UI rotation            |
| **Extension language** | JavaScript (GNOME Shell ESM, GNOME 45+)                                           |
| **Rust changes**       | Detect GNOME, skip Layer Shell, call D-Bus to register, do NOT position window    |
| **Config**             | New optional `[gnome_strut]` section; existing `[launcher]` values reused         |
| **Fallback**           | If extension is disabled or D-Bus unavailable, log warning (no positioning)       |
| **Packaging**          | Extension + Rust binary in one distribution package                               |
