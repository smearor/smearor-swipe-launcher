# Flexible Layout System Concept

## Overview

This document describes a concept for a flexible layout system that replaces the current static three-area layout (left_area, scroll_band, right_area) with a
dynamic, configurable system.

## Current Limitations

The current implementation has:

- Hardcoded three-area structure (left, scroll, right)
- Fixed area types (static areas on sides, scroll in center)
- No ability to add custom areas or change layout order
- Static width values (200px for side areas)

## Proposed Solution

### 1. Area Type Enumeration

```rust
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum AreaType {
    Fixed,
    Scroll,
}
```

### 2. Orientation Support

```rust
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum Orientation {
    #[serde(default)]
    Horizontal,
    Vertical,
}

impl Default for Orientation {
    fn default() -> Self {
        Orientation::Horizontal
    }
}
```

### 3. Area Transition Enumeration

```rust
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum AreaTransition {
    None,
    Fade,
    SlideLeft,
    SlideRight,
    SlideUp,
    SlideDown,
    Pop,
    Scale,
}

impl Default for AreaTransition {
    fn default() -> Self {
        AreaTransition::Fade
    }
}

impl FromStr for AreaTransition {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(AreaTransition::None),
            "fade" => Ok(AreaTransition::Fade),
            "slide_left" => Ok(AreaTransition::SlideLeft),
            "slide_right" => Ok(AreaTransition::SlideRight),
            "slide_up" => Ok(AreaTransition::SlideUp),
            "slide_down" => Ok(AreaTransition::SlideDown),
            "pop" => Ok(AreaTransition::Pop),
            "scale" => Ok(AreaTransition::Scale),
            _ => Err(format!("Invalid transition: {}", s)),
        }
    }
}
```

### 4. Layout Trigger Enumeration

```rust
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum LayoutTrigger {
    Default,
    Monitor(String),
    Workspace(i32),
    MonitorWorkspace { monitor: String, workspace: i32 },
}
```

### 5. Enhanced Area Configuration

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct AreaConfig {
    #[serde(default = "default_area_type")]
    pub area_type: AreaType,

    #[serde(default = "default_width")]
    pub width: Option<i32>,

    #[serde(default)]
    pub width_percent: Option<f32>,

    #[serde(default)]
    pub min_width: Option<i32>,

    #[serde(default)]
    pub max_width: Option<i32>,

    #[serde(default)]
    pub transition: AreaTransition,

    #[serde(default)]
    pub auto_close: bool,

    #[serde(default)]
    pub close_on_escape: bool,

    pub plugins: Vec<PluginEntry>,
}

fn default_area_type() -> AreaType {
    AreaType::Fixed
}

fn default_width() -> Option<i32> {
    Some(200)
}

impl Default for AreaConfig {
    fn default() -> Self {
        AreaConfig {
            area_type: default_area_type(),
            width: default_width(),
            width_percent: None,
            min_width: None,
            max_width: None,
            transition: AreaTransition::default(),
            auto_close: false,
            close_on_escape: false,
            plugins: Vec::new(),
        }
    }
}
```

### 6. Layout Configuration

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct LayoutConfig {
    #[serde(default)]
    pub orientation: Orientation,

    #[serde(default)]
    pub spacing: i32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        LayoutConfig {
            orientation: Orientation::Horizontal,
            spacing: 0,
        }
    }
}
```

### 7. Layout Profile

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct LayoutProfile {
    pub trigger: LayoutTrigger,
    pub areas: Vec<String>,
    #[serde(flatten)]
    pub entries: HashMap<String, ConfigEntry>,
}
```

### 8. Config Entry Enum for Serde

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ConfigEntry {
    Area(AreaConfig),
    Plugin(serde_json::Value),
}
```

### 9. Dynamic Launcher Configuration

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct SwipeLauncherConfig {
    pub launcher: SwipeLauncherSettings,

    /// Layout configuration
    #[serde(default)]
    pub layout: LayoutConfig,

    /// Default layout
    pub areas: Vec<String>,

    /// Area configurations and plugin configs keyed by ID
    #[serde(flatten)]
    pub entries: HashMap<String, ConfigEntry>,

    /// Alternative layout profiles for different contexts
    #[serde(default)]
    pub profiles: Vec<LayoutProfile>,

    #[serde(default)]
    pub services: Vec<PluginEntry>,
}

impl SwipeLauncherConfig {
    pub fn get_area_config(&self, area_id: &str) -> Option<&AreaConfig> {
        self.entries.get(area_id).and_then(|entry| {
            match entry {
                ConfigEntry::Area(config) => Some(config),
                ConfigEntry::Plugin(_) => None,
            }
        })
    }

    pub fn get_plugin_config(&self, plugin_id: &str) -> Option<&serde_json::Value> {
        self.entries.get(plugin_id).and_then(|entry| {
            match entry {
                ConfigEntry::Area(_) => None,
                ConfigEntry::Plugin(value) => Some(value),
            }
        })
    }

    pub fn plugin_config(&self, id: &str) -> PluginConfig {
        let config = self.get_plugin_config(id).cloned().unwrap_or_else(|| {
            tracing::warn!("No config found for plugin {id}, using empty config");
            serde_json::json!({})
        });
        PluginConfig { config }
    }

    pub fn get_layout_for_context(
        &self,
        monitor: Option<&str>,
        workspace: Option<i32>,
    ) -> (&Vec<String>, &HashMap<String, ConfigEntry>) {
        for profile in &self.profiles {
            match &profile.trigger {
                LayoutTrigger::MonitorWorkspace { monitor: m, workspace: w } => {
                    if Some(m.as_str()) == monitor && Some(*w) == workspace {
                        return (&profile.areas, &profile.entries);
                    }
                }
                LayoutTrigger::Monitor(m) => {
                    if Some(m.as_str()) == monitor {
                        return (&profile.areas, &profile.entries);
                    }
                }
                LayoutTrigger::Workspace(w) => {
                    if Some(*w) == workspace {
                        return (&profile.areas, &profile.entries);
                    }
                }
                LayoutTrigger::Default => {}
            }
        }
        (&self.areas, &self.entries)
    }
}
```

### 10. Configuration File Format

```toml
# smearor-swipe-launcher Configuration

# Services to load
[[services]]
id = "app_launcher"
path = "target/debug/libsmearor_app_launcher_service.so"

[launcher]
rotation = 0

[layout]
orientation = "horizontal"
spacing = 0

# Define the layout order
areas = ["left_area", "my_area", "scroll_area", "right_area"]

# Left static area
[left_area]
type = "fixed"
width = 200
transition = "fade"
plugins = [
    { id = "clock_widget_left", path = "target/debug/libsmearor_clock_widget.so" }
]

# Custom fixed area in the middle
[my_area]
type = "fixed"
width = 150
transition = "slide_left"
auto_close = true
close_on_escape = true
plugins = [
    { id = "clock_widget_custom", path = "target/debug/libsmearor_clock_widget.so" }
]

# Scroll area
[scroll_area]
type = "scroll"
transition = "fade"
plugins = [
    { id = "clock_widget_date", path = "target/debug/libsmearor_clock_widget.so" },
    { id = "clock_widget_time", path = "target/debug/libsmearor_clock_widget.so" },
    { id = "firefox_nightly", path = "target/debug/libsmearor_app_launcher_widget.so" }
]

# Right static area
[right_area]
type = "fixed"
width = 200
transition = "fade"
plugins = [
    { id = "clock_widget_right", path = "target/debug/libsmearor_clock_widget.so" }
]

# Layout profile for ultrawide monitor
[[profiles]]
trigger = { monitor = "DP-1" }
areas = ["left", "content", "center", "right"]

[content]
type = "scroll"
plugins = [...]

[center]
type = "fixed"
width = 400
plugins = [...]

# Plugin-specific configurations
[clock_widget_left]
format = "[hour]:[minute]:[second]"
timezone = "UTC"
```

## Implementation Strategy

### Phase 1: Data Structure Changes

1. Add `AreaType` enum to `config/area.rs`
2. Add `Orientation` enum to `config/area.rs`
3. Add `AreaTransition` enum to `config/area.rs`
4. Add `LayoutTrigger` enum to `config/area.rs`
5. Update `AreaConfig` to include responsive width options, transition, auto_close, close_on_escape
6. Add `LayoutConfig` struct for layout settings
7. Add `LayoutProfile` struct for context-specific layouts
8. Add `ConfigEntry` enum for serde untagged deserialization
9. Modify `SwipeLauncherConfig` to use `areas` vector, `entries` map, and `profiles`
10. Add validation to ensure all area IDs in `areas` exist in `entries`

### Phase 2: Configuration Loading

1. Update config deserialization to handle the new format
2. Implement validation for area types and required fields
3. Add helper methods `get_area_config()`, `get_plugin_config()`, and `get_layout_for_context()`
4. Implement strict validation with `ConfigValidationError` enum

### Phase 3: UI Building

1. Modify `application.rs` to iterate over `areas` vector instead of hardcoded areas
2. Apply layout orientation from config (horizontal/vertical)
3. Dynamically create UI widgets based on `area_type`:
    - `Fixed`: Create `GtkBox` with calculated width
    - `Scroll`: Create `ScrolledWindow` with scrolling
4. Apply responsive width calculations (pixel, percent, min/max)
5. Apply area transitions based on `transition` field
6. Maintain existing drag-to-scroll functionality for scroll areas
7. Apply layout spacing from config
8. Implement `LayoutSwitcher` for monitor/workspace context changes

### Phase 4: Plugin Loading

1. Update `load_plugins()` to iterate over dynamic areas
2. Load plugins based on area order from config
3. Maintain plugin ID uniqueness across all areas
4. Implement plugin lifecycle management with suspend/resume

### Phase 5: Dynamic Area Management

1. Create `AreaManager` struct for runtime area operations
2. Implement `add_area_from_config()` with transition support
3. Implement `remove_area()` with plugin cleanup
4. Implement `add_transient_area()` with auto-close detection
5. Add click-outside detection for transient areas
6. Add escape key handler for transient areas
7. Implement `AreaStack` for nested sub-menus
8. Implement `clone_area()` for duplicating existing areas
9. Implement `merge_areas()` for combining multiple areas
10. Implement `hot_reload_area()` for config reloading without removal

### Phase 6: Animation System

1. Create `LayoutTransition` struct for GTK animations
2. Implement `animate_width_change()` for smooth layout transitions
3. Implement `animate_widget_addition()` for different transition types
4. Integrate animations with area addition/removal operations

### Phase 7: Documentation

1. Update README with new config format examples
2. Add migration guide for existing users
3. Document all area types and their parameters
4. Provide example configurations for common layouts
5. Add dynamic area API documentation
6. Create tutorial for sub-menu implementation

## Validation Rules

1. All area IDs in `areas` must have corresponding config sections
2. At least one area must be defined
3. Area type must be valid enum value (Fixed, Scroll)
4. For Fixed areas: either `width` or `width_percent` must be specified
5. For Scroll areas: width is ignored (scrolling takes available space)
6. If both `width` and `width_percent` are specified, `width` takes precedence
7. `min_width` and `max_width` are only valid with `width_percent`
8. Plugin IDs must be unique across all areas
9. Layout orientation must be valid (Horizontal, Vertical)

## Example Use Cases

### Simple Two-Area Layout

```toml
areas = ["left", "scroll"]

[left]
type = "fixed"
width = 150
plugins = [
    { id = "clock_widget", path = "target/debug/libsmearor_clock_widget.so" }
]

[scroll]
type = "scroll"
plugins = [
    { id = "app_launcher", path = "target/debug/libsmearor_app_launcher_widget.so" }
]
```

### Multiple Fixed Areas with Pixel Widths

```toml
areas = ["status", "main", "info"]

[status]
type = "fixed"
width = 100
plugins = [...]

[main]
type = "fixed"
width = 600
plugins = [...]

[info]
type = "fixed"
width = 200
plugins = [...]
```

### Responsive Layout with Percentages

```toml
areas = ["sidebar", "content", "extras"]

[sidebar]
type = "fixed"
width_percent = 0.2
min_width = 150
max_width = 300
plugins = [...]

[content]
type = "scroll"
plugins = [...]

[extras]
type = "fixed"
width_percent = 0.15
min_width = 100
plugins = [...]
```

### Vertical Layout

```toml
[layout]
orientation = "vertical"
spacing = 10

areas = ["top_bar", "main_content", "bottom_bar"]

[top_bar]
type = "fixed"
width = 50
plugins = [...]

[main_content]
type = "scroll"
plugins = [...]

[bottom_bar]
type = "fixed"
width = 50
plugins = [...]
```

### Scroll-Only Layout

```toml
areas = ["content"]

[content]
type = "scroll"
plugins = [
    { id = "app_launcher", path = "target/debug/libsmearor_app_launcher_widget.so" }
]
```

## Testing Strategy

1. Unit tests for config deserialization with ConfigEntry enum
2. Validation tests for area configurations
3. Integration tests for UI building with different layouts
4. Tests for responsive width calculations
5. Tests for horizontal and vertical orientations
6. Error handling tests for invalid configurations

## Problem Solutions

### 1. Layout Transition Smoothing

**Problem**: Adding/removing fixed-width areas causes GTK layout recalculation, leading to "jumping" of other elements.

**Solution**: Implement smooth transitions using GTK's animation framework:

```rust
use gtk4::gdk::FrameClock;
use gtk4::glib::Object;

pub struct LayoutTransition {
    duration: u32, // milliseconds
    easing: gtk4::gdk::EaseMode,
}

impl LayoutTransition {
    pub fn animate_width_change(
        widget: &gtk4::Widget,
        from_width: i32,
        to_width: i32,
        duration: u32,
    ) {
        let animation = gtk4::PropertyAnimation::builder()
            .widget(widget)
            .property_name("width-request")
            .interpolated_value(from_width, to_width)
            .duration(duration)
            .easing_mode(gtk4::gdk::EaseMode::EaseInOutQuad)
            .build();

        animation.play();
    }

    pub fn animate_widget_addition(
        container: &gtk4::Widget,
        new_widget: &gtk4::Widget,
        transition: AreaTransition,
    ) {
        match transition {
            AreaTransition::SlideLeft => {
                new_widget.set_opacity(0.0);
                container.append(new_widget);

                let fade_anim = gtk4::PropertyAnimation::builder()
                    .widget(new_widget)
                    .property_name("opacity")
                    .interpolated_value(0.0, 1.0)
                    .duration(300)
                    .easing_mode(gtk4::gdk::EaseMode::EaseOutQuad)
                    .build();

                fade_anim.play();
            }
            AreaTransition::Fade => {
                new_widget.set_opacity(0.0);
                container.append(new_widget);

                let fade_anim = gtk4::PropertyAnimation::builder()
                    .widget(new_widget)
                    .property_name("opacity")
                    .interpolated_value(0.0, 1.0)
                    .duration(200)
                    .easing_mode(gtk4::gdk::EaseMode::EaseOutQuad)
                    .build();

                fade_anim.play();
            }
            AreaTransition::Pop => {
                new_widget.set_opacity(0.0);
                new_widget.set_scale(0.5);
                container.append(new_widget);

                let fade_anim = gtk4::PropertyAnimation::builder()
                    .widget(new_widget)
                    .property_name("opacity")
                    .interpolated_value(0.0, 1.0)
                    .duration(250)
                    .easing_mode(gtk4::gdk::EaseMode::EaseOutBack)
                    .build();

                let scale_anim = gtk4::PropertyAnimation::builder()
                    .widget(new_widget)
                    .property_name("scale")
                    .interpolated_value(0.5, 1.0)
                    .duration(250)
                    .easing_mode(gtk4::gdk::EaseMode::EaseOutBack)
                    .build();

                fade_anim.play();
                scale_anim.play();
            }
        }
    }
}
```

**Container-level smoothing**: Use `GtkBox` with `homogeneous=false` and animate width changes before adding/removing widgets:

```rust
pub fn add_area_with_transition(
    container: &GtkBox,
    area_widget: &Widget,
    position: AreaPosition,
    transition: AreaTransition,
) {
    // Pre-calculate final layout
    let final_width = calculate_final_width(container, area_widget, position);

    // Animate container width change if needed
    if let Some(current_width) = container.width_request() {
        if current_width != final_width {
            LayoutTransition::animate_width_change(container, current_width, final_width, 300);
        }
    }

    // Add widget with transition
    LayoutTransition::animate_widget_addition(container, area_widget, transition);
}
```

### 2. Plugin Lifecycle Management

**Problem**: Hidden/removed plugins continue consuming CPU cycles.

**Solution**: Implement plugin lifecycle states with suspend/resume:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum PluginState {
    Active,
    Suspended,
    Unloaded,
}

pub struct PluginLifecycle {
    state: PluginState,
    suspend_reason: Option<String>,
}

impl PluginLifecycle {
    pub fn suspend(&mut self, reason: String) {
        if self.state == PluginState::Active {
            self.state = PluginState::Suspended;
            self.suspend_reason = Some(reason);
            // Notify plugin to suspend
            self.notify_suspend();
        }
    }

    pub fn resume(&mut self) {
        if self.state == PluginState::Suspended {
            self.state = PluginState::Active;
            self.suspend_reason = None;
            // Notify plugin to resume
            self.notify_resume();
        }
    }

    fn notify_suspend(&self) {
        // Call plugin's suspend callback if available
        // Stop timers, pause animations, reduce update frequency
    }

    fn notify_resume(&self) {
        // Call plugin's resume callback if available
        // Restart timers, resume animations, restore update frequency
    }
}

// Extend plugin API
pub trait PluginLifecycleCallbacks {
    fn on_suspend(&self);
    fn on_resume(&self);
}
```

**Integration with AreaManager**:

```rust
impl AreaManager {
    pub fn hide_area(&self, area_id: &str) -> miette Result<() > {
    let area_config = self.get_area_config(area_id) ?;

    // Suspend all plugins in the area
    for plugin_entry in & area_config.plugins {
    if let Some(plugin) = self.plugin_manager.get_plugin( & plugin_entry.id) {
    plugin.lifecycle.suspend(format ! ("Area {} hidden", area_id));
    }
    }

    // Hide widget
    self.hide_area_widget(area_id) ?;

    Ok(())
    }

    pub fn show_area(&self, area_id: &str) -> miette Result<() > {
    let area_config = self.get_area_config(area_id) ?;

    // Show widget
    self.show_area_widget(area_id) ?;

    // Resume all plugins in the area
    for plugin_entry in & area_config.plugins {
    if let Some(plugin) = self.plugin_manager.get_plugin( & plugin_entry.id) {
    plugin.lifecycle.resume();
    }
    }

    Ok(())
    }

    pub fn remove_area(&self, area_id: String) -> miette Result<() > {
    let area_config = self.get_area_config( & area_id)?;

    // Suspend and unload plugins
    for plugin_entry in & area_config.plugins {
    if let Some(plugin) = self.plugin_manager.get_plugin( & plugin_entry.id) {
    plugin.lifecycle.suspend(format ! ("Area {} removed", area_id));
    }
    self.plugin_manager.unload_plugin( & plugin_entry.id) ?;
    }

    // Remove UI widget
    self.remove_area_widget( & area_id) ?;

    Ok(())
    }
}
```

### 3. Comprehensive Error Handling

**Problem**: Half-loaded layouts due to configuration errors and runtime failures.

**Solution**: Enforce strict validation with custom error types covering both static validation and runtime errors:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigValidationError {
    #[error("Area '{area_id}' listed in areas but not found in configuration")]
    AreaNotFound { area_id: String },

    #[error("Area '{area_id}' configuration is invalid: {reason}")]
    InvalidAreaConfig { area_id: String, reason: String },

    #[error("No areas defined in configuration")]
    NoAreasDefined,

    #[error("Duplicate area ID '{area_id}' found in areas list")]
    DuplicateAreaId { area_id: String },

    #[error("Plugin ID '{plugin_id}' is duplicated across areas")]
    DuplicatePluginId { plugin_id: String },

    #[error("Invalid area type '{area_type}' for area '{area_id}'")]
    InvalidAreaType { area_id: String, area_type: String },

    #[error("Fixed area '{area_id}' must have either width or width_percent specified")]
    MissingWidthSpec { area_id: String },

    #[error("Invalid width_percent {percent} for area '{area_id}': must be between 0.0 and 1.0")]
    InvalidWidthPercent { area_id: String, percent: f32 },

    #[error("Invalid transition '{transition}' for area '{area_id}'")]
    InvalidTransition { area_id: String, transition: String },

    #[error("Layout trigger validation failed: {reason}")]
    InvalidLayoutTrigger { reason: String },
}

#[derive(Error, Debug)]
pub enum AreaManagerError {
    #[error("Config file not found: {path}")]
    ConfigFileNotFound { path: String },

    #[error("Failed to read config file '{path}': {0}")]
    ConfigReadError { path: String, source: std::io::Error },

    #[error("Invalid config format in '{path}': {reason}")]
    InvalidConfigFormat { path: String, reason: String },

    #[error("Failed to load plugin '{plugin_id}': {reason}")]
    PluginLoadError { plugin_id: String, reason: String },

    #[error("Area '{area_id}' already exists")]
    AreaAlreadyExists { area_id: String },

    #[error("Area '{area_id}' not found")]
    AreaNotFound { area_id: String },

    #[error("Position target '{target_id}' not found for area insertion")]
    PositionTargetNotFound { target_id: String },

    #[error("Area ID conflict: '{area_id}' is already in use")]
    AreaIdConflict { area_id: String },

    #[error("Invalid area operation: {reason}")]
    InvalidOperation { reason: String },
}

#[derive(Error, Debug)]
pub enum ConfigLoadError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] toml::de::Error),

    #[error("Config validation failed: {0}")]
    ValidationError(#[from] ConfigValidationError),
}

impl SwipeLauncherConfig {
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        // Check that areas list is not empty
        if self.areas.is_empty() {
            return Err(ConfigValidationError::NoAreasDefined);
        }

        // Check for duplicate area IDs
        let mut seen_areas = std::collections::HashSet::new();
        for area_id in &self.areas {
            if !seen_areas.insert(area_id) {
                return Err(ConfigValidationError::DuplicateAreaId {
                    area_id: area_id.clone(),
                });
            }
        }

        // Check that all areas in the list have configurations
        for area_id in &self.areas {
            if self.get_area_config(area_id).is_none() {
                return Err(ConfigValidationError::AreaNotFound {
                    area_id: area_id.clone(),
                });
            }
        }

        // Validate each area configuration
        for area_id in &self.areas {
            if let Some(area_config) = self.get_area_config(area_id) {
                self.validate_area_config(area_id, area_config)?;
            }
        }

        // Check for duplicate plugin IDs across all areas
        let mut seen_plugins = std::collections::HashSet::new();
        for area_id in &self.areas {
            if let Some(area_config) = self.get_area_config(area_id) {
                for plugin in &area_config.plugins {
                    if !seen_plugins.insert(&plugin.id) {
                        return Err(ConfigValidationError::DuplicatePluginId {
                            plugin_id: plugin.id.clone(),
                        });
                    }
                }
            }
        }

        // Validate layout profiles
        for profile in &self.profiles {
            self.validate_layout_profile(profile)?;
        }

        Ok(())
    }

    fn validate_area_config(
        &self,
        area_id: &str,
        area_config: &AreaConfig,
    ) -> Result<(), ConfigValidationError> {
        // Validate area type
        match area_config.area_type {
            AreaType::Fixed => {
                // Fixed areas must have width or width_percent
                if area_config.width.is_none() && area_config.width_percent.is_none() {
                    return Err(ConfigValidationError::MissingWidthSpec {
                        area_id: area_id.to_string(),
                    });
                }

                // Validate width_percent range
                if let Some(percent) = area_config.width_percent {
                    if percent <= 0.0 || percent > 1.0 {
                        return Err(ConfigValidationError::InvalidWidthPercent {
                            area_id: area_id.to_string(),
                            percent,
                        });
                    }
                }
            }
            AreaType::Scroll => {
                // Scroll areas ignore width, no validation needed
            }
        }

        // Validate transition if specified
        if let Err(e) = std::str::FromStr::from_str(&format!("{:?}", area_config.transition)) {
            return Err(ConfigValidationError::InvalidTransition {
                area_id: area_id.to_string(),
                transition: format!("{:?}", area_config.transition),
            });
        }

        Ok(())
    }

    fn validate_layout_profile(
        &self,
        profile: &LayoutProfile,
    ) -> Result<(), ConfigValidationError> {
        // Validate that profile areas exist in profile entries
        for area_id in &profile.areas {
            if !profile.entries.contains_key(area_id) {
                return Err(ConfigValidationError::AreaNotFound {
                    area_id: area_id.clone(),
                });
            }
        }
        Ok(())
    }
}

// Use in config loading
pub fn load_config(path: &Path) -> Result<SwipeLauncherConfig, ConfigLoadError> {
    let content = std::fs::read_to_string(path)?;
    let config: SwipeLauncherConfig = toml::from_str(&content)?;

    // Strict validation - fail fast on any error
    config.validate()?;

    Ok(config)
}

impl AreaManager {
    pub fn load_area_config(&self, path: &Path) -> Result<AreaConfig, AreaManagerError> {
        if !path.exists() {
            return Err(AreaManagerError::ConfigFileNotFound {
                path: path.display().to_string(),
            });
        }

        let content = std::fs::read_to_string(path).map_err(|e| {
            AreaManagerError::ConfigReadError {
                path: path.display().to_string(),
                source: e,
            }
        })?;

        let area_config: AreaConfig = toml::from_str(&content).map_err(|e| {
            AreaManagerError::InvalidConfigFormat {
                path: path.display().to_string(),
                reason: e.to_string(),
            }
        })?;

        Ok(area_config)
    }

    pub fn add_area_from_config(
        &self,
        area_id: String,
        config_path: PathBuf,
        position: AreaPosition,
    ) -> Result<(), AreaManagerError> {
        // Check if area already exists
        if self.areas.borrow().contains_key(&area_id) {
            return Err(AreaManagerError::AreaAlreadyExists { area_id });
        }

        // Validate position target
        if let AreaPosition::After(target_id) = &position {
            if !self.areas.borrow().contains_key(target_id) {
                return Err(AreaManagerError::PositionTargetNotFound {
                    target_id: target_id.clone(),
                });
            }
        }

        let area_config = self.load_area_config(&config_path)?;

        // Load plugins with error handling
        for plugin_entry in &area_config.plugins {
            if let Err(e) = self.plugin_manager.load_plugin(plugin_entry, PluginConfig::default()) {
                tracing::error!("Failed to load plugin {}: {}", plugin_entry.id, e);
                return Err(AreaManagerError::PluginLoadError {
                    plugin_id: plugin_entry.id.clone(),
                    reason: e.to_string(),
                });
            }
        }

        // Build and insert widget
        let area_widget = self.build_area_widget(&area_config)?;
        self.insert_area_widget(area_id, area_widget, position)?;

        Ok(())
    }
}
```

## Performance Considerations

- Area iteration is O(n) where n is number of areas (typically small)
- Plugin loading remains unchanged
- UI building is dynamic but still linear in number of areas
- No significant performance impact expected
- Plugin suspend/resume reduces CPU usage for hidden areas
- Layout profiles add minimal overhead for context switching
- Lazy loading: Only load plugins when area is added
- Plugin caching: Keep frequently used plugins loaded
- UI updates: Use batch updates for multiple area changes
- Memory management: Unload plugins when area is removed

## Dynamic Areas at Runtime

### Overview

Areas can be dynamically added or removed at runtime, enabling use cases like sub-menus, contextual panels, and dynamic content areas. This is particularly
useful for navigation systems where clicking a button opens a new area with specific content.

### API Extensions

#### 1. Area Management API

```rust
pub struct AreaManager {
    areas: Rc<RefCell<HashMap<String, AreaConfig>>>,
    area_order: Rc<RefCell<Vec<String>>>,
    plugin_manager: Arc<PluginManager>,
    main_container: GtkBox,
}

impl AreaManager {
    /// Add a new area at runtime from a config file
    pub fn add_area_from_config(
        &self,
        area_id: String,
        config_path: PathBuf,
        position: AreaPosition,
    ) -> miette Result<() > {
    // Load area config from file
    let area_config = self.load_area_config( & config_path)?;

    // Load plugins for the area
    self.load_area_plugins( & area_id, &area_config) ?;

    // Build UI for the area
    let area_widget = self.build_area_widget( & area_config) ?;

    // Insert at specified position
    self.insert_area_widget(area_id, area_widget, position);

    Ok(())
    }

    /// Remove an area at runtime
    pub fn remove_area(&self, area_id: String) -> miette Result<() > {
    // Unload plugins
    self.unload_area_plugins( & area_id) ?;

    // Remove UI widget
    self.remove_area_widget( & area_id) ?;

    Ok(())
    }

    /// Replace an area with another from config
    pub fn replace_area_from_config(
        &self,
        area_id: String,
        config_path: PathBuf,
    ) -> miette Result<() > {
    self.remove_area(area_id.clone())?;
    self.add_area_from_config(area_id, config_path, AreaPosition::Replace) ?;
    Ok(())
    }

    /// Clone an existing area with a new ID
    pub fn clone_area(
        &self,
        source_area_id: String,
        new_area_id: String,
        position: AreaPosition,
    ) -> miette Result<() > {
    let source_config = self.get_area_config( & source_area_id)
    .ok_or_else( | | miette ! ("Source area {} not found", source_area_id)) ?;

    // Create new config from source
    let new_config = source_config.clone();

    // Load plugins for the new area
    self.load_area_plugins( & new_area_id, & new_config) ?;

    // Build UI for the new area
    let area_widget = self.build_area_widget( & new_config) ?;

    // Insert at specified position
    self.insert_area_widget(new_area_id, area_widget, position);

    Ok(())
    }

    /// Merge multiple areas into a single area
    pub fn merge_areas(
        &self,
        source_area_ids: Vec<String>,
        target_area_id: String,
        target_config: AreaConfig,
    ) -> miette Result<() > {
    // Collect all plugins from source areas
    let mut all_plugins = Vec::new();
    for source_id in & source_area_ids {
    if let Some(source_config) = self.get_area_config(source_id) {
    all_plugins.extend(source_config.plugins.clone());
    }
    }

    // Create merged config
    let mut merged_config = target_config;
    merged_config.plugins = all_plugins;

    // Remove source areas
    for source_id in & source_area_ids {
    self.remove_area(source_id.clone())?;
    }

    // Add merged area
    self.load_area_plugins( & target_area_id, & merged_config)?;
    let area_widget = self.build_area_widget( &merged_config) ?;
    self.insert_area_widget(target_area_id, area_widget, AreaPosition::End) ?;

    Ok(())
    }

    /// Hot-reload an area configuration without removing it
    pub fn hot_reload_area(
        &self,
        area_id: String,
        config_path: PathBuf,
    ) -> miette Result<() > {
    let new_config = self.load_area_config( & config_path)?;
    let old_config = self.get_area_config( &area_id)
    .ok_or_else( | | miette ! ("Area {} not found", area_id)) ?;

    // Determine which plugins need to be added/removed
    let old_plugin_ids: std::collections::HashSet < _ > =
    old_config.plugins.iter().map( |p | & p.id).collect();
    let new_plugin_ids: std::collections::HashSet< _ > =
    new_config.plugins.iter().map( | p | & p.id).collect();

    // Remove plugins that are no longer in the new config
    for plugin_id in old_plugin_ids.difference( & new_plugin_ids) {
    self.plugin_manager.unload_plugin(plugin_id) ?;
    }

    // Add new plugins
    for plugin_entry in & new_config.plugins {
    if ! old_plugin_ids.contains( & plugin_entry.id) {
    let plugin_config = self.plugin_config( & plugin_entry.id);
    self.plugin_manager.load_plugin(plugin_entry, plugin_config) ?;
    }
    }

    // Update area configuration
    self.areas.borrow_mut().insert(area_id.clone(), new_config);

    // Rebuild UI widget
    let area_widget = self.build_area_widget( & new_config) ?;
    self.replace_area_widget( & area_id, area_widget)?;

    Ok(())
    }
}

pub enum AreaPosition {
    Start,
    End,
    After(String),
    Before(String),
    Replace,
}
```

#### 2. Plugin API for Area Management

Extend the plugin API to allow plugins to request area changes:

```rust
// In smearor-swipe-launcher-plugin-api
pub enum AreaAction {
    AddArea {
        area_id: String,
        config_path: String,
        position: AreaPosition,
    },
    RemoveArea {
        area_id: String,
    },
    ReplaceArea {
        area_id: String,
        config_path: String,
    },
    CloneArea {
        source_area_id: String,
        new_area_id: String,
        position: AreaPosition,
    },
    MergeAreas {
        source_area_ids: Vec<String>,
        target_area_id: String,
        target_config: AreaConfig,
    },
    HotReloadArea {
        area_id: String,
        config_path: String,
    },
}

pub fn request_area_change(action: AreaAction);
```

#### 3. Dynamic Area Config File Format

Separate config files for dynamic areas:

```toml
# configs/submenu_settings.toml
type = "fixed"
width = 300
plugins = [
    { id = "settings_general", path = "target/debug/libsmearor_settings_widget.so" },
    { id = "settings_display", path = "target/debug/libsmearor_settings_widget.so" },
    { id = "settings_audio", path = "target/debug/libsmearor_settings_widget.so" }
]

[settings_general]
icon = "general"
label = "General"

[settings_display]
icon = "display"
label = "Display"

[settings_audio]
icon = "audio"
label = "Audio"
```

### Implementation Strategy

#### Phase 1: Core Area Management

1. Create `AreaManager` struct to handle dynamic area operations
2. Implement `add_area_from_config()` method
3. Implement `remove_area()` method
4. Add thread-safe area state management using `Rc<RefCell<>>`

#### Phase 2: UI Integration

1. Extend main container to support dynamic widget insertion/removal
2. Implement area positioning logic (start, end, after, before, replace)
3. Add animations for smooth area transitions
4. Handle layout recalculation after area changes

#### Phase 3: Plugin API Extension

1. Add `AreaAction` enum to plugin API
2. Implement `request_area_change()` function
3. Route area change requests from plugins to AreaManager
4. Add error handling for invalid area operations

#### Phase 4: Config File Loading

1. Implement config file parser for dynamic area configs
2. Add validation for dynamic area configurations
3. Support relative and absolute config paths
4. Add config file watching for hot-reload (optional)

### Use Case: Sub-menu System

#### Main Config

```toml
areas = ["navigation", "content"]

[navigation]
type = "fixed"
width = 100
plugins = [
    { id = "menu_button", path = "target/debug/libsmearor_menu_widget.so" }
]

[content]
type = "scroll"
plugins = [
    { id = "main_content", path = "target/debug/libsmearor_content_widget.so" }
]

[menu_button]
label = "Menu"
submenu_config = "configs/submenu_settings.toml"
```

#### Plugin Implementation

```rust
// In menu widget plugin
use smearor_swipe_launcher_plugin_api::{AreaAction, AreaPosition, request_area_change};

pub struct MenuWidget {
    button: gtk4::Button,
    submenu_config: String,
}

impl MenuWidget {
    pub fn on_button_click(&self) {
        let action = AreaAction::AddArea {
            area_id: "submenu".to_string(),
            config_path: self.submenu_config.clone(),
            position: AreaPosition::After("navigation".to_string()),
        };
        request_area_change(action);
    }

    pub fn on_close_click(&self) {
        let action = AreaAction::RemoveArea {
            area_id: "submenu".to_string(),
        };
        request_area_change(action);
    }
}
```

### Area Stack Management

For nested sub-menus, implement a stack-based approach:

```rust
pub struct AreaStack {
    stack: Rc<RefCell<Vec<String>>>,
    base_areas: Vec<String>,
}

impl AreaStack {
    pub fn push_area(&self, area_id: String, config_path: PathBuf) -> miette Result<() > {
    // Hide current top area if exists
    if let Some(current) = self.stack.borrow().last() {
    self.hide_area(current) ?;
    }

    // Add new area
    self.add_area_from_config(area_id.clone(), config_path, AreaPosition::End) ?;
    self.stack.borrow_mut().push(area_id);

    Ok(())
    }

    pub fn pop_area(&self) -> miette Result<() > {
    let area_id = self.stack.borrow_mut().pop()
    .ok_or_else( | | miette! ("Area stack is empty")) ?;

    self.remove_area(area_id.clone()) ?;

    // Show previous area if exists
    if let Some(previous) = self.stack.borrow().last() {
    self.show_area(previous) ?;
    }

    Ok(())
    }

    pub fn reset_to_base(&self) -> miette Result<() > {
    while let Some(area_id) = self.stack.borrow().pop() {
    self.remove_area(area_id) ?;
    }
    Ok(())
    }
}
```

### Configuration Examples

#### Simple Sub-menu

```toml
# configs/submenu_apps.toml
type = "fixed"
width = 250
plugins = [
    { id = "app_category_games", path = "target/debug/libsmearor_category_widget.so" },
    { id = "app_category_productivity", path = "target/debug/libsmearor_category_widget.so" },
    { id = "app_category_media", path = "target/debug/libsmearor_category_widget.so" }
]
```

#### Contextual Panel

```toml
# configs/context_panel.toml
type = "scroll"
plugins = [
    { id = "context_info", path = "target/debug/libsmearor_context_widget.so" },
    { id = "context_actions", path = "target/debug/libsmearor_actions_widget.so" }
]
```

#### Multi-level Menu

```toml
# configs/level1_menu.toml
type = "fixed"
width = 200
plugins = [
    { id = "menu_item_settings", path = "target/debug/libsmearor_menu_item.so" },
    { id = "menu_item_apps", path = "target/debug/libsmearor_menu_item.so" }
]

[menu_item_settings]
label = "Settings"
submenu_config = "configs/level2_settings.toml"

[menu_item_apps]
label = "Applications"
submenu_config = "configs/level2_apps.toml"
```
