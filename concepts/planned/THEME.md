# Concept: Theme Service & Widget

This document describes the concept for a **Theme Service**, a **Theme Switcher Widget**, and the shared **Personalization & Wallpaper Integration** between the
Theme, Personalization, and Wallpaper services. All components follow the decoupled architecture of the *Smearor Swipe Launcher*.

---

## 1. Motivation

The launcher currently applies CSS globally via a built-in `style.css` and optional per-instance user CSS files. There is no mechanism for users to define named
themes that bundle CSS files, respond to the system color scheme (dark/light/system), and optionally couple with wallpaper themes. A dedicated Theme system
enables:

- **Named themes** with metadata (name, description, mode, CSS files, preview icon, optional preview image, 5 theme colors per mode)
- **CSS custom properties (variables)**: each theme defines 5 colors per mode (Dark and Light) exported as CSS variables (`--theme-color-1` through
  `--theme-color-5`), enabling easy color customization without editing CSS files and eliminating the need for separate dark-mode and light-mode themes
- **Mode-aware switching**: themes can be fixed (Dark/Light) or follow the system color scheme (System)
- **Wallpaper coupling**: a theme can optionally select and start a wallpaper theme (e.g. "Halloween" theme + "Halloween Pumpkins" wallpaper)
- **MCP integration**: voice assistant and external automation can query and switch themes
- **Hot-reload**: CSS file changes are picked up without restarting the launcher
- **Default theme**: a built-in `default` theme ships with the official Smearor design palette (see `docs/DESIGN.md`)

The Theme system is implemented in dedicated crates (`model/theme`, `services/theme`, `plugins/theme`), analogous to the Wallpaper crates.

---

## 2. Crate Structure

| Crate       | Path              | Responsibility                                                                          |
|-------------|-------------------|-----------------------------------------------------------------------------------------|
| **Model**   | `model/theme/`    | Shared structs, enums, message formats, FFI types, MCP tool definitions                 |
| **Service** | `services/theme/` | Theme config loading, CSS application, status broadcasts, wallpaper coupling, MCP tools |
| **Widget**  | `plugins/theme/`  | GTK4 tile widget with per-theme views, swipe navigation, and preview images             |

---

## 3. Model Crate (`model/theme`)

### 3.1 Message Topics

```rust
pub const TOPIC_STATUS: &str = "service.theme.status";
pub const TOPIC_COMMAND: &str = "service.theme.command";
```

### 3.2 Theme Colors

Each theme defines **5 colors** for both **Dark** and **Light** modes, exported as CSS custom properties (variables). This enables users to customize colors
without editing CSS files and eliminates the need for separate dark-mode and light-mode themes — a single theme adapts its colors automatically based on the
effective mode.

The service resolves the effective mode (Dark or Light), selects the matching palette, generates a CSS string with `:root { --theme-color-1: ...; }`
declarations, and applies it as a dedicated `CssProvider` alongside the theme's CSS files.

The 5 colors correspond to the official Smearor design palette defined in `docs/DESIGN.md`:

| Slot      | CSS Variable      | Default (default theme) | Color Name       |
|-----------|-------------------|-------------------------|------------------|
| `color_1` | `--theme-color-1` | `#04e762ff`             | malachite        |
| `color_2` | `--theme-color-2` | `#f5b700ff`             | selective-yellow |
| `color_3` | `--theme-color-3` | `#00a1e4ff`             | celestial-blue   |
| `color_4` | `--theme-color-4` | `#dc0073ff`             | mexican-pink     |
| `color_5` | `--theme-color-5` | `#89fc00ff`             | chartreuse       |

```rust
/// Five theme colors exported as CSS custom properties.
///
/// Each color is a hex string (e.g. "#04e762ff") that the theme service
/// injects as a CSS variable (`--theme-color-1` through `--theme-color-5`)
/// via a generated `:root { ... }` CSS block.
///
/// The default values correspond to the official Smearor design palette
/// defined in `docs/DESIGN.md`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ThemePalette {
    /// Primary color. Exported as `--theme-color-1`.
    /// Default: malachite `#04e762ff`.
    #[serde(default = "default_color_1")]
    pub color_1: String,

    /// Secondary color. Exported as `--theme-color-2`.
    /// Default: selective-yellow `#f5b700ff`.
    #[serde(default = "default_color_2")]
    pub color_2: String,

    /// Tertiary color. Exported as `--theme-color-3`.
    /// Default: celestial-blue `#00a1e4ff`.
    #[serde(default = "default_color_3")]
    pub color_3: String,

    /// Quaternary color. Exported as `--theme-color-4`.
    /// Default: mexican-pink `#dc0073ff`.
    #[serde(default = "default_color_4")]
    pub color_4: String,

    /// Quinary color. Exported as `--theme-color-5`.
    /// Default: chartreuse `#89fc00ff`.
    #[serde(default = "default_color_5")]
    pub color_5: String,
}

fn default_color_1() -> String {
    "#04e762ff".to_string() // malachite
}

fn default_color_2() -> String {
    "#f5b700ff".to_string() // selective-yellow
}

fn default_color_3() -> String {
    "#00a1e4ff".to_string() // celestial-blue
}

fn default_color_4() -> String {
    "#dc0073ff".to_string() // mexican-pink
}

fn default_color_5() -> String {
    "#89fc00ff".to_string() // chartreuse
}

impl ThemePalette {
    /// Generates a CSS `:root { ... }` block with all 5 color variables.
    /// Used by the service to inject CSS custom properties via `CssProvider::load_from_data()`.
    pub fn to_css(&self) -> String {
        format!(
            ":root {{\n\
            \    --theme-color-1: {};\n\
            \    --theme-color-2: {};\n\
            \    --theme-color-3: {};\n\
            \    --theme-color-4: {};\n\
            \    --theme-color-5: {};\n\
            }}",
            self.color_1, self.color_2, self.color_3, self.color_4, self.color_5
        )
    }
}

impl Default for ThemePalette {
    fn default() -> Self {
        Self {
            color_1: default_color_1(),
            color_2: default_color_2(),
            color_3: default_color_3(),
            color_4: default_color_4(),
            color_5: default_color_5(),
        }
    }
}

/// Theme colors for both Dark and Light modes.
///
/// Each mode has its own `ThemePalette` with 5 colors. The service selects
/// the appropriate palette based on the effective mode (Dark or Light) and
/// injects the corresponding CSS custom properties.
///
/// This eliminates the need for separate dark-mode and light-mode themes —
/// a single theme adapts its colors automatically.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ThemeColors {
    /// Color palette for Dark mode.
    /// Defaults to the official Smearor design palette.
    #[serde(default)]
    pub dark: ThemePalette,

    /// Color palette for Light mode.
    /// Defaults to the official Smearor design palette.
    #[serde(default)]
    pub light: ThemePalette,
}

impl ThemeColors {
    /// Returns the palette for the given effective mode.
    /// For Dark mode, returns `self.dark`; for Light mode, returns `self.light`.
    /// For System mode, the caller must resolve to Dark or Light first.
    pub fn palette_for_mode(&self, mode: ThemeMode) -> &ThemePalette {
        match mode {
            ThemeMode::Dark => &self.dark,
            ThemeMode::Light => &self.light,
            ThemeMode::System => &self.dark, // fallback — System should be resolved before calling
        }
    }

    /// Generates a CSS `:root { ... }` block with all 5 color variables
    /// for the given effective mode.
    /// Used by the service to inject CSS custom properties via `CssProvider::load_from_data()`.
    pub fn to_css(&self, mode: ThemeMode) -> String {
        self.palette_for_mode(mode).to_css()
    }
}
```

### 3.3 Theme Definition

```rust
/// A theme definition with metadata, CSS files, theme colors, and optional wallpaper coupling.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Theme {
    /// Human-readable name of the theme (e.g. "default", "Halloween").
    pub name: String,
    /// Description of the theme.
    #[serde(default)]
    pub description: String,
    /// Nerd Font icon name shown in the widget tile (e.g. "nf-md-palette").
    #[serde(default)]
    pub preview_icon: String,
    /// Optional path to a preview image shown in the widget tile.
    /// When set, the widget displays this image instead of the Nerd Font icon.
    #[serde(default)]
    pub preview_image_path: String,
    /// Color scheme mode: Dark, Light, or System.
    /// System mode resolves based on the personalization service's ColorScheme.
    #[serde(default)]
    pub mode: ThemeMode,
    /// CSS file paths applied when the effective mode is Dark.
    /// Used for Dark mode and System mode (when resolved to Dark).
    /// Multiple files may be provided; all are loaded as separate CssProviders.
    #[serde(default)]
    pub css_files_dark: Vec<String>,
    /// CSS file paths applied when the effective mode is Light.
    /// Used for Light mode and System mode (when resolved to Light).
    /// Multiple files may be provided; all are loaded as separate CssProviders.
    /// If empty, `css_files_dark` is used as fallback for Light mode.
    #[serde(default)]
    pub css_files_light: Vec<String>,
    /// Theme colors for Dark and Light modes (5 hex strings each).
    /// Defaults to the official Smearor design palette for both modes.
    #[serde(default)]
    pub colors: ThemeColors,
    /// Optional wallpaper theme name to couple with this theme.
    /// When set, applying this theme also selects and starts the named wallpaper theme.
    #[serde(default)]
    pub wallpaper_theme: Option<String>,
}
```

### 3.4 Theme Mode Enum

The `ThemeMode` enum reuses the same variants as `smearor_personalization_model::ColorScheme` but is defined independently to avoid a cross-crate dependency
from the model crate. The service crate performs the mapping.

```rust
/// Color scheme mode for a theme.
/// Determines how the theme resolves CSS files and reacts to system color scheme changes.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    /// Follow system color scheme (default). Resolves to Dark or Light based on
    /// the personalization service's ColorScheme. Uses `css_files_dark` when
    /// resolved to Dark, `css_files_light` when resolved to Light.
    #[default]
    System,
    /// Fixed dark mode. Uses `css_files_dark`.
    Dark,
    /// Fixed light mode. Uses `css_files_light` (falls back to `css_files_dark` if empty).
    Light,
}

impl std::str::FromStr for ThemeMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "System" | "system" => Ok(ThemeMode::System),
            "Dark" | "dark" => Ok(ThemeMode::Dark),
            "Light" | "light" => Ok(ThemeMode::Light),
            _ => Err(format!("Unknown theme mode: {s}")),
        }
    }
}

impl ThemeMode {
    /// Resolves the effective mode given the current system color scheme.
    /// System mode resolves to Dark or Light based on the personalization status.
    /// Dark and Light modes return themselves unchanged.
    pub fn resolve(self, system_scheme: smearor_personalization_model::ColorScheme) -> Self {
        match self {
            ThemeMode::System => match system_scheme {
                smearor_personalization_model::ColorScheme::Dark => ThemeMode::Dark,
                smearor_personalization_model::ColorScheme::Light => ThemeMode::Light,
                smearor_personalization_model::ColorScheme::System => ThemeMode::Dark, // fallback
            },
            ThemeMode::Dark => ThemeMode::Dark,
            ThemeMode::Light => ThemeMode::Light,
        }
    }
}
```

### 3.5 Theme Info (for status broadcast)

```rust
/// Lightweight theme info included in status messages.
/// Contains only display-relevant fields, not full CSS file paths.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct ThemeInfo {
    /// Theme name.
    pub name: StabbyString,
    /// Theme description.
    pub description: StabbyString,
    /// Nerd Font preview icon name.
    pub preview_icon: StabbyString,
    /// Optional path to a preview image shown in the widget tile.
    pub preview_image_path: StabbyString,
    /// Theme mode (Dark, Light, System).
    pub mode: ThemeMode,
    /// Whether this theme is coupled with a wallpaper theme.
    pub has_wallpaper: bool,
    /// Theme colors for Dark and Light modes (5 hex strings each).
    pub colors: ThemeColors,
}
```

### 3.6 Theme Status Message

```rust
/// Status message broadcast by the theme service.
/// Consumed by the theme switcher widget and other interested services.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct ThemeStatusMessage {
    /// All configured themes (display info only).
    pub themes: StabbyVec<ThemeInfo>,
    /// Index of the currently selected theme in the `themes` list.
    pub selected_theme_index: u32,
    /// Name of the currently applied theme, if any.
    pub current_theme: StabbyOption<StabbyString>,
    /// The effective mode after System resolution (Dark or Light).
    /// For fixed-mode themes, this equals the theme's mode.
    pub effective_mode: ThemeMode,
    /// Timestamp of the last status update (ISO 8601).
    pub last_updated: StabbyString,
}
```

### 3.7 Command Message

```rust
/// Actions that the theme service can perform.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum ThemeCommandAction {
    /// Select a theme by name. Does not apply CSS — use `ApplySelected` to apply.
    #[default]
    SelectTheme,
    /// Apply the currently selected theme (load CSS, optionally start wallpaper).
    ApplySelected,
    /// Select a theme by name and apply it immediately.
    SelectAndApply,
    /// Refresh status and re-broadcast.
    Refresh,
    /// Add a new theme to the configuration.
    AddTheme,
    /// Remove a theme from the configuration by name.
    RemoveTheme,
}

/// Command message sent to the theme service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct ThemeCommandMessage {
    /// The action to perform.
    pub action: ThemeCommandAction,
    /// Theme name for name-based actions (`SelectTheme`, `SelectAndApply`, `RemoveTheme`).
    pub name: StabbyOption<StabbyString>,
    /// Full theme definition for `AddTheme`.
    pub theme: StabbyOption<Theme>,
}
```

### 3.8 Theme View Enum (Deprecated)

The `ThemeView` enum (`CurrentTheme`, `ThemeList`, `ModeIndicator`) was used in the initial design for view-based rotation. It has been **replaced** by a
per-theme view model inspired by the Wallpaper widget:

- **One view per theme**: each configured theme gets its own view in the widget.
- **Swipe up/down** cycles through themes (selects without applying).
- **Click** applies the currently selected theme.
- **Preview image**: each view shows a preview image (if `preview_image_path` is set) or falls back to the Nerd Font icon.

The enum is retained in the model crate for backward compatibility but is no longer used by the widget.

### 3.9 MCP Tools Enum

```rust
use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP tools registered by the theme service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMcpTools {
    /// Get the current theme status (applied theme, effective mode, configured themes).
    GetTheme,
    /// Set the current theme by name (selects and applies immediately).
    SetTheme,
}

impl AsRef<str> for ThemeMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::GetTheme => "get_theme",
            Self::SetTheme => "set_theme",
        }
    }
}

impl FromStr for ThemeMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "get_theme" => Ok(Self::GetTheme),
            "set_theme" => Ok(Self::SetTheme),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for ThemeMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
```

### 3.10 MCP Prompts Enum

```rust
use smearor_model_mcp::UnknownPromptError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP prompts registered by the theme service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMcpPrompts {
    /// Guide for theme management: get current theme, set theme, list themes.
    ThemeGuide,
}

impl AsRef<str> for ThemeMcpPrompts {
    fn as_ref(&self) -> &str {
        match self {
            Self::ThemeGuide => "theme_guide",
        }
    }
}

impl FromStr for ThemeMcpPrompts {
    type Err = UnknownPromptError;

    fn from_str(prompt: &str) -> Result<Self, Self::Err> {
        match prompt {
            "theme_guide" => Ok(Self::ThemeGuide),
            _ => Err(UnknownPromptError::new(prompt)),
        }
    }
}

impl Display for ThemeMcpPrompts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
```

### 3.11 MCP Request Args

```rust
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Arguments for the `set_theme` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SetThemeArgs {
    /// Name of the theme to select and apply.
    pub name: String,
}
```

### 3.12 JSON Converters

The model crate uses the `impl_json_convertible!` macro for FFI serialization. Manual `parse_*` functions are forbidden. All structs must derive `Default` for
deserialization fallback.

In `lib.rs`:

```rust
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::impl_json_convertible;

impl_json_convertible!(ThemeStatusMessageConverter, ThemeStatusMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

impl_json_convertible!(ThemeCommandMessageConverter, ThemeCommandMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

/// Register all JSON converter implementations for theme messages.
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    ThemeStatusMessageConverter::register_in_host(context);
    ThemeCommandMessageConverter::register_in_host(context);
}
```

All FFI-relevant types carry `#[stabby::stabby]`. The `stabby` dependency must include the `serde` feature
(`stabby = { workspace = true, features = ["serde"] }`).

### 3.13 Model Crate `Cargo.toml`

```toml
[package]
name = "smearor_theme_model"
edition = "2024"

[dependencies]
schemars = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
stabby = { workspace = true, features = ["serde"] }
smearor-model-mcp = { path = "../../model/mcp" }
smearor-personalization-model = { path = "../personalization" }
smearor_swipe_launcher_plugin_api = { path = "../../plugin-api" }
```

> **Note**: `smearor-personalization-model` is needed for `ThemeMode::resolve()` which maps `ColorScheme` to `ThemeMode`. This is a model-to-model dependency
> (no service crate involved), keeping the resolution logic co-located with the type definition.

### 3.14 Model Crate File Structure

```
model/theme/
├── Cargo.toml
└── src/
    ├── lib.rs              # Re-exports, impl_json_convertible!, register_json_converters
    ├── topics.rs           # TOPIC_STATUS, TOPIC_COMMAND
    ├── messages/
    │   ├── mod.rs          # Module declarations
    │   ├── theme.rs        # Theme struct
    │   ├── theme_colors.rs # ThemePalette struct + ThemeColors (dark/light) + to_css() + default palette
    │   ├── theme_mode.rs   # ThemeMode enum + resolve()
    │   ├── theme_info.rs   # ThemeInfo struct (FFI-safe)
    │   ├── status.rs       # ThemeStatusMessage struct (FFI-safe)
    │   └── command.rs      # ThemeCommandMessage, ThemeCommandAction (FFI-safe)
    ├── mcp/
    │   ├── mod.rs          # Module declarations
    │   ├── tools.rs        # ThemeMcpTools enum
    │   ├── prompts.rs      # ThemeMcpPrompts enum
    │   └── requests.rs     # SetThemeArgs struct
    └── view.rs             # ThemeView enum (deprecated, retained for backward compat)
```

---

## 4. Service Crate (`services/theme`)

### 4.1 Overview

The Theme Service is a singleton background service that loads theme definitions from `themes.toml`, applies CSS providers to the GTK display, optionally
couples with the Wallpaper service via the message broker, and reacts to Personalization service color scheme changes for System-mode themes.

The service does **not** use D-Bus. It operates entirely within the launcher process, reading config files and applying CSS via GTK's `CssProvider` API. This
makes it lightweight compared to services like Bluetooth or Network.

### 4.2 CSS Application

When a theme is applied, the service:

1. **Removes old CSS providers**: Any previously applied theme CSS providers (both file-based and variable-based) are removed from the GTK display via
   `style_context_remove_provider_for_display()`.
2. **Injects CSS custom properties**: The service resolves the effective mode (Dark or Light) and calls `ThemeColors::to_css(mode)` to generate a
   `:root { --theme-color-1: ...; ... }` CSS block using the matching palette. This is loaded via `CssProvider::load_from_data()` and registered at
   `STYLE_PROVIDER_PRIORITY_USER + 2` so that theme CSS files can reference `var(--theme-color-1)` through `var(--theme-color-5)`.
3. **Loads new CSS files**: The service selects the CSS file list matching the effective mode (`css_files_dark` for Dark, `css_files_light` for Light). If
   `css_files_light` is empty, `css_files_dark` is used as fallback. For each file path, a `CssProvider` is created via `CssProvider::load_from_path()`. Paths
   are expanded with `shellexpand::tilde()`. CSS files can use `var(--theme-color-1)` through `var(--theme-color-5)` to reference the theme's colors.
4. **Registers at `STYLE_PROVIDER_PRIORITY_USER + 2`**: This places theme CSS (both variables and file-based rules) above instance CSS (`USER + 1`), global user
   CSS (`USER`), per-widget scoped CSS (`APPLICATION + 2`), global scaled CSS (`APPLICATION + 1`), and built-in CSS (`APPLICATION`).
5. **Stores provider handles**: All applied providers (variable provider + file providers) are stored in `ThemeState` so they can be removed on the next theme
   switch.

For **System mode** themes, the service selects the CSS file list based on the effective mode:

- Dark: `css_files_dark`
- Light: `css_files_light` (falls back to `css_files_dark` if empty)

If the personalization service reports `ColorScheme::System` (unresolved), the service defaults to `css_files_dark` and the dark color palette.

CSS application must happen on the GTK main thread. Since the service runs on a separate `std::thread`, `glib::MainContext::default()` would refer to the worker
thread's context, not the GTK main thread. Use `glib::idle_add_once()` to dispatch closures to the GTK main loop:

```rust
let display = gdk::Display::default().expect("No display found");
glib::idle_add_once(move || {
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_USER + 2,
    );
});
```

This ensures `CssProvider` addition/removal runs on the thread that owns the `GdkDisplay`.

### 4.3 Service Struct

```rust
/// Internal command enum for the service event loop.
pub enum ThemeCommand {
    /// Select a theme by name (does not apply).
    SelectTheme(String),
    /// Apply the currently selected theme.
    ApplySelected,
    /// Select a theme by name and apply it immediately.
    SelectAndApply(String),
    /// Refresh status and re-broadcast.
    Refresh,
    /// Add a new theme to the configuration.
    AddTheme(Theme),
    /// Remove a theme from the configuration by name.
    RemoveTheme(String),
    /// Personalization color scheme changed — re-evaluate System mode themes.
    ColorSchemeChanged(ColorScheme),
}

pub struct ThemeService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: ThemeServiceConfig,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<ThemeCommand>,
    pub command_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<ThemeCommand>>,
    pub state: Arc<RwLock<ThemeState>>,
}
```

The service implements the following traits:

- `ServicePlugin` — provides `on_message` (dispatches `FfiEnvelope` to typed `MessageHandler`) and `start` (spawns async runtime)
- `MessageHandler<FfiEnvelopePayload<ThemeCommandMessage>>` — converts incoming command messages to internal `ThemeCommand` enum
- `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` — listens to personalization status for System mode resolution
- `MessageBroadcaster` — empty impl for broadcasting messages
- `MessageTopicBroadcaster<ThemeStatusMessage>` — for broadcasting status on `TOPIC_STATUS`
- `PluginMetaGetter` — returns `self.meta.clone()`
- `AsRef<Option<FfiCoreContext>>` — returns `&self.core_context`
- `McpCapabilitiesRegistrator` — registers MCP tools, resources, and prompts (see Section 4.8)
- `MessageHandler<FfiEnvelopePayload<InvokeToolMessage>>` — handles MCP tool invocations
- `MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>>` — handles MCP resource reads
- `MessageHandler<FfiEnvelopePayload<InvokePromptMessage>>` — handles MCP prompt invocations
- `AcceptTopic<FfiEnvelope>` — filters relevant topics in `on_message`

### 4.4 Service State

```rust
/// Runtime state of the theme service.
pub struct ThemeState {
    /// All configured themes loaded from themes.toml.
    pub themes: Vec<Theme>,
    /// Index of the currently selected theme.
    pub selected_theme_index: usize,
    /// Name of the currently applied theme.
    pub current_theme: Option<String>,
    /// Currently applied CSS providers (variable provider + file providers, for removal on next switch).
    pub applied_providers: Vec<CssProvider>,
    /// Current effective mode (Dark or Light after System resolution).
    pub effective_mode: ThemeMode,
    /// Latest personalization color scheme (for System mode resolution).
    pub system_color_scheme: ColorScheme,
}
```

### 4.5 Service Config

```rust
/// Configuration for the theme service.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ThemeServiceConfig {
    /// List of all configured themes.
    /// Loaded from `config_path` (themes.toml) at startup, not from services.toml.
    #[serde(default)]
    pub themes: Vec<Theme>,
    /// Name of the default theme to apply on startup.
    #[serde(default)]
    pub default_theme: String,
    /// Path to the configuration file where themes are persisted.
    /// If empty, the host resolves the path via config discovery.
    #[serde(default)]
    pub config_path: String,
    /// Whether to automatically apply the default theme on service initialization.
    #[serde(default)]
    pub auto_apply: bool,
    /// Whether System-mode themes should react to personalization color scheme changes.
    /// When true, the service re-applies CSS when ColorScheme changes.
    #[serde(default = "default_true")]
    pub follow_system_color_scheme: bool,
}

fn default_true() -> bool {
    true
}
```

Config file discovery follows the same pattern as `WallpaperServiceConfig`:

1. Working directory → `themes.toml`
2. `~/.config/smearor/services/themes.toml`
3. `/usr/share/smearor/services/themes.toml`

### 4.6 Async Loop

The async loop uses `tokio::select!` with the command channel. There are no external signal streams — the service is entirely event-driven via the message
broker.

```rust
async fn run_theme_async(
    meta: PluginMeta,
    core_context: FfiCoreContext,
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<ThemeCommand>,
    config: ThemeServiceConfig,
    state: Arc<RwLock<ThemeState>>,
) {
    // Initial: load themes from config, apply default if auto_apply is set
    {
        let mut state_guard = state.write().await;
        state_guard.themes = config.load_or_discover_themes();
        if config.auto_apply && !config.default_theme.is_empty() {
            // Select default theme
            if let Some(index) = state_guard.themes.iter().position(|t| t.name == config.default_theme) {
                state_guard.selected_theme_index = index;
            }
        }
    }

    // Apply default theme on startup (on GTK main thread)
    apply_selected_theme(&meta, &core_context, &state).await;

    // Broadcast initial status
    broadcast_status(&meta, &core_context, &state).await;

    loop {
        tokio::select! {
            // Command channel: handle incoming commands from the message broker
            Some(cmd) = command_receiver.recv() => {
                match cmd {
                    ThemeCommand::SelectTheme(name) => { /* update selected index */ }
                    ThemeCommand::ApplySelected => {
                        apply_selected_theme(&meta, &core_context, &state).await;
                        broadcast_status(&meta, &core_context, &state).await;
                    }
                    ThemeCommand::SelectAndApply(name) => {
                        // Select + apply
                        apply_selected_theme(&meta, &core_context, &state).await;
                        broadcast_status(&meta, &core_context, &state).await;
                    }
                    ThemeCommand::Refresh => {
                        broadcast_status(&meta, &core_context, &state).await;
                    }
                    ThemeCommand::AddTheme(theme) => { /* add + persist */ }
                    ThemeCommand::RemoveTheme(name) => { /* remove + persist */ }
                    ThemeCommand::ColorSchemeChanged(scheme) => {
                        // Only re-apply if current theme is System mode
                        let needs_reapply = {
                            let state_guard = state.read().await;
                            state_guard.system_color_scheme != scheme
                                && state_guard.themes.get(state_guard.selected_theme_index)
                                    .map(|t| t.mode == ThemeMode::System)
                                    .unwrap_or(false)
                        };
                        if needs_reapply {
                            {
                                let mut state_guard = state.write().await;
                                state_guard.system_color_scheme = scheme;
                            }
                            apply_selected_theme(&meta, &core_context, &state).await;
                            broadcast_status(&meta, &core_context, &state).await;
                        }
                    }
                }
            }
        }
    }
}
```

### 4.7 Wallpaper Coupling

When applying a theme that has `wallpaper_theme` set, the service sends a `WallpaperCommandMessage` to the wallpaper service via the message broker:

1. **Select wallpaper theme**: Broadcast `WallpaperCommandMessage { action: SelectTheme, name: <wallpaper_theme> }` on `service.wallpaper.command`.
2. **Start wallpaper process**: Broadcast `WallpaperCommandMessage { action: StartSelected }` on `service.wallpaper.command`.

This is a **one-directional coordination**: Theme Service broadcasts commands, Wallpaper Service reacts. No changes to the Wallpaper Service's command interface
are needed — it already supports `SelectTheme` and `StartSelected` actions.

The coupling is **optional** — themes without `wallpaper_theme` only change CSS. This keeps the system flexible.

```rust
fn send_wallpaper_coupling(core_context: &FfiCoreContext, meta: &PluginMeta, wallpaper_theme_name: &str) {
    // Select the wallpaper theme
    let select_command = WallpaperCommandMessage {
        action: WallpaperCommandAction::SelectTheme,
        name: StabbyOption::Some(StabbyString::from(wallpaper_theme_name)),
    };
    broadcast_to_topic(core_context, meta, WallpaperCommandMessage::topic(), select_command);

    // Start the selected wallpaper process
    let start_command = WallpaperCommandMessage {
        action: WallpaperCommandAction::StartSelected,
        name: StabbyOption::None(),
    };
    broadcast_to_topic(core_context, meta, WallpaperCommandMessage::topic(), start_command);
}
```

### 4.8 MCP Tools

The service implements `McpCapabilitiesRegistrator` to register MCP tools, resources, and prompts. This is called during `start()`.

| Tool Name   | Parameters     | Description                                    |
|-------------|----------------|------------------------------------------------|
| `get_theme` | —              | Get current theme status (applied theme, mode) |
| `set_theme` | `name: String` | Select and apply a theme by name immediately   |

Resources:

| Resource URI     | Description                                           |
|------------------|-------------------------------------------------------|
| `theme://status` | Current theme status including applied theme and mode |
| `theme://themes` | List of all configured themes with metadata           |

Prompts:

| Prompt Name   | Description                                                  |
|---------------|--------------------------------------------------------------|
| `theme_guide` | System prompt with theme management tools and current status |

### 4.9 Service Crate File Structure

```
services/theme/
├── Cargo.toml
├── data/
│   └── prompts/
│       └── theme_guide.md       # Prompt template
└── src/
    ├── lib.rs                   # service_plugin!(ThemeService);
    ├── config.rs                # ThemeServiceConfig, load_or_discover_themes()
    ├── command.rs               # ThemeCommand internal enum
    ├── state.rs                 # ThemeState
    ├── service.rs               # ThemeService struct + trait impls
    └── mcp/
        ├── mod.rs               # Module declarations
        ├── capabilities.rs      # McpCapabilitiesRegistrator impl
        └── handler/
            ├── mod.rs           # Module declarations
            ├── tools.rs         # InvokeToolMessage handler
            ├── resources.rs     # InvokeResourceMessage handler
            └── prompt.rs        # InvokePromptMessage handler
```

### 4.10 Service Crate `Cargo.toml`

```toml
[package]
name = "smearor-service-theme"
edition = "2024"

[dependencies]
gtk4 = { workspace = true }
glib = { workspace = true }
schemars = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
shellexpand = "3"
stabby = { workspace = true, features = ["serde"] }
tokio = { workspace = true, features = ["sync", "rt", "macros"] }
toml = "0.8"
tracing = { workspace = true }
dirs = "5"

smearor-model-mcp = { path = "../../model/mcp" }
smearor-personalization-model = { path = "../../model/personalization" }
smearor-theme-model = { path = "../../model/theme" }
smearor-wallpaper-model = { path = "../../model/wallpaper" }
smearor_swipe_launcher_plugin_api = { path = "../../plugin-api" }
```

---

## 5. Widget Crate (`plugins/theme`)

### 5.1 Widget Struct

The widget follows the same pattern as the Wallpaper widget: **one view per theme**, with swipe navigation and click-to-apply. The unified 4-line layout is
used:

| Line | Height      | Content                                                        |
|------|-------------|----------------------------------------------------------------|
| 0    | `icon_size` | Preview image (`gtk4::Image`) or fallback icon (`gtk4::Image`) |
| 1    | 20px        | `widget-main-text` (theme name)                                |
| 2    | 16px        | `widget-info-text` (mode + ✓ if applied)                      |
| 3    | 16px        | spacer                                                         |

In Compact mode with `icon_only = true`, lines 1–3 retain their `height_request` to preserve icon alignment across widgets.

The widget has two `gtk4::Image` widgets:

- **`preview_image`**: displays the theme's `preview_image_path` as a `Texture` when available (hidden otherwise).
- **`fallback_image`**: displays the theme's `preview_icon` (Nerd Font icon) when no preview image is available.

This mirrors the Wallpaper widget's `preview.rs` pattern.

```rust
pub struct ThemeWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: ThemeWidgetConfig,
    pub preview_image: Rc<RefCell<Option<gtk4::Image>>>,
    pub fallback_image: Rc<RefCell<Option<gtk4::Image>>>,
    pub value_label: Rc<RefCell<Option<Label>>>,
    pub info_label: Rc<RefCell<Option<Label>>>,
    pub latest_status: Rc<RefCell<Option<ThemeStatusMessage>>>,
    pub latest_personalization: Rc<RefCell<Option<PersonalizationStatusMessage>>>,
    pub personalization: Rc<RefCell<PersonalizationOverride>>,
}
```

The widget implements the following traits:

- `WidgetPlugin` — provides `on_message` (dispatches `FfiEnvelope` to typed `MessageHandler`) and `start` (spawns listener task)
- `WidgetBuilder` — provides `build_widget` returning a `gtk4::Box` with icon, labels, and gesture handlers
- `MessageHandler<FfiEnvelopePayload<ThemeStatusMessage>>` — updates `latest_status` and triggers `update_ui`
- `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` — updates `latest_personalization` for locale-aware labels
- `MessageBroadcaster` — for sending command messages to the service
- `MessageTopicBroadcaster<ThemeCommandMessage>` — for broadcasting commands on `TOPIC_COMMAND`
- `MessageTopicBroadcaster<WidgetUpdateMessage>` — for broadcasting widget updates (headless/Web instance sync)
- `PluginMetaGetter` — returns `self.meta.clone()`
- `AsRef<Option<FfiCoreContext>>` — returns `&self.core_context`
- `DefaultFallback` — provides fallback click/longpress/drag behavior for `GestureHandler`
- `AcceptTopic<FfiEnvelope>` — topic filtering for incoming messages (subscribes to `TOPIC_STATUS`, `TOPIC_PERSONALIZATION_STATUS`)
- `GestureHandler` — provides `attach_gesture_handlers` and `DefaultFallback`
- `GraphicRenderer` — for headless instance pixel rendering
- `WebRenderer` — for web instance HTML rendering

### 5.2 Widget Config

```rust
// Default Nerd Font icon names
pub const DEFAULT_ICON_THEME: &str = "nf-md-palette";
pub const DEFAULT_ICON_THEME_DARK: &str = "nf-md-weather_night";
pub const DEFAULT_ICON_THEME_LIGHT: &str = "nf-md-weather_sunny";
pub const DEFAULT_ICON_THEME_SYSTEM: &str = "nf-md-theme_light_dark";
pub const DEFAULT_ICON_NO_THEME: &str = "nf-md-palette_outline";

/// Theme-specific icon configuration.
/// All Nerd Font icon names used by the Theme widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct ThemeIcons {
    /// Icon for a generic theme.
    #[builder(default = DEFAULT_ICON_THEME.to_string())]
    #[serde(default = "default_icon_theme")]
    pub(crate) icon_theme: String,

    /// Icon for dark mode theme.
    #[builder(default = DEFAULT_ICON_THEME_DARK.to_string())]
    #[serde(default = "default_icon_theme_dark")]
    pub(crate) icon_theme_dark: String,

    /// Icon for light mode theme.
    #[builder(default = DEFAULT_ICON_THEME_LIGHT.to_string())]
    #[serde(default = "default_icon_theme_light")]
    pub(crate) icon_theme_light: String,

    /// Icon for system mode theme.
    #[builder(default = DEFAULT_ICON_THEME_SYSTEM.to_string())]
    #[serde(default = "default_icon_theme_system")]
    pub(crate) icon_theme_system: String,

    /// Icon when no theme is applied.
    #[builder(default = DEFAULT_ICON_NO_THEME.to_string())]
    #[serde(default = "default_icon_no_theme")]
    pub(crate) icon_no_theme: String,
}

impl Default for ThemeIcons {
    fn default() -> Self {
        ThemeIcons {
            icon_theme: DEFAULT_ICON_THEME.to_string(),
            icon_theme_dark: DEFAULT_ICON_THEME_DARK.to_string(),
            icon_theme_light: DEFAULT_ICON_THEME_LIGHT.to_string(),
            icon_theme_system: DEFAULT_ICON_THEME_SYSTEM.to_string(),
            icon_no_theme: DEFAULT_ICON_NO_THEME.to_string(),
        }
    }
}

/// Configuration for the Theme widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct ThemeWidgetConfig {
    /// Shared widget dimensions (width, height, max_width, scale).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) dimensions: WidgetDimensions,

    /// Shared widget layout (spacing, css_classes).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) layout: WidgetLayout,

    /// Shared widget icon settings (icon_size, icon_only, icon_color).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) icon: WidgetIcon,

    /// Shared widget text colors (main_text_color, info_text_color).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) text_colors: WidgetTextColors,

    /// Shared widget mode (compact, wide).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) mode: WidgetMode,

    /// Action bindings for click, longpress, drag, and scroll gestures.
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) actions: ActionBindings,

    /// Theme-specific Nerd Font icons.
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) icons: ThemeIcons,
}
```

> **Note**: The `views` field has been removed. The widget now uses one view per theme, with swipe up/down cycling through themes automatically.

### 5.3 View Rendering

`render_view` returns a `ViewData` struct (with `icon_name`, `main_text`, `info_text`), analogous to other widgets. It renders the **currently selected theme**
(based on `selected_theme_index` from the status message), not a fixed set of view variants.

```rust
pub fn render_view(
    status: Option<&ThemeStatusMessage>,
    config: &ThemeWidgetConfig,
    labels: &ThemeLabels,
) -> ViewData {
    let theme_info = status.and_then(|s| s.themes.get(s.selected_theme_index as usize).cloned());

    match theme_info {
        Some(theme) => {
            let is_applied = status
                .and_then(|s| s.current_theme.as_ref().map(|ct| ct.to_string() == theme.name.to_string()))
                .unwrap_or(false);

            let icon = if !theme.preview_icon.is_empty() {
                theme.preview_icon.to_string()
            } else {
                config.icons.icon_theme.clone()
            };

            let mode_text = match theme.mode {
                ThemeMode::Dark => &labels.dark,
                ThemeMode::Light => &labels.light,
                ThemeMode::System => &labels.system,
            };

            let info = if is_applied {
                format!("{} \u{2713}", mode_text)
            } else {
                mode_text.clone()
            };

            ViewData::new(icon, theme.name.to_string(), info)
        }
        None => ViewData::new(config.icons.icon_no_theme.clone(), labels.no_theme.clone(), labels.theme.clone()),
    }
}
```

The `ThemeLabels` struct provides locale-aware labels:

```rust
struct ThemeLabels {
    theme: String,
    themes: String,
    no_theme: String,
    mode: String,
    dark: String,
    light: String,
    system: String,
    applied: String,
}

impl ThemeLabels {
    fn from_personalization(p: Option<&PersonalizationStatusMessage>) -> Self {
        // Use locale from personalization status, fallback to English
    }
}
```

### 5.4 Interaction (Swipe + Click)

The widget implements `DefaultFallback` to provide theme navigation and selection behavior, mirroring the Wallpaper widget:

| Gesture                      | Action                                                          |
|------------------------------|-----------------------------------------------------------------|
| **Swipe Up** / Scroll Up     | `select_next_theme()` — selects next theme without applying     |
| **Swipe Down** / Scroll Down | `select_prev_theme()` — selects previous theme without applying |
| **Click** / Double Press     | `apply_selected_theme()` — applies the currently selected theme |
| **Long-press** / Right Click | `apply_selected_theme()` — applies the currently selected theme |
| **Expand** / ToggleView      | `select_next_theme()` — selects next theme                      |
| **Collapse**                 | `select_prev_theme()` — selects previous theme                  |

**Selection vs Application**: Swipe only **selects** a theme (updates `selected_theme_index` via `ThemeCommandMessage::select_theme()`). Click **applies** the
selected theme (via `ThemeCommandMessage::apply_selected()`). This separation allows the user to browse themes before committing.

The `is_applied` indicator (✓) is shown in the info text when the currently selected theme is also the applied theme.

**Long-press fallback** (when no `longpress` action is configured in `ActionBindings`): applies the selected theme.

### 5.5 Multi-Instance Rendering (Headless / Web)

The widget supports all three instance types (GTK, Headless, Web) by implementing the rendering traits:

- **GTK** (`InstanceType::Gtk`): `WidgetBuilder::build_widget()` produces a `gtk4::Box` with icon, labels, and gesture handlers. Nerd Font icon names are
  resolved via `resolve_gtk_nerd_icon()` to GResource SVGs.
- **Headless** (`InstanceType::Headless`): `GraphicRenderer::render_graphic(w, h)` produces a raw RGBA pixel buffer via `image` + `ab_glyph`. Nerd Font icon
  names are resolved via `resolve_icon_codepoint()` to Unicode codepoints.
- **Web** (`InstanceType::Web`): `WebRenderer::render_html(instance_id, plugin_id)` produces an HTML fragment with inline styles.

All three pipelines use the same `render_view` function, ensuring consistent output across instance types.

### 5.6 Widget Crate File Structure

```
plugins/theme/
├── Cargo.toml
└── src/
    ├── lib.rs                   # widget_plugin_graphic!(ThemeWidget);
    ├── config.rs                # ThemeWidgetConfig, ThemeIcons
    ├── widget.rs                # ThemeWidget struct + trait impls + render_view()
    ├── preview.rs               # update_preview() — preview image loading with fallback icon
    ├── personalization.rs       # PersonalizationOverride
    ├── labels.rs                # ThemeLabel enum + ThemeLabels struct
    ├── graphic.rs               # GraphicRenderer impl
    ├── html.rs                  # WebRenderer impl
    └── mcp/
        ├── mod.rs               # Module declarations
        ├── capabilities.rs      # McpCapabilitiesRegistrator impl
        └── handler/
            ├── mod.rs           # Module declarations
            └── tools.rs         # InvokeToolMessage handler
```

### 5.7 Widget Crate `Cargo.toml`

```toml
[package]
name = "smearor-plugin-theme"
edition = "2024"

[dependencies]
gtk4 = { workspace = true }
glib = { workspace = true }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
stabby = { workspace = true, features = ["serde"] }
tokio = { workspace = true, features = ["sync"] }
tracing = { workspace = true }
typed-builder = { workspace = true }

smearor-personalization-model = { path = "../../model/personalization" }
smearor-theme-model = { path = "../../model/theme" }
smearor_swipe_launcher_plugin_api = { path = "../../plugin-api" }
```

---

## 6. Cross-Service Coordination

### 6.1 Theme ↔ Personalization

When the personalization service broadcasts a `PersonalizationStatusMessage` with a changed `ColorScheme`, the theme service re-evaluates the currently applied
theme:

1. **Personalization Service** broadcasts `PersonalizationStatusMessage` on `service.personalization.status`.
2. **Theme Service** subscribes via `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` and `AcceptTopic<FfiEnvelope>`.
3. If the current theme's mode is `System` and `follow_system_color_scheme` is `true`, the service re-applies the CSS (selecting dark or light CSS file based on
   the new `ColorScheme`).
4. **Theme Service** broadcasts updated `ThemeStatusMessage` with the new `effective_mode`.

This is a **one-directional coordination**: Personalization Service broadcasts status, Theme Service reacts.

### 6.2 Theme ↔ Wallpaper

When a theme with `wallpaper_theme` set is applied:

1. **Theme Service** broadcasts `WallpaperCommandMessage { action: SelectTheme, name: <wallpaper_theme> }` on `service.wallpaper.command`.
2. **Theme Service** broadcasts `WallpaperCommandMessage { action: StartSelected }` on `service.wallpaper.command`.
3. **Wallpaper Service** handles these commands via its existing `MessageHandler<FfiEnvelopePayload<WallpaperCommandMessage>>`.
4. **Wallpaper Service** broadcasts `WallpaperStatusMessage` on `service.wallpaper.status` with the new running theme.

This is a **one-directional coordination**: Theme Service broadcasts commands, Wallpaper Service reacts. No changes to the Wallpaper Service are needed.

### 6.3 Coordination Diagram

```
Personalization Service ──(PersonalizationStatusMessage)──> Theme Service
                                                              │
                                                              ├── CSS re-application (System mode)
                                                              │
                                                              ├──(WallpaperCommandMessage)──> Wallpaper Service
                                                              │                                   │
                                                              │                                   └──(WallpaperStatusMessage)──> Wallpaper Widget
                                                              │
                                                              └──(ThemeStatusMessage)──> Theme Widget
```

---

## 7. Config Integration

### 7.1 Service Config (`configs/services/themes.toml`)

```toml
# Theme service configuration
# Themes are defined here and loaded at startup.

# The built-in default theme uses the official Smearor design palette.
# Colors default to the values below; omit the [themes.colors] table
# to use the same defaults for both dark and light modes.
[[themes]]
name = "default"
description = "Default Smearor theme with the official design palette"
mode = "System"
css_files_dark = ["~/.config/smearor/themes/default-dark.css"]
css_files_light = ["~/.config/smearor/themes/default-light.css"]
preview_icon = "nf-md-palette"
preview_image_path = ""  # Optional: path to a preview image (e.g. "~/.config/smearor/themes/default-preview.png")

# Both dark and light palettes default to the Smearor design palette.
# Override either one to customize per mode:
# [themes.colors.dark]
# color_1 = "#04e762ff"  # malachite
# color_2 = "#f5b700ff"  # selective-yellow
# color_3 = "#00a1e4ff"  # celestial-blue
# color_4 = "#dc0073ff"  # mexican-pink
# color_5 = "#89fc00ff"  # chartreuse

# [themes.colors.light]
# color_1 = "#04e762ff"  # malachite
# color_2 = "#f5b700ff"  # selective-yellow
# color_3 = "#00a1e4ff"  # celestial-blue
# color_4 = "#dc0073ff"  # mexican-pink
# color_5 = "#89fc00ff"  # chartreuse

[[themes]]
name = "Halloween"
description = "Spooky Halloween theme with coupled wallpaper"
mode = "Dark"
css_files_dark = ["~/.config/smearor/themes/halloween.css"]
preview_icon = "nf-md-ghost"
preview_image_path = ""  # Optional: path to a preview image (e.g. "~/.config/smearor/themes/halloween-preview.png")

[themes.colors.dark]
color_1 = "#ff6b00ff"  # pumpkin orange
color_2 = "#8b00ffff"  # witch purple
color_3 = "#00ff00ff"  # toxic green
color_4 = "#ff0000ff"  # blood red
color_5 = "#fff200ff"  # moon yellow

wallpaper_theme = "Halloween Pumpkins"
```

> **Note**: The `default` theme's colors match the official palette in `docs/DESIGN.md`. Themes that omit the `[themes.colors]` table get the default palette
> for both dark and light modes via `ThemeColors::default()`. For Dark-only or Light-only themes, only the relevant palette needs to be set. CSS files can
> reference these colors via `var(--theme-color-1)` through `var(--theme-color-5)`.

### 7.2 Service Registration (`configs/services/services.toml`)

```toml
[[services]]
id = "theme"
type = "theme"

[services.config]
default_theme = "default"
auto_apply = true
follow_system_color_scheme = true
# config_path is injected by the host via config discovery
```

### 7.3 Widget Config (in `config.toml` or area config)

```toml
[[plugins]]
plugin_id = "theme_widget"
display_name = "Theme"
icon_name = "nf-md-palette"
width = 100
height = 100
icon_size = 36
# scale = 1.0  # optional per-widget override of global [launcher] scale
# Note: views field has been removed. The widget now uses one view per theme.
# Swipe up/down cycles through themes, click applies the selected theme.

[plugins.config.icons]
icon_theme = "nf-md-palette"
icon_theme_dark = "nf-md-weather_night"
icon_theme_light = "nf-md-weather_sunny"
icon_theme_system = "nf-md-theme_light_dark"
icon_no_theme = "nf-md-palette_outline"

[plugins.config.actions]
longpress = { topic = "area.open", payload = { area_id = "theme_area" } }
```

### 7.4 Theme Area (`configs/areas/scroll_menu.toml`)

The `theme_area` is a scroll menu area that provides quick theme switching. Each theme is rendered as a button tile with a preview icon, theme name, and mode
indicator.

```toml
# Theme area tiles are generated dynamically by the Theme Widget
# based on the latest status. Each tile shows:
# - Theme preview icon
# - Theme name as main_text
# - Mode (Dark/Light/System) as info_text
# - Click action: SelectAndApply(theme_name)
```

---

## 8. Implementation Phases

### Phase 1: Model Crate (`model/theme`)

**Order:** First — no dependencies (except `smearor-personalization-model` for `ThemeMode::resolve()`).

**Tasks:**

- Create `model/theme/Cargo.toml` with `stabby` (with `serde` feature), `serde`, `serde_json`, `schemars`, `smearor-model-mcp`, `smearor-personalization-model`
  dependencies
- Implement `topics.rs` with `TOPIC_STATUS`, `TOPIC_COMMAND` constants
- Implement `messages/theme.rs` with `Theme` struct (includes `colors: ThemeColors` field)
- Implement `messages/theme_colors.rs` with `ThemePalette` struct (5 colors, `to_css()`, default palette functions, `Default` impl) and `ThemeColors` struct
  (dark + light `ThemePalette`, `palette_for_mode()`, `to_css(mode)`, `Default` impl)
- Implement `messages/theme_mode.rs` with `ThemeMode` enum, `FromStr`, and `resolve()` method
- Implement `messages/theme_info.rs` with `ThemeInfo` struct (FFI-safe, `#[stabby::stabby]`)
- Implement `messages/status.rs` with `ThemeStatusMessage` struct (FFI-safe, `#[stabby::stabby]`)
- Implement `messages/command.rs` with `ThemeCommandMessage`, `ThemeCommandAction` (FFI-safe, `#[stabby::stabby]`)
- Implement `view.rs` with `ThemeView` enum
- Implement `mcp/tools.rs` with `ThemeMcpTools` enum (`AsRef<str>`, `FromStr`, `Display`)
- Implement `mcp/prompts.rs` with `ThemeMcpPrompts` enum (`AsRef<str>`, `FromStr`, `Display`)
- Implement `mcp/requests.rs` with `SetThemeArgs` struct (`JsonSchema`)
- Implement `lib.rs` with `pub use` re-exports
- Add `#[stabby::stabby]` to all FFI-relevant types with fields sorted by descending alignment
- Use `impl_json_convertible!` macro invocations with `serde_json::from_value(json).unwrap_or_default()`
- Implement `register_json_converters(context)` function calling `Converter::register_in_host(context)`
- No manual `parse_*` functions — use `impl_json_convertible!` only

**Exit Criteria:** `cargo build -p smearor_theme_model` succeeds.

### Phase 2: Service Crate (`services/theme`)

**Order:** Second — depends on Phase 1.

**Tasks:**

- Create `services/theme/Cargo.toml` with `gtk4`, `glib`, `tokio`, `tracing`, `serde`, `serde_json`, `toml`, `shellexpand`, `dirs`, `schemars`, `plugin-api`,
  `model/theme`, `model/personalization`, `model/wallpaper`, `model/mcp` dependencies
- Implement `config.rs` with `ThemeServiceConfig` struct, `#[serde(default)]` on all fields, `load_or_discover_themes()` method (analogous to
  `WallpaperServiceConfig`)
- Implement `command.rs` with `ThemeCommand` internal enum
- Implement `state.rs` with `ThemeState` struct (themes, selected index, current theme, applied providers, effective mode, system color scheme)
- Implement `service.rs` with `ThemeService` struct
- Implement `ServicePlugin` trait (`on_message`, `start`)
- Implement `MessageHandler<FfiEnvelopePayload<ThemeCommandMessage>>` trait — dispatches commands to internal `ThemeCommand` enum
- Implement `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` trait — sends `ColorSchemeChanged` to command channel
- Implement `MessageBroadcaster` trait
- Implement `MessageTopicBroadcaster<ThemeStatusMessage>` trait
- Implement `PluginMetaGetter`, `AsRef<Option<FfiCoreContext>>` traits
- Implement `McpCapabilitiesRegistrator` trait — register tools (`get_theme`, `set_theme`), resources (`theme://status`, `theme://themes`), prompt
  (`theme_guide`)
- Implement `MessageHandler<FfiEnvelopePayload<InvokeToolMessage>>` — handle `get_theme` and `set_theme` tool invocations
- Implement `MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>>` — handle resource reads
- Implement `MessageHandler<FfiEnvelopePayload<InvokePromptMessage>>` — handle `theme_guide` prompt
- Implement `AcceptTopic<FfiEnvelope>` trait for topic filtering
- Implement `new(config, core_context)` constructor with `register_json_converters` call
- Implement `start()` with `std::thread::spawn` + `tokio::runtime::Builder::new_current_thread().enable_all()` + `LocalSet`
- Implement `run_theme_async` with `tokio::select!` for command channel
- Implement `apply_selected_theme` — remove old CSS providers, resolve effective mode, inject CSS custom properties via `ThemeColors::to_css(mode)` +
  `CssProvider::load_from_data()`, load CSS files from `css_files_dark` or `css_files_light` based on effective mode (fallback to dark if light is empty),
  register all at `STYLE_PROVIDER_PRIORITY_USER + 2` on GTK main thread
- Implement `send_wallpaper_coupling` — broadcast `WallpaperCommandMessage` for wallpaper-coupled themes
- Implement `broadcast_status` — build and broadcast `ThemeStatusMessage`
- Implement CSS file path expansion with `shellexpand::tilde()`
- Implement config file discovery (working dir → XDG config → system path)
- Use `service_plugin!(ThemeService);` macro in `lib.rs`
- Use `tokio::sync::mpsc::unbounded_channel` for command channel
- Use `glib::idle_add_once()` for GTK CSS application (dispatch from worker thread to GTK main thread)
- No `unwrap()` or `expect()` in production code
- No polling loops; use event-driven `recv().await`

**Exit Criteria:** `cargo build -p smearor-service-theme` succeeds. Service loads themes from `themes.toml`, applies CSS, and broadcasts status.

### Phase 3: Widget Crate (`plugins/theme`)

**Order:** Third — depends on Phase 1 and Phase 2.

**Tasks:**

- Create `plugins/theme/Cargo.toml` with `gtk4`, `glib`, `serde`, `serde_json`, `stabby`, `tokio`, `tracing`, `typed-builder`, `plugin-api`, `model/theme`,
  `model/personalization` dependencies
- Implement `config.rs` with `ThemeWidgetConfig` struct using shared config structs (`WidgetDimensions`, `WidgetLayout`, `WidgetIcon`, `WidgetTextColors`,
  `WidgetMode`) via `#[serde(flatten)]`
- Implement `ThemeIcons` struct with all theme-specific icon fields and `Default` impl, used via `#[serde(flatten)]` in `ThemeWidgetConfig`
- Use `ActionBindings` via `#[serde(flatten)]` for gesture bindings
- Support `BindingMode` (`replace`/`supplement`) per binding
- Implement `widget.rs` with `ThemeWidget` struct
- Implement `WidgetPlugin` trait (`on_message`, `start`)
- Implement `WidgetBuilder` trait (`build_widget`)
- Implement `MessageHandler<FfiEnvelopePayload<ThemeStatusMessage>>` trait
- Implement `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` trait for locale-aware labels
- Implement `MessageBroadcaster` trait
- Implement `MessageTopicBroadcaster<ThemeCommandMessage>` trait
- Implement `MessageTopicBroadcaster<WidgetUpdateMessage>` trait for headless/Web instance sync
- Implement `PluginMetaGetter`, `AsRef<Option<FfiCoreContext>>` traits
- Implement `DefaultFallback` trait for theme navigation (swipe = select, click = apply)
- Implement `AcceptTopic<FfiEnvelope>` trait for topic filtering
- Implement `GestureHandler` trait and call `attach_gesture_handlers` in `build_widget`
- Implement `render_view` returning `ViewData` for the currently selected theme (no `ThemeView` parameter)
- Implement `GraphicRenderer::render_graphic` for headless instance pixel rendering
- Implement `WebRenderer::render_html` for web instance HTML fragment rendering
- Use `resolve_gtk_nerd_icon()` for GTK icon resolution and `resolve_icon_codepoint()` for pixel/atomic rendering
- Implement `ThemeLabels` struct for locale-aware labels
- Implement `preview.rs` with `update_preview()` for preview image loading with fallback icon (analogous to Wallpaper widget)
- Implement `update_ui` with `glib::idle_add_once()` for GTK updates (dispatch from worker thread to GTK main thread)
- Implement `broadcast_widget_update` after every UI update
- Implement `start_listeners` subscribing to `TOPIC_STATUS` and `TOPIC_PERSONALIZATION_STATUS`
- Use `glib::idle_add_once()` for GTK updates
- Use `tokio::sync::mpsc` for message reception
- Use `widget_plugin_graphic!(ThemeWidget);` macro in `lib.rs`
- No polling loops (`timeout_add_local`); use event-driven `recv().await`
- No `unwrap()` or `expect()` in production code

**Exit Criteria:** `cargo build -p smearor-plugin-theme` succeeds. Widget displays one view per theme with preview image or fallback icon, swipe cycles themes,
click applies the selected theme.

### Phase 4: Cross-Service Coordination

**Order:** Fourth — depends on Phase 2 and Phase 3.

**Tasks:**

- Theme Service subscribes to `service.personalization.status` for `PersonalizationStatusMessage`
- Theme Service sends `ColorSchemeChanged` command when personalization status updates
- Theme Service re-applies CSS for System-mode themes when color scheme changes
- Theme Service broadcasts `WallpaperCommandMessage` on `service.wallpaper.command` when applying wallpaper-coupled themes
- Verify Wallpaper Service handles `SelectTheme` and `StartSelected` commands from Theme Service

**Exit Criteria:** Changing system color scheme re-applies CSS for System-mode themes. Applying a wallpaper-coupled theme also switches the wallpaper.

### Phase 5: Workspace Wiring

**Order:** Fifth — depends on all previous phases.

**Tasks:**

- Add `model/theme`, `services/theme`, `plugins/theme` to workspace `Cargo.toml` members
- Add service loading to launcher service discovery
- Add plugin loading to launcher plugin discovery
- Add default config entries to `configs/services/services.toml`
- Add `theme_widget` to default launcher config
- Add `theme_area` to area configuration
- Add `smearor-plugin-theme` and `smearor-service-theme` to `packages/full/Cargo.toml` metapackage `depends` list
- Create default `configs/services/themes.toml` with example themes

**Exit Criteria:** Launcher starts with Theme service and widget loaded. `config.toml` contains theme entries. Metapackage includes the new crates.

### Phase 6: Integration and Tests

**Order:** Sixth — depends on all previous phases.

**Tasks:**

- Verify theme switching works (CSS changes applied, CSS variables injected, icon and labels update)
- Verify CSS custom properties (`--theme-color-1` through `--theme-color-5`) are injected with the correct palette for the effective mode (Dark or Light)
- Verify the `default` theme uses the official Smearor design palette colors for both dark and light modes
- Verify themes with custom dark/light palettes override the defaults correctly per mode
- Verify System-mode themes with a single CSS file adapt colors correctly when the system color scheme changes
- Verify System-mode theme reacts to personalization color scheme changes
- Verify wallpaper coupling: applying a theme with `wallpaper_theme` also switches wallpaper
- Verify theme without `wallpaper_theme` only changes CSS (no wallpaper effect)
- Verify swipe up/down cycles through themes (selects without applying)
- Verify click applies the currently selected theme
- Verify preview image is displayed when `preview_image_path` is set, falls back to Nerd Font icon otherwise
- Verify ✓ indicator appears in info text when the selected theme is also the applied theme
- Verify long-press opens `theme_area`
- Verify MCP `get_theme` tool returns current status
- Verify MCP `set_theme` tool selects and applies a theme by name
- Verify MCP `theme_guide` prompt returns status snapshot
- Test with empty `themes.toml` (graceful degradation — no themes, no crash)
- Test with missing CSS files (logged error, no crash)
- Test with System mode but no personalization service running (fallback to dark CSS)
- Test config parsing with partial TOML (defaults applied)
- Test adding/removing themes via MCP tools
- Verify `theme_area` tiles appear for all configured themes and apply on click
- No `unwrap()` or `expect()` in production code paths

**Exit Criteria:** All tests pass. Theme widget is fully functional.

### Phase 7: Documentation

**Order:** Seventh — depends on all previous phases.

**Tasks:**

- Update `book/src/SUMMARY.md` with Theme-related chapters
- Add `book/src/features/theme.md` describing the Theme widget, views, and configuration
- Add `book/src/architecture/theme.md` describing the service architecture, CSS application, and personalization/wallpaper integration
- Update `book/src/configuration/` with Theme service and widget config examples
- Update `README.md` feature list to include Theme widget and service
- Document `themes.toml` format and available fields
- Document wallpaper coupling in the book
- Document System mode and personalization integration in the book

**Exit Criteria:** `mdbook build` succeeds. README.md lists Theme as a feature. Book contains Theme documentation.

---

## 9. Dependencies

| Crate            | Dependencies                                                                                                                                                                               |
|------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `model/theme`    | `stabby` (with `serde` feature), `serde`, `serde_json`, `schemars`, `smearor-model-mcp`, `smearor-personalization-model`, `plugin-api`                                                     |
| `services/theme` | `gtk4`, `glib`, `tokio`, `tracing`, `serde`, `serde_json`, `toml`, `shellexpand`, `dirs`, `schemars`, `plugin-api`, `model/theme`, `model/personalization`, `model/wallpaper`, `model/mcp` |
| `plugins/theme`  | `gtk4`, `glib`, `serde`, `serde_json`, `stabby`, `tokio`, `tracing`, `typed-builder`, `plugin-api`, `model/theme`, `model/personalization`                                                 |

---

## 10. Error Handling

- Missing `themes.toml`: service starts with empty themes list, broadcasts status with `current_theme: None`
- Missing CSS file: logged via `warn!`, theme application skipped, status still updated
- Invalid TOML: logged via `warn!`, falls back to empty themes list
- No personalization service running: System mode defaults to dark CSS
- No wallpaper service running: wallpaper coupling commands are silently ignored (message broker discards unmatched messages)
- GTK display not available: CSS application skipped, logged via `debug!`
- No `unwrap()` or `expect()` in production code
- Graceful degradation when theme service is not loaded (widget shows "No Theme")

---

## 11. Icon Reference

| Icon Name           | Nerd Font Icon           | Usage                                           |
|---------------------|--------------------------|-------------------------------------------------|
| `icon_theme`        | `nf-md-palette`          | Fallback icon for themes without `preview_icon` |
| `icon_theme_dark`   | `nf-md-weather_night`    | Dark mode theme (reserved)                      |
| `icon_theme_light`  | `nf-md-weather_sunny`    | Light mode theme (reserved)                     |
| `icon_theme_system` | `nf-md-theme_light_dark` | System mode theme (reserved)                    |
| `icon_no_theme`     | `nf-md-palette_outline`  | No theme available / fallback                   |

---

## 12. Personalization Integration

The Theme widget subscribes to `TOPIC_PERSONALIZATION_STATUS` (from the Personalization service) to receive locale updates. When a
`PersonalizationStatusMessage` arrives, the widget stores it in `latest_personalization` and triggers a UI re-render.

The `ThemeLabel` struct uses the locale from `PersonalizationStatusMessage` to select appropriate label strings for all view text. This is analogous to
`NetworkLabel` in the Network widget.

The Theme **Service** also subscribes to `TOPIC_PERSONALIZATION_STATUS` to react to color scheme changes for System-mode themes. When the color scheme changes,
the service re-applies the CSS (selecting the appropriate dark/light CSS file).

Both the widget and the service implement `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` and `AcceptTopic<FfiEnvelope>` filtering for
`TOPIC_PERSONALIZATION_STATUS`.

---

## 13. CSS Provider Priority

Theme CSS is registered at `STYLE_PROVIDER_PRIORITY_USER + 2`, placing it above all other CSS sources:

| Priority                              | Source                | Owner                       |
|---------------------------------------|-----------------------|-----------------------------|
| `STYLE_PROVIDER_PRIORITY_APPLICATION` | Built-in `style.css`  | `create_css_provider()`     |
| `APPLICATION + 1`                     | Global scaled CSS     | `apply_global_scaled_css()` |
| `APPLICATION + 2`                     | Per-widget scoped CSS | `apply_widget_scaled_css()` |
| `STYLE_PROVIDER_PRIORITY_USER`        | Global user CSS       | `CssWatcher`                |
| `USER + 1`                            | Instance CSS          | `CssWatcher` (per-instance) |
| `USER + 2`                            | **Theme CSS**         | **Theme Service**           |

This ensures theme CSS overrides all other CSS sources, including per-instance user CSS. Theme CSS is removed and re-added when switching themes.

---

## 14. Common Pitfalls

- **GTK main thread dispatch**: CSS provider addition/removal must happen on the GTK main thread. The service runs in a separate `std::thread` where
  `glib::MainContext::default()` refers to the worker thread's context, not the GTK main thread. Use `glib::idle_add_once()` to dispatch closures to the GTK
  main loop — do **not** use `glib::MainContext::default().spawn_local()` from a worker thread.
- **CSS file path expansion**: All CSS file paths must be expanded with `shellexpand::tilde()` before loading, analogous to
  `WallpaperServiceConfig::expand_theme_paths()`.
- **System mode fallback**: When the personalization service reports `ColorScheme::System` (unresolved), the theme service defaults to dark CSS. This prevents a
  no-op when the system has not yet resolved its own color scheme.
- **Provider cleanup**: Old CSS providers must be removed from the display before adding new ones. Failing to do so accumulates CSS providers and causes style
  conflicts.
- **Mutex discipline**: CSS file loading (file I/O) should be done outside the state lock. Acquire the lock only briefly for updating state and storing provider
  handles.
- **StabbyOption vs Option**: Do not use `.map()/.unwrap_or()` on `StabbyOption`; use explicit `match` statements.
- **Empty themes list**: The service must handle an empty themes list gracefully — broadcast status with `current_theme: None` and `themes: []`.

---

## 15. Future Enhancements

- **Theme hot-reload**: Watch `themes.toml` and CSS files for changes, re-apply automatically (via `CssWatcher` extension or a dedicated file watcher)
- **Theme transitions**: Animate CSS transitions when switching themes (GTK CSS transition support)
- **Theme scheduling**: Time-based theme switching (e.g. dark at night, light during day)
- **Theme export/import**: Export current theme configuration as a shareable `.toml` file
- **Per-instance themes**: Different themes for different launcher instances (e.g. dark theme for bottom bar, light theme for side bar)
- **Theme color extraction**: Extract dominant colors from wallpaper and generate theme CSS automatically
- **Theme marketplace**: Download and install community themes via MCP
- **Custom CSS editor**: In-app CSS editing with live preview
- **Theme inheritance**: Themes can extend other themes, overriding only specific CSS rules
