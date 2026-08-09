# Concept: Top-Level Config Includes

This document describes the concept for **top-level `includes`** in launcher instance configuration files — a mechanism to deduplicate widget and area
configurations that are shared across multiple launcher instance config files.

---

## 1. Motivation

The launcher supports multiple instances, each defined by its own TOML config file (e.g. `config.toml`, `example-bottom.toml`, `example-left.toml`). Currently,
plugin configs and area definitions that appear in multiple instance configs must be **fully duplicated** in each file. For example, `open_top_area_button` is
identically defined in `example-bottom.toml`, `example-left.toml`, and other example configs.

The existing deduplication mechanisms are:

- **`[defaults.*]` templates**: Plugin configs can reference a default template via `defaults = "menu_button"`. This works *within a single config file* —
  `resolve_defaults()` merges template values with instance-specific values. However, the templates themselves are defined per-file and cannot be shared across
  instances.
- **Per-area `include` directive**: An area can include an external TOML file (`include = "../areas/scroll_menu.toml"`) for area-level fields and plugin
  configs. This is scoped to a single area and cannot share defaults, launcher settings, or multiple areas at once.

**The gap**: There is no way to share plugin configs, area definitions, or default templates *across* instance config files. The top-level `includes` mechanism
fills this gap by allowing an instance config to pull in shared TOML fragments as a **base layer** before applying its own overrides.

---

## 2. Crate Structure

This feature does **not** introduce new crates. It is an internal enhancement to the config loading pipeline in the `smearor-swipe-launcher` crate.

| Component              | Path                                                    | Responsibility                                                                              |
|------------------------|---------------------------------------------------------|---------------------------------------------------------------------------------------------|
| **Config struct**      | `smearor-swipe-launcher/src/config/launcher.rs`         | `SwipeLauncherConfig` — add `includes` field, `resolve_top_level_includes()` method         |
| **Config error**       | `smearor-swipe-launcher/src/config/error.rs`            | `ConfigValidationError` — add include-related error variants                                |
| **Config watcher**     | `smearor-swipe-launcher/src/config/watcher.rs`          | `ConfigWatcher` — watch top-level include files for hot-reload                              |
| **Config loading**     | `smearor-swipe-launcher/src/args/launcher.rs`           | `load_config_from_file()` — call `resolve_top_level_includes()` before `resolve_defaults()` |
| **Instance lifecycle** | `smearor-swipe-launcher/src/host/instance_lifecycle.rs` | `load_instance()` — call `resolve_top_level_includes()` before `resolve_defaults()`         |

No model, service, or widget crates are affected. No FFI types are introduced. No message system changes are needed.

---

## 3. Design

### 3.1 Top-Level `includes` Field

Add a new optional field to `SwipeLauncherConfig`:

```rust
/// Top-level include files to merge as a base layer before the main config.
///
/// Each path is resolved relative to the config file's directory.
/// Include files are TOML files containing any combination of:
/// - `[defaults.*]` templates
/// - Plugin configs (top-level `[plugin_id]` tables)
/// - Area configs (top-level `[area_id]` tables with `area_type` or `plugins`)
///
/// Merge order: first include = deepest base, last include = higher base,
/// main config = highest priority (overrides all includes).
/// Entries are merged by key — if the same ID appears in an include and the
/// main config, the main config wins.
#[serde(default)]
pub includes: Vec<String>,
```

### 3.2 Include File Format

An include file is a standard TOML file with the same structure as a launcher config, minus the `areas`, `launcher`, `layout`, `profiles`, and `includes`
top-level fields. It contains:

- `[defaults.<name>]` tables — default templates for plugin configs
- `[<plugin_id>]` tables — plugin configurations
- `[<area_id>]` tables — area configurations (recognized by `area_type` or `plugins` key)

```toml
# shared/defaults.toml — shared default templates

[defaults.menu_button]
click_topic = "area.open"
longpress_topic = "area.close"
enabled = true
active = false
css_classes = ["menu-button", "glow-blue"]

[defaults.close_button]
main_text = ""
info_text = "Zurück"
icon = "nf-md-undo"
icon_color = "#dc0073ff"
click_topic = "area.close"
enabled = true
active = false
css_classes = ["close-button"]

[defaults.app_launcher]
```

```toml
# shared/buttons/navigation.toml — shared navigation buttons

[open_top_area_button]
defaults = "menu_button"
main_text = "Open Top"
icon = "panel-top-symbolic"
icon_only = true
click_topic = "area.open"
click_instance = "top"
click_payload = { area_id = "top_area" }
longpress_topic = "area.close"
longpress_instance = "top"
longpress_payload = { area_id = "top_area" }
enabled = true
active = false
css_classes = ["menu-button", "primary"]

[open_bottom_area_button]
defaults = "menu_button"
main_text = "Open Bottom"
icon = "panel-bottom-symbolic"
icon_only = true
click_topic = "area.open"
click_instance = "bottom"
click_payload = { area_id = "bottom_area" }
longpress_topic = "area.close"
longpress_instance = "bottom"
longpress_payload = { area_id = "bottom_area" }
enabled = true
active = false
css_classes = ["menu-button", "primary"]
```

```toml
# shared/areas/office_room.toml — complete area with plugins

[office_area]
area_type = "scroll"
open_transition = "SlideUp"
plugins = [
    { id = "office_area_close_button", path = "target/release/libsmearor_button_widget.so" },
    { id = "office_light_button", path = "target/release/libsmearor_button_widget.so" },
]

[office_area_close_button]
defaults = "close_button"
click_payload = { area_id = "office_area" }

[office_light_button]
defaults = "menu_button"
main_text = "Light"
icon = "nf-md-lightbulb"
click_topic = "homeassistant.toggle"
click_payload = { entity_id = "light.office" }
```

### 3.3 Merge Semantics

The merge follows a **layered base** strategy:

```
[include_1.toml]  ← deepest base
[include_2.toml]
[include_3.toml]
[main_config.toml] ← highest priority
```

**Rules:**

1. **Entries (`HashMap<String, ConfigEntry>`)**: If the same key (area ID or plugin ID) appears in multiple layers, the **highest-priority layer wins**
   entirely. There is no field-level merge for individual entries — the entire `ConfigEntry` (area or plugin) is replaced. This matches the existing
   `resolve_includes` behavior where main config entries override include entries.

2. **Defaults (`HashMap<String, Value>`)**: If the same default template name appears in multiple layers, the **highest-priority layer wins**. This allows an
   instance to override a shared default template with a custom one.

3. **Top-level fields (`areas`, `launcher`, `layout`, `profiles`)**: These are **not merged** from include files. Only the main config defines them. Include
   files that contain these fields have them silently ignored (with a `debug!` log).

4. **Nested `includes`**: Include files may themselves declare `includes`. These are resolved recursively, with cycle detection via a visited-path set. A cycle
   produces a `ConfigValidationError::IncludeCycle` error.

5. **Per-area `include` directive**: The existing per-area `include` mechanism in `AreaConfig` continues to work independently. Top-level includes are resolved
   first, then per-area includes are resolved within the merged config. This preserves backward compatibility.

### 3.4 Resolution Order

The full config resolution pipeline becomes:

```
1. Parse main config TOML → SwipeLauncherConfig
2. resolve_top_level_includes()  ← NEW: merge top-level include files
3. resolve_includes()            ← EXISTING: merge per-area include files
4. resolve_defaults()            ← EXISTING: apply [defaults.*] templates to plugin configs
5. validate()                    ← EXISTING: validate areas, plugins, profiles
```

This ordering ensures:

- Top-level includes provide base entries that per-area includes can reference
- Defaults templates from includes are available when `resolve_defaults()` runs
- Validation sees the fully merged config

### 3.5 Path Resolution

Include paths are resolved **relative to the directory of the referencing config file** (the main config or the include file that declares the `includes`
field). This matches the existing per-area `include` behavior.

```toml
# configs/launcher/config.toml
includes = ["../shared/defaults.toml", "../shared/buttons/navigation.toml"]
```

Paths are resolved relative to `configs/launcher/`, so `../shared/defaults.toml` resolves to `configs/shared/defaults.toml`.

### 3.6 Hot-Reload Integration

The `ConfigWatcher` must watch top-level include files for changes, just as it watches per-area include files today. The `collect_include_paths` method reads
`self.includes` — which is **flattened** by `resolve_top_level_includes()` to contain all recursively discovered include paths (direct and transitive). This
ensures that changes to a nested include file (e.g. `inc_b.toml` included by `inc_a.toml`) also trigger a hot-reload.

```rust
pub fn collect_include_paths(&self, base_path: &std::path::Path) -> Vec<std::path::PathBuf> {
    let base_dir = base_path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let mut paths: Vec<std::path::PathBuf> = self
        .includes
        .iter()
        .map(|inc| base_dir.join(inc))
        .collect();
    paths.extend(self.entries.values().filter_map(|entry| match entry {
        ConfigEntry::Area(area) => area.include.as_ref().map(|inc| base_dir.join(inc)),
        ConfigEntry::Plugin(_) => None,
    }));
    paths
}
```

When any watched include file changes, the entire instance is reloaded (same behavior as today).

---

## 4. Config Integration

### 4.1 Main Config (using top-level includes)

```toml
# configs/launcher/config.toml

includes = [
    "../shared/defaults.toml",
    "../shared/buttons/navigation.toml",
    "../shared/areas/office_room.toml",
    "../shared/areas/livingroom_room.toml",
]

areas = ["scroll_band", "office_area", "livingroom_area"]

[launcher]
layer = "top"
namespace = "smearor-swipe-launcher"
exclusive_zone = 105
max_width = 1080
show_decorations = false

[layout]
orientation = "horizontal"
spacing = 0

# Only instance-specific overrides — shared button configs come from includes

[scroll_band]
area_type = "scroll"
plugins = [
    { id = "open_top_area_button", path = "target/release/libsmearor_button_widget.so" },
    { id = "open_bottom_area_button", path = "target/release/libsmearor_button_widget.so" },
    { id = "office_menu_button", path = "target/release/libsmearor_button_widget.so" },
]

# Override a shared button for this instance
[open_top_area_button]
defaults = "menu_button"
main_text = "Open Top (Custom)"
```

### 4.2 Shared Defaults File

```toml
# configs/shared/defaults.toml

[defaults.menu_button]
click_topic = "area.open"
longpress_topic = "area.close"
enabled = true
active = false
css_classes = ["menu-button", "glow-blue"]

[defaults.close_button]
main_text = ""
info_text = "Zurück"
icon = "nf-md-undo"
icon_color = "#dc0073ff"
click_topic = "area.close"
enabled = true
active = false
css_classes = ["close-button"]

[defaults.app_launcher]
```

### 4.3 Shared Buttons File (per room)

```toml
# configs/shared/buttons/office.toml

[office_light_button]
defaults = "menu_button"
main_text = "Light"
icon = "nf-md-lightbulb"
click_topic = "homeassistant.toggle"
click_payload = { entity_id = "light.office" }

[office_blinds_button]
defaults = "menu_button"
main_text = "Blinds"
icon = "nf-md-blinds"
click_topic = "homeassistant.toggle"
click_payload = { entity_id = "cover.office_blinds" }
```

### 4.4 Complete Area File

```toml
# configs/shared/areas/office_room.toml

[office_area]
area_type = "scroll"
open_transition = "SlideUp"
plugins = [
    { id = "office_area_close_button", path = "target/release/libsmearor_button_widget.so" },
    { id = "office_light_button", path = "target/release/libsmearor_button_widget.so" },
    { id = "office_blinds_button", path = "target/release/libsmearor_button_widget.so" },
]

[office_area_close_button]
defaults = "close_button"
click_payload = { area_id = "office_area" }

[office_light_button]
defaults = "menu_button"
main_text = "Light"
icon = "nf-md-lightbulb"
click_topic = "homeassistant.toggle"
click_payload = { entity_id = "light.office" }

[office_blinds_button]
defaults = "menu_button"
main_text = "Blinds"
icon = "nf-md-blinds"
click_topic = "homeassistant.toggle"
click_payload = { entity_id = "cover.office_blinds" }
```

### 4.5 Multiple Instance Configs Sharing the Same Includes

```toml
# configs/launcher/config.toml (main instance, rotation 0)
includes = ["../shared/defaults.toml", "../shared/buttons/navigation.toml"]
areas = ["scroll_band"]
[launcher]
rotation = 0
```

```toml
# configs/launcher/side3.toml (side instance, rotation 90)
includes = ["../shared/defaults.toml", "../shared/buttons/navigation.toml"]
areas = ["scroll_band"]
[launcher]
rotation = 90
```

Both instances share the same button definitions and default templates. Only the launcher-specific settings (rotation, layer, areas list) differ.

---

## 5. Implementation

### 5.1 `SwipeLauncherConfig` Changes

Add the `includes` field to `SwipeLauncherConfig`:

```rust
/// Top-level include files to merge as a base layer.
///
/// Paths are resolved relative to the config file's directory.
/// Merge order: first include = deepest base, last include = higher base,
/// main config = highest priority.
#[serde(default)]
pub includes: Vec<String>,
```

### 5.2 `resolve_top_level_includes()` Method

New method on `SwipeLauncherConfig`:

```rust
/// Resolve top-level `includes` by loading each include file and merging
/// its `defaults` and `entries` as a base layer beneath the main config.
///
/// Include files are loaded in declaration order. Earlier includes form
/// deeper base layers; later includes and the main config override them.
/// The main config always wins on key conflicts.
///
/// Include files may themselves declare `includes` (recursive includes).
/// Cycles are detected and reported as `ConfigValidationError::IncludeCycle`.
pub fn resolve_top_level_includes(
    &mut self,
    base_path: &std::path::Path,
) -> Result<(), ConfigValidationError> {
    if self.includes.is_empty() {
        return Ok(());
    }

    let mut visited: std::collections::HashSet<std::path::PathBuf> = std::collections::HashSet::new();
    let canonical_base = std::fs::canonicalize(base_path).unwrap_or_else(|_| base_path.to_path_buf());
    visited.insert(canonical_base);

    let base_dir = base_path.parent().unwrap_or_else(|| std::path::Path::new("."));

    // Preserve the original direct includes; we'll append transitive ones
    // discovered during recursion so that collect_include_paths() can watch
    // the full dependency tree for hot-reload.
    let direct_includes: Vec<String> = self.includes.clone();
    let mut all_include_paths: Vec<String> = Vec::new();

    // Collect all include configs in declaration order.
    let mut include_configs: Vec<SwipeLauncherConfig> = Vec::new();
    for include_path in &direct_includes {
        let full_path = base_dir.join(include_path);
        let canonical = std::fs::canonicalize(&full_path).map_err(|e| ConfigValidationError::TopLevelIncludeNotFound {
            path: include_path.clone(),
            reason: e.to_string(),
        })?;

        if !visited.insert(canonical.clone()) {
            return Err(ConfigValidationError::IncludeCycle {
                path: canonical.to_string_lossy().to_string(),
            });
        }

        let content = std::fs::read_to_string(&full_path).map_err(|e| ConfigValidationError::TopLevelIncludeNotFound {
            path: include_path.clone(),
            reason: e.to_string(),
        })?;

        let mut include_config: SwipeLauncherConfig = toml::from_str(&content).map_err(|e| ConfigValidationError::InvalidTopLevelInclude {
            path: include_path.clone(),
            reason: e.to_string(),
        })?;

        // Warn about ignored top-level structural fields (rule 3, section 3.3).
        // Include files may only contribute defaults, entries, and nested includes.
        if !include_config.areas.is_empty()
            || include_config.launcher != SwipeLauncherSettings::default()
            || include_config.layout != LayoutConfig::default()
            || !include_config.profiles.is_empty()
        {
            tracing::debug!(
                path = %include_path,
                "Top-level structural fields (areas, launcher, layout, profiles) in include file are ignored"
            );
        }

        // Record this direct include path (relative to base_dir).
        all_include_paths.push(include_path.clone());

        // Recursively resolve nested includes. The helper appends any
        // transitive include paths (relative to the include file's own dir)
        // to include_config.includes, which we collect below.
        include_config.resolve_top_level_includes_with_visited(&full_path, &mut visited)?;

        // Collect transitive include paths discovered during recursion.
        // These are stored relative to the include file's directory, so we
        // re-relativize them to base_dir for consistency.
        let include_dir = full_path.parent().unwrap_or_else(|| std::path::Path::new("."));
        for transitive in &include_config.includes {
            if !direct_includes.contains(transitive) {
                // Store the path relative to base_dir so collect_include_paths
                // resolves it correctly from the main config's perspective.
                let transitive_abs = include_dir.join(transitive);
                let transitive_rel = pathdiff::diff_paths(&transitive_abs, base_dir)
                    .unwrap_or(transitive_abs)
                    .to_string_lossy()
                    .to_string();
                all_include_paths.push(transitive_rel);
            }
        }

        include_configs.push(include_config);
    }

    // Merge in reverse declaration order: the last include has the highest
    // priority among includes, so it is inserted first. Earlier includes
    // only fill gaps via or_insert. The main config (already in self)
    // always wins because its entries are present before any include is merged.
    for include_config in include_configs.iter().rev() {
        // Merge defaults: only insert if not already present (main config or
        // a higher-priority include has already set the value).
        for (key, value) in &include_config.defaults {
            self.defaults.entry(key.clone()).or_insert(value.clone());
        }

        // Merge entries: only insert if not already present.
        for (key, entry) in &include_config.entries {
            self.entries.entry(key.clone()).or_insert(entry.clone());
        }
    }

    // Flatten self.includes to include all transitive paths so that
    // collect_include_paths() returns the full dependency tree for the
    // ConfigWatcher. Deduplicate while preserving order.
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    self.includes = all_include_paths
        .into_iter()
        .filter(|p| seen.insert(p.clone()))
        .collect();

    Ok(())
}
```

**Note**: The recursive call uses a private helper `resolve_top_level_includes_with_visited` that accepts a mutable `visited` set for cycle detection. The
public method initializes the set with the main config path. After resolution, `self.includes` is **flattened** to contain all direct and transitive include
paths (relative to the main config's directory), enabling `collect_include_paths()` to watch the full dependency tree.

### 5.3 `ConfigValidationError` Additions

```rust
/// Failed to load a top-level include file.
#[error("Failed to load top-level include '{path}': {reason}")]
TopLevelIncludeNotFound { path: String, reason: String },

/// Failed to parse a top-level include file.
#[error("Failed to parse top-level include '{path}': {reason}")]
InvalidTopLevelInclude { path: String, reason: String },

/// Circular include detected — include file '{path}' is already in the include chain.
#[error("Circular include detected: '{path}' is already in the include chain")]
IncludeCycle { path: String },
```

### 5.4 `collect_include_paths()` Extension

Extend the existing method to also return top-level include paths. Since `resolve_top_level_includes()` flattens `self.includes` to contain all direct and
transitive include paths (see section 5.2), this method automatically covers the full dependency tree:

```rust
pub fn collect_include_paths(&self, base_path: &std::path::Path) -> Vec<std::path::PathBuf> {
    let base_dir = base_path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let mut paths: Vec<std::path::PathBuf> = self
        .includes
        .iter()
        .map(|inc| base_dir.join(inc))
        .collect();
    paths.extend(
        self.entries
            .values()
            .filter_map(|entry| match entry {
                ConfigEntry::Area(area) => area.include.as_ref().map(|inc| base_dir.join(inc)),
                ConfigEntry::Plugin(_) => None,
            }),
    );
    paths
}
```

**Important**: `collect_include_paths()` must be called **after** `resolve_top_level_includes()` has run, so that `self.includes` contains the flattened
transitive paths. The call sites in section 5.5 already ensure this ordering.

### 5.5 Call Sites

#### `load_config_from_file()` (args/launcher.rs)

Add `resolve_top_level_includes()` before `resolve_includes()` and `resolve_defaults()`. The order must match the pipeline in section 3.4 — `resolve_defaults()`
runs last so that plugin configs loaded by per-area includes also get template expansion:

```rust
pub fn load_config_from_file(&self, config_path: &PathBuf) -> Result<SwipeLauncherConfig> {
    let config_content = std::fs::read_to_string(config_path).into_diagnostic()?;
    let mut config: SwipeLauncherConfig = toml::from_str(&config_content).into_diagnostic()?;

    // 1. Resolve top-level includes (shared config fragments from external files)
    config.resolve_top_level_includes(config_path).map_err(|e| miette::miette!("{e}"))?;

    // 2. Resolve per-area includes (external TOML files referenced by area configs)
    config.resolve_includes(config_path).map_err(|e| miette::miette!("{e}"))?;

    // 3. Resolve global defaults for plugin configs (template expansion via defaults = "...")
    //    Must run after both include phases so that plugins loaded by includes
    //    also receive their default template values.
    config.resolve_defaults();

    // 4. Validate the fully merged config
    config.validate().map_err(|e| miette::miette!("{e}"))?;

    // ... rest unchanged
}
```

#### `load_instance()` (host/instance_lifecycle.rs)

Add `resolve_top_level_includes()`, `resolve_includes()`, and `resolve_defaults()` in the correct pipeline order. **Bug fix**: `load_instance()` currently does
not call `resolve_includes()` — this must be added for consistency with `load_config_from_file()`, since dynamically loaded instances may have per-area includes
too.

```rust
let mut config: SwipeLauncherConfig = toml::from_str( & config_content)
.map_err( | e| format!("Failed to parse config '{}': {}", config_path, e)) ?;

// 1. Resolve top-level includes
config.resolve_top_level_includes(std::path::Path::new(config_path))
.map_err( | e| format!("{e}")) ?;

// 2. Resolve per-area includes (bug fix: was missing in load_instance)
config.resolve_includes(std::path::Path::new(config_path))
.map_err( | e| format!("{e}")) ?;

// 3. Resolve defaults (template expansion)
config.resolve_defaults();
```

---

## 6. Relationship to Existing Per-Area `include`

| Aspect     | Per-Area `include`                                       | Top-Level `includes`                     |
|------------|----------------------------------------------------------|------------------------------------------|
| Scope      | Single area                                              | Entire config (defaults, plugins, areas) |
| Location   | `AreaConfig.include` field                               | `SwipeLauncherConfig.includes` field     |
| Type       | `Option<String>` (single file)                           | `Vec<String>` (multiple files)           |
| Merge      | Field-level merge for area config, key-level for plugins | Key-level merge for entries and defaults |
| Recursion  | Not supported                                            | Supported with cycle detection           |
| Hot-reload | Watched via `collect_include_paths`                      | Watched via same mechanism (extended)    |

Both mechanisms coexist. Top-level includes are resolved first (providing base entries), then per-area includes are resolved within the merged config (providing
area-level field overrides and additional plugins).

---

## 7. Implementation Phases

### Phase 1: Core Implementation

- Add `includes: Vec<String>` field to `SwipeLauncherConfig`
- Add `resolve_top_level_includes()` method with recursive include support and cycle detection
- Add new error variants to `ConfigValidationError`
- Extend `collect_include_paths()` to include top-level include paths
- **Exit Criteria**: `cargo build -p smearor_swipe_launcher` succeeds. Unit tests for merge logic pass.

### Phase 2: Call Site Integration

- Add `resolve_top_level_includes()` call in `load_config_from_file()` (args/launcher.rs)
- Add `resolve_top_level_includes()` call in `load_instance()` (host/instance_lifecycle.rs)
- Add missing `resolve_includes()` call in `load_instance()` (bug fix)
- **Exit Criteria**: Launcher starts with config files that use `includes`. Hot-reload works when include files change.

### Phase 3: Shared Config Files

- Create `configs/shared/defaults.toml` with extracted default templates
- Create `configs/shared/buttons/` directory with shared button configs
- Create `configs/shared/areas/` directory with shared area configs
- Refactor `configs/launcher/config.toml` to use top-level includes
- Refactor `configs/launcher/example-*.toml` to use top-level includes
- **Exit Criteria**: All existing configs work identically after refactoring. No duplicated plugin configs remain across example configs.

### Phase 4: Documentation

- Update `book/src/configuration/` with top-level includes documentation
- Add examples to `book/src/SUMMARY.md`
- Update `README.md` if config format is documented there
- **Exit Criteria**: `mdbook build` succeeds. Documentation clearly explains the merge order and use cases.

---

## 8. Dependencies

No new crate-level dependencies. All required crates (`serde`, `serde_json`, `toml`, `thiserror`, `tracing`) are already in use.

---

## 9. Testing Checklist

- **Basic include**: Single include file with defaults and plugin configs — merged correctly
- **Multiple includes**: Two include files with different plugins — both appear in merged config
- **Override semantics**: Same plugin ID in include and main config — main config wins
- **Default template override**: Same `[defaults.x]` in include and main config — main config wins
- **Area include**: Complete area definition in include file — area appears in merged config
- **Recursive includes**: Include file that itself has `includes` — nested includes merged correctly
- **Cycle detection**: Include A includes B, B includes A — `IncludeCycle` error returned
- **Missing include file**: Non-existent path — `TopLevelIncludeNotFound` error returned
- **Malformed include file**: Invalid TOML — `InvalidTopLevelInclude` error returned
- **Empty includes list**: `includes = []` — no-op, config loads normally
- **No includes field**: Config without `includes` — backward compatible, no-op
- **Hot-reload**: Modifying an include file triggers instance reload
- **Per-area include coexistence**: Config with both top-level includes and per-area `include` — both resolved correctly
- **Path resolution**: Include paths resolved relative to config file directory, not working directory
- **Ignored top-level fields**: Include file with `areas` or `launcher` fields — silently ignored with debug log

---

## 10. Future Enhancements

- **Glob patterns in includes**: `includes = ["../shared/buttons/*.toml"]` to auto-discover all button files in a directory
- **Include profiling**: Log which include files contribute which entries for debugging
- **Config inheritance (`extends`)**: A deeper alternative where a child config inherits the full `SwipeLauncherConfig` (including `areas`, `launcher`,
  `layout`, `profiles`) from a base config and overrides individual fields
- **Include validation**: Warn when an include file contains `areas` or `launcher` fields (currently silently ignored)
- **Deduplication analysis tool**: CLI tool that scans all config files and reports duplicated plugin configs that could be extracted into includes
