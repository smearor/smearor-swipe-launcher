# CSS Classes & Dynamic CSS Loading

## Overview

This concept describes a comprehensive CSS customization system for smearor-swipe-launcher. It introduces automatic CSS classes for instances, areas, and
widgets, user-configurable CSS classes via TOML config, dynamic loading of global and per-instance CSS files, and file watchers for hot-reload.

## Motivation

Currently, CSS styling is limited to the compiled-in `resources/style.css`. Users have no way to override styles without rebuilding. Additionally, CSS selectors
cannot reliably target specific instances, areas, or widgets because no predictable CSS classes are assigned automatically. This concept enables full user
customization at runtime.

## Current State

- **CSS loading**: `css_provider.rs` loads `resources/style.css` via `include_str!` with `STYLE_PROVIDER_PRIORITY_APPLICATION`. No user CSS is loaded.
- **Instance CSS classes**: `ApplicationWindow` gets only `transparent-background` (`window.rs:65`). No instance-identifying class.
- **Area CSS classes**: `static-area` or `scroll-area` + transition class + user `css_classes` from `AreaConfig` (`area/container/gtk.rs:48-50`). No `area-{id}`
  class.
- **Widget CSS classes**: Each plugin sets its own hardcoded classes (e.g. `scroll-item`, `menu-button` for buttons). Button has `css_classes` in config. No
  `widget-{plugin_id}` class. No generic `css_classes` field in `WidgetLayout`.
- **File watching**: `ConfigWatcher` (`config/watcher.rs`) uses `notify` crate for TOML config hot-reload. No CSS file watching exists.

## Requirements

### 1. Automatic Instance CSS Class

Every GTK instance window shall receive a CSS class `instance-{instance_id}`.

**Affected files**:

- `smearor-swipe-launcher/src/instance/launcher_instance.rs` — `build_window()` method, after `create_window()` returns the `ApplicationWindow`
- `smearor-swipe-launcher/src/window.rs` — alternatively, pass `instance_id` to `create_window()` and add the class there

**Implementation**:

- In `build_window()`, after `let window = create_window(...)`, call `window.add_css_class(&format!("instance-{}", self.instance_id))`
- The `instance_id` is already available as `self.instance_id` on `LauncherInstance`
- This class is added in addition to the existing `transparent-background` class

**CSS selector example**:

```css
.instance-bottom .menu-button {
    background-color: rgba(0, 0, 0, 0.5);
}
```

### 2. User-Configurable Instance CSS Classes

The `[launcher]` section in TOML shall accept optional `css_classes`.

**Affected files**:

- `smearor-swipe-launcher/src/config/launcher.rs` — add `css_classes: Vec<String>` field to `SwipeLauncherSettings` with `#[serde(default)]`
- `smearor-swipe-launcher/src/instance/launcher_instance.rs` — in `build_window()`, iterate over `self.config.launcher.css_classes` and call
  `window.add_css_class()` for each

**TOML example**:

```toml
[launcher]
css_classes = ["dark-theme", "compact"]
```

**Behavior**: Classes are added in addition to `transparent-background` and `instance-{id}`. No existing classes are removed.

### 3. Automatic Area CSS Class

Every area container shall receive a CSS class `area-{area_id}`.

**Affected files**:

- `smearor-swipe-launcher/src/area/container/gtk.rs` — `create_area_widget()` function
- The `area_id` is currently not passed to `create_area_widget()`. It must be added as a parameter.

**Implementation**:

- Add `area_id: &str` parameter to `AreaBackend::create_area_widget()`
- Update all callers (in `area_manager.rs` `add_area_from_config`, `rebuild_areas`) to pass `area_id`
- In `create_area_widget()`, add `format!("area-{}", area_id)` to the `css_classes` vector before building the widget
- For the headless backend, the `area_id` can be ignored (no CSS)

**CSS selector example**:

```css
.area-games .menu-button {
    border: 1px solid #dc0073;
}
```

### 4. User-Configurable Area CSS Classes

This already exists. `AreaConfig.css_classes: Vec<String>` (`model/area/src/config.rs:69`) is applied in `area/container/gtk.rs:50` via
`css_classes.extend(area_config.css_classes.iter()...)`.

**No changes needed.** Documentation should be updated to mention the new `area-{id}` class alongside the existing `css_classes` field.

### 5. Automatic Widget CSS Class

Every widget shall receive a CSS class `widget-{plugin_id}`.

**Affected files**:

- `plugin-api/src/widget/layout.rs` — add `css_classes` field to `WidgetLayout` (see requirement 6)
- All plugin crates — in `build_widget()`, add `widget-{plugin_id}` class to the root widget

**Implementation approach**:

- The `plugin_id` is available in each plugin's config or via the plugin metadata. Each plugin's `build_widget()` method should add
  `format!("widget-{}", plugin_id)` to the root widget's CSS classes.
- Alternatively, a helper function in `plugin-api` could standardize this:
  `pub fn apply_widget_css_classes(widget: &impl WidgetExt, plugin_id: &str, config_css_classes: &[String])`
- This helper would add both `widget-{plugin_id}` and any user-configured `css_classes` from `WidgetLayout`

**Affected plugins** (each needs its `build_widget()` updated):

- `plugins/button/src/widget.rs`
- `plugins/app-launcher/src/widget.rs`
- `plugins/audio/src/widget.rs`
- `plugins/clock/src/widget.rs`
- `plugins/mpris/src/widget.rs`
- `plugins/network/src/widget.rs`
- `plugins/notifications/src/widget.rs`
- `plugins/power/src/widget.rs`
- `plugins/sysinfo/src/*/widget.rs`
- `plugins/wallpaper/src/widget.rs`
- `plugins/weather/src/widget.rs`
- `plugins/workspace-switcher/src/widget.rs`

**CSS selector example**:

```css
.widget-fan .nerd-icon {
    color: #00a1e4;
}
```

### 6. User-Configurable Widget CSS Classes via WidgetLayout

`WidgetLayout` in `plugin-api` shall carry an optional `css_classes: Vec<String>` field. Since all widget configs already `#[serde(flatten)]` `WidgetLayout`,
this makes `css_classes` available in every widget's TOML config without per-plugin changes.

**Affected files**:

- `plugin-api/src/widget/layout.rs` — add `css_classes: Vec<String>` field to `WidgetLayout` with `#[serde(default)]`
- All plugin crates — in `build_widget()`, apply `self.config.layout.css_classes` to the root widget (and optionally to sub-elements like icons, similar to how
  Button already does it)

**WidgetLayout change**:

```rust
pub struct WidgetLayout {
    pub spacing: Option<i32>,
    pub css_classes: Vec<String>,
}
```

**TOML example** (works in any widget config):

```toml
[[areas.plugins]]
id = "fan"
type = "button"
css_classes = ["custom-fan-button"]
```

**Migration**: The Button plugin currently has its own `css_classes` field in `ButtonConfig`. This field will be removed and replaced by
`WidgetLayout.css_classes`. Since `WidgetLayout` is already `#[serde(flatten)]`-ed into `ButtonConfig`, the TOML field name stays `css_classes` — no config
changes needed. The bundled config files (`configs/launcher/config.toml`, `configs/launcher/example-*.toml`, `configs/launcher/streamdeck.toml`) will continue
to work without modification.

**Behavior**: Classes from `WidgetLayout.css_classes` are added in addition to built-in classes (`scroll-item`, `menu-button`, etc.) and the automatic
`widget-{plugin_id}` class. No existing classes are removed.

### 7. Global User CSS File

A CSS file at `~/.config/smearor/style.css` shall be loaded automatically if it exists.

**Affected files**:

- `smearor-swipe-launcher/src/css_provider.rs` — extend `create_css_provider()`

**Implementation**:

- After loading the built-in `resources/style.css` with `STYLE_PROVIDER_PRIORITY_APPLICATION`, check if `~/.config/smearor/style.css` exists (using
  `dirs::config_dir().join("smearor").join("style.css")`)
- If it exists, create a second `CssProvider`, load it via `load_from_file()`, and register with `STYLE_PROVIDER_PRIORITY_USER` (higher priority than
  `APPLICATION`)
- This ensures user styles override built-in defaults

**Priority order** (low to high):

1. `resources/style.css` — `STYLE_PROVIDER_PRIORITY_APPLICATION`
2. `~/.config/smearor/style.css` — `STYLE_PROVIDER_PRIORITY_USER`
3. Per-instance CSS (see requirement 9) — `STYLE_PROVIDER_PRIORITY_USER + 1`

### 8. File Watcher for Global CSS

A file watcher shall monitor `~/.config/smearor/style.css` and reload the CSS provider when the file changes.

**Affected files**:

- `smearor-swipe-launcher/src/css_provider.rs` — add file watcher logic, or create a new `css_watcher.rs` module

**Implementation**:

- Use `notify` crate (already a workspace dependency, used by `ConfigWatcher`)
- Watch the file for modifications using `notify::Watcher` with `RecursiveMode::NonRecursive`
- On change, remove the old `CssProvider` from the display and add a new one with the updated content
- Debounce events (500ms, same as `ConfigWatcher`) to avoid excessive reloads during file writes
- The watcher must run on a background thread (tokio task or std thread). GTK calls (`CssProvider::load_from_file`, `style_context_add_provider_for_display`,
  `style_context_remove_provider_for_display`) must **never** be invoked directly from the watcher thread — they are only safe on the GTK main thread. All GTK
  operations must be dispatched via `glib::MainContext::default().spawn_local()`. Violating this causes undefined behavior or crashes.
- Store the `CssProvider` in a `RefCell` or `Mutex` so it can be removed and replaced

**Considerations**:

- If the file is deleted or renamed (`Remove` or `Rename` event), remove the corresponding `CssProvider` from the display immediately. This ensures styles do
  not persist after the source file is gone.
- After removal, fall back to directory watching (see inotify limitation below) to detect re-creation.
- If the file is created after startup (didn't exist initially), start loading it.
- Log all reload, removal, and creation events at `debug` level.

**Atomic saves (editor compatibility)**: Many Linux text editors (VS Code, Vim, Gedit) do not overwrite files in-place. They write to a temporary file and swap
it via `rename()`, producing a rapid sequence of `Remove`/`Rename` followed by `Create`/`Write` events. To prevent styles from flickering or briefly unloading
during saves, the debounce logic (500ms) must not remove the `CssProvider` immediately on `Remove`/`Rename`. Instead, after the debounce interval elapses, the
watcher checks `!path.exists()` — if the file still exists (because it was already re-created by the atomic swap), it is treated as a modification and the CSS
is reloaded rather than removed. Only if the file is genuinely gone after debounce does the provider get removed.

**inotify limitation (Linux)**: `notify` cannot watch a non-existent file directly — `inotify_add_watch` fails with `ENOENT`. If `~/.config/smearor/style.css`
does not exist at startup, the watcher must instead watch the **parent directory** (`~/.config/smearor/`) with `RecursiveMode::NonRecursive` and listen for
`EventKind::Create` events matching the target filename. Once the file is created, the CSS is loaded immediately, and the watcher switches to direct file
watching for subsequent modifications. If the file is deleted later, the watcher falls back to directory watching to detect re-creation. The same pattern
applies to per-instance CSS files (requirement 10).

**Directory watch deduplication**: The CSS watcher must maintain a set of currently-watched directories. Before adding a new directory watch (for the
parent-directory fallback), it must check whether that directory is already being watched — either for the global CSS or for another per-instance CSS file. If
already watched, no new `inotify_add_watch` call is made; instead, the expected filename is added to a multi-map of `directory -> Set<expected_filenames>`. When
a `Create` event arrives, the watcher checks which expected filenames in that directory match and triggers the appropriate CSS load. This prevents redundant
watches when multiple instance CSS files reside in the same directory (e.g., `~/.config/smearor/launcher/`).

### 9. Per-Instance CSS File

When a launcher instance is loaded from a TOML config file, a CSS file with the same stem but `.css` extension shall be loaded if it exists.

**File resolution**:

- If config is loaded from `~/.config/smearor/launcher/my-launcher.toml`, look for `~/.config/smearor/launcher/my-launcher.css`
- If config is loaded from working directory `./config.toml`, look for `./config.css`
- If config is loaded from `/usr/share/smearor/launcher/config.toml`, look for `/usr/share/smearor/launcher/config.css`
- General rule: replace `.toml` extension with `.css` in the same directory

**Affected files**:

- `smearor-swipe-launcher/src/main.rs` — after loading each config file, check for corresponding CSS
- `smearor-swipe-launcher/src/host/mod.rs` — `load_instance()` method for dynamically loaded instances
- `smearor-swipe-launcher/src/css_provider.rs` — new function `create_instance_css_provider(instance_id: &str, css_path: &Path)`

**Implementation**:

- In `main.rs`, after loading each config file (line ~97-117), compute the CSS path by replacing the extension
- Pass the CSS path (if it exists) to a new function that creates a per-instance `CssProvider`
- The per-instance provider is registered with `STYLE_PROVIDER_PRIORITY_USER + 1` to override both built-in and global user CSS
- For dynamically loaded instances (`load_instance()` in `host/mod.rs`), perform the same CSS path resolution

**Priority order** (per display):

1. Built-in `resources/style.css` — `APPLICATION`
2. `~/.config/smearor/style.css` — `USER`
3. `~/.config/smearor/launcher/{instance}.css` — `USER + 1`

**Multi-Provider Spillover (high risk)**: GTK4 registers `CssProvider` instances via `gtk_style_context_add_provider_for_display` at the `GdkDisplay` level, not
per-window. A per-instance CSS file loaded with `STYLE_PROVIDER_PRIORITY_USER + 1` applies to **all** windows on the same display, not just the instance it was
loaded for. Unscoped selectors (e.g. `button { background: red; }`) will leak into other instances.

**Mitigation**:

- The `design-css.md` documentation must explicitly state that per-instance CSS files do **not** have an isolated scope.
- Authors of per-instance CSS files must prefix all selectors with the instance class (e.g. `.instance-my-id .menu-button { ... }`).
- Unscoped selectors in per-instance CSS should be considered a user error. A debug-level log warning could be emitted if a per-instance CSS file contains
  selectors without an `.instance-*` prefix, but this is optional (GTK CSS parsing does not expose selector structure).

### 10. File Watcher for Per-Instance CSS

Each per-instance CSS file shall be monitored for changes and hot-reloaded.

**Affected files**:

- `smearor-swipe-launcher/src/css_provider.rs` or new `css_watcher.rs` module

**Implementation**:

- Extend the CSS file watcher from requirement 8 to also watch per-instance CSS files
- Maintain a map of `instance_id -> (css_path, CssProvider)` for all loaded instance CSS files
- On file change, remove the old provider and add the new one with `STYLE_PROVIDER_PRIORITY_USER + 1`
- Same debouncing and main-thread dispatch rules as requirement 8, including the atomic-save `!path.exists()` check and the GTK thread-safety constraint (all
  GTK calls via `glib::MainContext::default().spawn_local()`, never from the watcher thread)
- When an instance is unloaded, remove its CSS provider and stop watching its file
- **inotify fallback**: If a per-instance CSS file does not exist at load time, watch its parent directory (the same directory as the `.toml` config) for
  `Create` events matching the expected `.css` filename. Once created, switch to direct file watching. This mirrors the strategy from requirement 8.
- **Directory watch deduplication**: Multiple per-instance CSS files often share the same parent directory (e.g., all configs in `~/.config/smearor/launcher/`).
  The watcher must deduplicate directory watches — see the deduplication strategy described in requirement 8. A `HashMap<PathBuf, HashSet<String>>` (directory →
  expected filenames) tracks all pending CSS files per directory. A single directory watch covers all pending files within it.

### 11. Book Documentation Update

The book documentation shall be updated to reflect all CSS customization features.

**Affected files**:

- `book/src/configuration/design-css.md` — major update
- `book/src/configuration/launcher-config.md` — mention `css_classes` in `[launcher]` section
- `book/src/configuration/area-config.md` — mention automatic `area-{id}` class
- `book/src/plugin-api/widget-plugin.md` — mention `WidgetLayout.css_classes` and automatic `widget-{id}` class
- `book/src/SUMMARY.md` — no new entry needed (design-css.md already listed)

**Content for `design-css.md`**:

- Document the CSS loading order (built-in > global user > per-instance)
- Document automatic CSS classes: `instance-{id}`, `area-{id}`, `widget-{plugin_id}`
- Document user-configurable `css_classes` at instance, area, and widget levels
- Document `~/.config/smearor/style.css` global override
- Document per-instance CSS file convention (`{config_stem}.css` alongside `{config_stem}.toml`)
- Document hot-reload behavior
- Add CSS selector examples for each level

## CSS Class Name Sanitization

Instance IDs, area IDs, and plugin IDs can contain characters that are invalid in CSS class names (spaces, dots, special characters). A central helper function
must sanitize all generated CSS class names before passing them to `add_css_class()`.

**Affected files**:

- `plugin-api/src/widget/layout.rs` or a new `plugin-api/src/css.rs` — central helper function

**Implementation**:

- Add a public function `sanitize_css_class_name(input: &str) -> String` that:
    1. Replaces every character outside `[a-zA-Z0-9_-]` with `-`
    2. Collapses multiple consecutive `-` into a single `-`
    3. Returns the resulting string
- All automatic CSS class generation (`instance-{id}`, `area-{id}`, `widget-{plugin_id}`) must apply this function to the ID portion before constructing the
  class name
- Example: `sanitize_css_class_name("my.widget v2")` → `"my-widget-v2"`

**Usage**:

- `format!("instance-{}", sanitize_css_class_name(&self.instance_id))`
- `format!("area-{}", sanitize_css_class_name(area_id))`
- `format!("widget-{}", sanitize_css_class_name(plugin_id))`

**User-configured `css_classes`**: User-provided CSS classes from TOML config (`[launcher].css_classes`, `AreaConfig.css_classes`, `WidgetLayout.css_classes`)
are **not** sanitized — these are intentional user input and passed verbatim to `add_css_class()`. If a user writes an invalid CSS class name, GTK4 silently
ignores it.

## Implementation Order

### Phase 1: Automatic CSS Classes (Requirements 1, 3, 5)

- Add `sanitize_css_class_name()` helper in `plugin-api`
- Add `instance-{id}` to windows in `build_window()`
- Add `area_id` parameter to `create_area_widget()` and add `area-{id}` class
- Add `widget-{plugin_id}` class in each plugin's `build_widget()`
- All automatic classes use `sanitize_css_class_name()` on the ID portion
- **Dependencies**: None
- **Exit criteria**: Every window, area, and widget has a predictable, sanitized CSS class

### Phase 2: User-Configurable CSS Classes (Requirements 2, 6)

- Add `css_classes` to `SwipeLauncherSettings`
- Add `css_classes` to `WidgetLayout`
- Apply user classes in `build_window()` and each plugin's `build_widget()`
- Migrate Button's `css_classes` from `ButtonConfig` to `WidgetLayout`
- **Dependencies**: Phase 1
- **Exit criteria**: Users can set `css_classes` at all three levels via TOML

### Phase 3: Global CSS Loading (Requirement 7)

- Extend `create_css_provider()` to load `~/.config/smearor/style.css`
- **Dependencies**: None (independent of Phases 1-2)
- **Exit criteria**: Global user CSS is loaded at startup if it exists

### Phase 4: Per-Instance CSS Loading (Requirement 9)

- Add `create_instance_css_provider()` function
- Call it from `main.rs` and `load_instance()` with the resolved CSS path
- **Dependencies**: Phase 3
- **Exit criteria**: Per-instance CSS files are loaded at startup and on dynamic instance load

### Phase 5: File Watchers (Requirements 8, 10)

- Create CSS file watcher module (or extend `css_provider.rs`)
- Watch global CSS and all per-instance CSS files
- Hot-reload on change with debouncing
- **Dependencies**: Phases 3, 4
- **Exit criteria**: CSS changes are reflected at runtime without restart

### Phase 6: Documentation (Requirement 11)

- Update `design-css.md` with full CSS customization guide
- Update `launcher-config.md`, `area-config.md`, `widget-plugin.md`
- **Dependencies**: Phases 1-5
- **Exit criteria**: All CSS features are documented in the book

## CSS Priority Summary

| Priority Level | Source                                      | GTK Constant                                |
|----------------|---------------------------------------------|---------------------------------------------|
| Lowest         | `resources/style.css` (built-in)            | `STYLE_PROVIDER_PRIORITY_APPLICATION` (600) |
| Medium         | `~/.config/smearor/style.css` (global user) | `STYLE_PROVIDER_PRIORITY_USER` (800)        |
| Highest        | `{config_stem}.css` (per-instance)          | `STYLE_PROVIDER_PRIORITY_USER + 1` (801)    |

## CSS Class Summary

| Level    | Automatic Class          | User-Configurable | Config Field                       |
|----------|--------------------------|-------------------|------------------------------------|
| Instance | `instance-{instance_id}` | Yes               | `[launcher].css_classes`           |
| Area     | `area-{area_id}`         | Yes (existing)    | `[[areas]].css_classes`            |
| Widget   | `widget-{plugin_id}`     | Yes               | `css_classes` (via `WidgetLayout`) |

## Graceful Shutdown

The CSS file watchers and background tasks must not prevent the application from shutting down cleanly. There are multiple exit paths that must be handled:

### Full Application Shutdown

Triggered by window close (`close_request` → `app.quit()`) or SIGINT/Ctrl-C (`shutdown_flag` → `gtk_app.quit()`). In both cases, `host.run()` returns and
`main.rs` performs explicit cleanup (MCP server stop, service unload, `std::process::exit(0)`).

The CSS watcher shutdown must be integrated into the post-`host.run()` cleanup block in `main.rs` (lines ~231-256), **before** `std::process::exit(0)`:

1. **`host.run()` returns**: GTK main loop has ended. No new `spawn_local` callbacks will be scheduled.
2. **MCP server stopped** (existing): `server.stop()`.
3. **CSS watchers stopped**: Call `css_watcher.shutdown()` — stops all `notify` file and directory watches, aborts the debounce task (`CancellationToken` or
   `JoinHandle::abort()`), releases file handles.
4. **Display providers cleaned up**: Remove all `CssProvider` instances (global + per-instance) from the `GdkDisplay` via
   `style_context_remove_provider_for_display`. Must run on the GTK main thread — since the main loop has already stopped, this must be done via a direct call
   in the cleanup sequence (not `spawn_local`, which would not execute after `quit()`). Alternatively, providers can be removed before `app.quit()` is called.
5. **Services unloaded** (existing): `host.service_manager.unload_services()`.
6. **Process exit** (existing): `std::process::exit(0)`.

**Affected files**:

- `smearor-swipe-launcher/src/main.rs` — add `css_watcher.shutdown()` call in the post-`host.run()` cleanup block
- `smearor-swipe-launcher/src/css_provider.rs` or `css_watcher.rs` — implement `shutdown()` method

### Per-Instance Teardown

Triggered by MCP `StopInstance` command, MCP `ReloadInstance` (stop + reload), or MacroPad disconnect. Calls `host.stop_instance()` (`host/mod.rs:1985`),
which removes the instance from the map and closes its window.

The per-instance CSS provider and its file watch must be cleaned up in `stop_instance()`:

1. **Instance removed from map** (existing): `instances.remove(instance_id)`.
2. **Window closed** (existing): `window.close()`.
3. **Per-instance CSS provider removed**: `style_context_remove_provider_for_display` for the instance's `CssProvider`.
4. **Per-instance file watch stopped**: `Watcher::unwatch()` for the instance's CSS file, or removal from the directory-watch multi-map if the file doesn't
   exist yet.

**Affected files**:

- `smearor-swipe-launcher/src/host/mod.rs` — extend `stop_instance()` to call CSS watcher cleanup for the instance
- `smearor-swipe-launcher/src/css_provider.rs` or `css_watcher.rs` — implement `remove_instance_css(instance_id: &str)` method

### Considerations

- The `LauncherHost` must hold a reference to the `CssWatcher` (new field `css_watcher: Mutex<Option<CssWatcher>>`) so both full-shutdown and per-instance
  teardown paths can access it.
- Provider removal from the display is a GTK call and must happen on the main thread. For per-instance teardown (`stop_instance`), this is already the case
  since MCP commands are dispatched on the main thread. For full-app shutdown, the cleanup block in `main.rs` runs on the main thread after the GTK loop ends —
  direct GTK calls are safe there as long as the `GdkDisplay` is still valid.
- The debounce task (tokio task) must be aborted via `CancellationToken` or `JoinHandle::abort()` — not just dropped, since tokio tasks run on the runtime
  independently.
- Dropping the `notify::Watcher` automatically unregisters all kernel-level watches, but explicit `unwatch()` calls allow finer-grained cleanup and earlier
  resource release.
- The existing `ConfigWatcher` (`config/watcher.rs`) has the same lifecycle gap — its tokio debounce loop runs indefinitely with no `shutdown()` method or
  `CancellationToken`. Analysis of all exit paths:

  | Exit Path | ConfigWatcher Cleanup? | Problem |
      |---|---|---|
  | Window close → `app.quit()` | No | Watcher-Task läuft bis `process::exit(0)` |
  | SIGINT/Ctrl-C → `app.quit()` | No | Same |
  | MCP `StopInstance` | No | Config-Dateien der gestoppten Instance bleiben gewatched — Reload-Requests für nicht-existierende Instance erzeugen Log-Noise und CPU-Last |
  | MCP `ReloadInstance` | No | `stop_instance` + `load_instance` — Watcher wird nicht aktualisiert, funktioniert nur zufällig weil Config-Pfad gleich bleibt |
  | MacroPad disconnect → `stop_instance` | No | Same as MCP `StopInstance` |
  | Post-`host.run()` cleanup | No | `std::process::exit(0)` beendet Task hart — 500ms-Grace-Period ist Workaround, kein richtiger Shutdown |

  **Konkrete Probleme**:
    1. `stop_instance()` entfernt keine Config-Dateien aus dem Watcher — Reload-Requests für gestoppte Instanzen schlagen fehl ("Instance not found")
    2. Kein `unwatch()` für gestoppte Instanzen — bei MacroPad connect/disconnect-Zyklen akkumulieren sich stale File-Watches
    3. Kein `CancellationToken` oder `JoinHandle` für den Debounce-Task — Task wird bei `process::exit(0)` hart abgebrochen

  **Empfehlung**: `ConfigWatcher` und `CssWatcher` sollten in einen gemeinsamen Shutdown-Pfad integriert werden. Dafür benötigt `ConfigWatcher`:
    - Eine `shutdown()`-Methode, die alle Watches entfernt und den Debounce-Task abbricht
    - Eine `remove_instance(instance_id: &str)`-Methode, die Config-Dateien einer Instanz aus dem Watcher entfernt (aufzurufen in `stop_instance()`)
    - Ein `CancellationToken` oder `JoinHandle` anstelle des nackten `tokio::spawn`
    - Aufruf von `config_watcher.shutdown()` im Post-`host.run()` Cleanup-Block in `main.rs`

## Risks and Considerations

- **`create_area_widget` signature change**: Adding `area_id` parameter changes a trait method signature. All implementations and callers must be updated
  simultaneously.
- **Button `css_classes` migration**: `ButtonConfig.css_classes` is removed and replaced by `WidgetLayout.css_classes`. Since `WidgetLayout` is already
  `#[serde(flatten)]`-ed into `ButtonConfig`, the TOML field name `css_classes` stays the same — no config file changes needed.
- **Multi-Provider Spillover (high risk)**: GTK4 registers `CssProvider` at the `GdkDisplay` level. Per-instance CSS loaded with `USER + 1` priority affects
  **all** windows on the same display, not just the target instance. Unscoped selectors leak into other instances. Mitigation: documentation must mandate
  `.instance-{id}`-prefixed selectors in per-instance CSS files. The `instance-{id}` CSS class (requirement 1) is the only scoping mechanism.
- **File watcher lifecycle**: Watchers must be properly cleaned up when instances are unloaded to avoid resource leaks.
- **Headless/Web instances**: CSS is not applicable to headless or web instances. Per-instance CSS loading should be skipped for `InstanceType::Headless` and
  `InstanceType::Web`.
- **`WidgetLayout` `#[serde(default)]`**: Adding `css_classes` with `#[serde(default)]` is backward-compatible — existing configs without the field will default
  to an empty vector.
