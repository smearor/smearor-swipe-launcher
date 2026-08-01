# Concept: Personalization Service

This document describes the concept for a **Personalization Service** in the *Smearor Swipe Launcher*. The service acts as a central source of truth for
user-specific data such as geographic location, timezone, language, and display preferences. Other services and widgets subscribe to personalization updates and
adapt their behavior accordingly.

The system follows the decoupled SOA architecture:

1. **Model Crate (`model/personalization`):** Shared structs, enums, topics, and message formats.
2. **Service Crate (`services/personalization`):** Singleton background service that queries system APIs and broadcasts personalization status.
3. **Consumer Crates:** Services and widgets that subscribe to personalization updates (Clock, Weather, Voice Assistant, etc.).

---

## 1. Motivation & Problem

Currently, personalization data is scattered across multiple plugins:

- **Clock Widget:** Has its own `timezone` config field and hardcoded German weekday names.
- **Weather Service:** Has hardcoded `latitude`, `longitude`, `location_name`, and `timezone` in its config.
- **Voice Assistant:** Has no access to the user's locale for language-aware responses.

This leads to:

- **Redundant configuration:** Users must set their location and timezone in multiple places.
- **Inconsistent localization:** Weekday names are hardcoded to German; other widgets cannot adapt to the user's preferred language.
- **No dynamic adaptation:** If the user travels or changes system settings, each plugin must be reconfigured individually.

### The Solution: A Central Personalization Service

A single service queries the system for location, timezone, and locale data, then broadcasts changes to all interested consumers. Configuration overrides allow
users to pin specific values if automatic detection is undesired.

---

## 2. System Architecture & Data Flow

```
+--------------------------+                 +--------------------------------+
| Personalization Service  |                 | System APIs                    |
| (Singleton)              |                 |                                |
|                          |                 |  iana_time_zone::get_timezone()|
|  1. Query timezone       |<================|  sys_locale::get_locale()      |
|  2. Query locale         |                 |  ashpd::LocationProxy          |
|  3. Query location       |                 |    (XDG Desktop Portal)        |
|  4. Apply config overrides                 +--------------------------------+
|  5. Broadcast status     |
+--------------------------+
         |
         |  PersonalizationStatusMessage
         |  Topic: "service.personalization.status"
         |
    +----+----+----+----+----+
    |    |    |    |    |    |
    v    v    v    v    v    v
+-------+ +-------+ +-------+ +-------+ +-------+
| Clock | |Weather| | Voice | | Sysinfo| | Power |
|Widget | |Service| | Assist| | Widget | | Widget|
+-------+ +-------+ +-------+ +-------+ +-------+
```

The service also registers **MCP resources** so that AI clients can query personalization data at any time.

---

## 3. Crate Structure

Following the workspace conventions (`AGENTS.md`), the feature is split into two crates:

| Crate       | Path                        | Responsibility                                                           |
|-------------|-----------------------------|--------------------------------------------------------------------------|
| **Model**   | `model/personalization/`    | Shared structs, enums, topics, and message formats (`#[stabby::stabby]`) |
| **Service** | `services/personalization/` | Backend logic, system API queries, periodic polling, MCP resources       |

Consumer crates (Clock, Weather, Voice Assistant, etc.) depend on the model crate and subscribe to status messages.

---

## 4. Model Crate (`model/personalization`)

### 4.1 Message Topics

```rust
pub const TOPIC_COMMAND: &str = "service.personalization.command";
pub const TOPIC_STATUS: &str = "service.personalization.status";
```

### 4.2 Personalization Data Struct

```rust
/// Geographic coordinates of the user's current location.
#[repr(C)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct GeoCoordinates {
    /// Latitude in decimal degrees.
    pub latitude: f64,
    /// Longitude in decimal degrees.
    pub longitude: f64,
    /// Human-readable location name (reverse-geocoded or configured).
    pub location_name: stabby::option::Option<stabby::string::String>,
}

/// The user's preferred temperature unit.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum TemperatureUnit {
    /// Degrees Celsius (default).
    #[default]
    Celsius,
    /// Degrees Fahrenheit.
    Fahrenheit,
}

/// The user's preferred wind speed unit.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum WindSpeedUnit {
    /// Kilometers per hour (default).
    #[default]
    Kmh,
    /// Miles per hour.
    Mph,
    /// Meters per second.
    Ms,
}

/// The user's preferred time format.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum TimeFormat {
    /// 24-hour format (e.g. 14:30).
    #[default]
    Hour24,
    /// 12-hour format with AM/PM (e.g. 2:30 PM).
    Hour12,
}

/// The user's preferred date format.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum DateFormat {
    /// Day.Month.Year (e.g. 26.07.2026) — common in Europe.
    #[default]
    Dmy,
    /// Month/Day/Year (e.g. 07/26/2026) — common in the US.
    Mdy,
    /// Year-Month-Day (e.g. 2026-07-26) — ISO 8601.
    Ymd,
}

/// The user's preferred first day of the week.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum FirstDayOfWeek {
    /// Monday as the first day (default, ISO standard).
    #[default]
    Monday,
    /// Sunday as the first day (common in the US).
    Sunday,
}

/// The user's preferred measurement system.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum MeasurementSystem {
    /// Metric system (default).
    #[default]
    Metric,
    /// Imperial system.
    Imperial,
}

/// The user's preferred color scheme.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum ColorScheme {
    /// Follow system settings (default).
    #[default]
    System,
    /// Light mode.
    Light,
    /// Dark mode.
    Dark,
}

/// Complete personalization profile broadcast by the service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct PersonalizationStatusMessage {
    /// Geographic coordinates of the user's location.
    pub coordinates: stabby::option::Option<GeoCoordinates>,
    /// IANA timezone identifier (e.g. "Europe/Berlin").
    pub timezone: stabby::option::Option<stabby::string::String>,
    /// System locale string (e.g. "de-DE", "en-US").
    pub locale: stabby::option::Option<stabby::string::String>,
    /// Preferred temperature unit.
    pub temperature_unit: TemperatureUnit,
    /// Preferred wind speed unit.
    pub wind_speed_unit: WindSpeedUnit,
    /// Preferred time format (12h/24h).
    pub time_format: TimeFormat,
    /// Preferred date format.
    pub date_format: DateFormat,
    /// First day of the week.
    pub first_day_of_week: FirstDayOfWeek,
    /// Preferred measurement system (metric/imperial).
    pub measurement_system: MeasurementSystem,
    /// Preferred color scheme (light/dark/system).
    pub color_scheme: ColorScheme,
    /// Whether the data was fetched successfully.
    pub success: bool,
    /// Error message if fetching failed.
    pub error_message: stabby::option::Option<stabby::string::String>,
}
```

### 4.3 Command Message (Consumer -> Service)

```rust
/// Actions the personalization service can perform on request.
#[repr(C, u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PersonalizationCommandAction {
    /// Force an immediate refresh of all personalization data from system APIs.
    /// Clears all runtime overrides and re-queries system APIs.
    Refresh,
    /// Update the user's location at runtime.
    /// The service stores the new coordinates and broadcasts an updated status.
    /// This overrides any config or auto-detected value until a `Refresh` is triggered
    /// or the service is restarted.
    UpdateLocation(GeoCoordinates),
    /// Update the user's locale at runtime.
    /// The service stores the new locale, re-derives unit/format preferences,
    /// and broadcasts an updated status.
    /// This overrides any config or auto-detected value until a `Refresh` is triggered
    /// or the service is restarted.
    UpdateLocale(stabby::string::String),
    /// Request an immediate status re-broadcast without clearing runtime overrides.
    /// Used by widgets that are lazily loaded after the initial status broadcast.
    RequestStatus,
}

/// Command message sent by consumers to the personalization service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersonalizationCommandMessage {
    /// The action to execute.
    pub action: PersonalizationCommandAction,
}
```

The `PersonalizationCommandMessage` provides convenience constructors: `refresh()`, `update_location(coords)`, `update_locale(locale)`, `request_status()`.

---

## 5. Service Crate (`services/personalization`)

### 5.1 Configuration

```rust
/// Configuration overrides for personalization data.
///
/// All fields are optional. When present, they override the auto-detected
/// system values. Runtime overrides (via command messages) take priority
/// over these config values.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct PersonalizationServiceConfig {
    /// Fixed latitude override.
    #[serde(default)]
    pub latitude: Option<f64>,
    /// Fixed longitude override.
    #[serde(default)]
    pub longitude: Option<f64>,
    /// Fixed location name override.
    #[serde(default)]
    pub location_name: Option<String>,
    /// Fixed timezone override (IANA identifier, e.g. "Europe/Berlin").
    #[serde(default)]
    pub timezone: Option<String>,
    /// Fixed locale override (e.g. "de-DE", "en-US").
    #[serde(default)]
    pub locale: Option<String>,
    /// Fixed temperature unit override (parsed via FromStr).
    #[serde(default)]
    pub temperature_unit: Option<String>,
    /// Fixed wind speed unit override (parsed via FromStr).
    #[serde(default)]
    pub wind_speed_unit: Option<String>,
    /// Fixed time format override (parsed via FromStr).
    #[serde(default)]
    pub time_format: Option<String>,
    /// Fixed date format override (parsed via FromStr).
    #[serde(default)]
    pub date_format: Option<String>,
    /// Fixed first day of week override (parsed via FromStr).
    #[serde(default)]
    pub first_day_of_week: Option<String>,
    /// Fixed measurement system override (parsed via FromStr).
    #[serde(default)]
    pub measurement_system: Option<String>,
    /// Fixed color scheme override (parsed via FromStr).
    #[serde(default)]
    pub color_scheme: Option<String>,
    /// Whether to enable location detection via XDG Desktop Portal.
    /// Currently not yet wired up (marked `#[allow(dead_code)]`).
    #[serde(default = "default_enable_location")]
    pub enable_location: bool,
    /// Update interval in seconds for periodic system API re-queries.
    #[serde(default = "default_update_interval_seconds")]
    pub update_interval_seconds: u64,
}
```

**Note:** Unit/format override fields are stored as `Option<String>` and parsed via `FromStr` at query time, not as strongly-typed enums. This allows invalid
values to gracefully fall back to locale-derived defaults.

### 5.2 Data Sources & Priority

The service queries data in the following priority order:

1. **Runtime Override** — Set via command message (`UpdateLocation`, `UpdateLocale`). Takes absolute precedence.
2. **Config Override** — If a config field is set, it takes precedence over auto-detection.
3. **System API** — Automatic detection via the appropriate library.
4. **Fallback Default** — Sensible defaults if detection fails.

| Data          | Library                    | Sync/Async | Fallback     | Status                                      |
|---------------|----------------------------|------------|--------------|---------------------------------------------|
| Timezone      | `iana_time_zone`           | Sync       | `UTC`        | ✅ Implemented                              |
| Locale        | `sys_locale`               | Sync       | `en-US`      | ✅ Implemented                              |
| Coordinates   | `ashpd` (XDG Portal)       | Async      | `None`       | ✅ Implemented (gated by `enable_location`) |
| Location Name | Reverse geocoding (Photon) | Async      | `None`       | ✅ Implemented                              |
| Units/Formats | Derived from locale        | Sync       | Metric / 24h | ✅ Implemented                              |
| Color Scheme  | `ashpd` (Settings)         | Async      | `System`     | ✅ Implemented                              |

**Implemented:** `ashpd` integration for XDG Desktop Portal location queries (gated by `enable_location` config field), color-scheme detection via the Settings
portal, and reverse geocoding via the Photon API (photon.komoot.io) for automatic location name resolution.

### 5.3 Service Implementation

The service implements the standard service plugin traits:

- `ServicePlugin` — `on_message` handles `PersonalizationCommandMessage` and MCP tool invocations.
- `MessageHandler<FfiEnvelopePayload<PersonalizationCommandMessage>>` — Processes refresh commands.
- `MessageHandler<FfiEnvelopePayload<InvokeToolMessage>>` — MCP tool handlers.
- `MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>>` — MCP resource handler.
- `MessageBroadcaster` — Empty impl.
- `MessageTopicBroadcaster<PersonalizationStatusMessage>` — Broadcasts status updates.
- `PluginMetaGetter` — Returns plugin metadata.
- `AsRef<Option<FfiCoreContext>>` — Returns core context.

### 5.4 Update Loop

```
1. Start:
   a. Query timezone via iana_time_zone::get_timezone() (sync)
   b. Query locale via sys_locale::get_locale() (sync)
   c. Derive units/formats from locale
   d. If enable_location: query coordinates via ashpd (one-shot session)
   e. If coordinates resolved: reverse geocode via Photon API for location_name
   f. Broadcast initial PersonalizationStatusMessage

2. Periodic (every update_interval_seconds):
   a. Re-query timezone and locale (detect system changes)
   b. Re-query color scheme via ashpd Settings portal
   c. If enable_location and location_update_interval_seconds elapsed:
      - Re-query coordinates via ashpd
      - If change > location_change_threshold: update cached coords + reverse geocode
      - If change <= threshold: keep cached coords (skip reverse geocoding)
   d. Broadcast updated PersonalizationStatusMessage

3. On Refresh command:
   a. Clear runtime overrides + cached portal coords
   b. Immediately re-query all sources (including location)
   c. Broadcast updated PersonalizationStatusMessage
```

### 5.5 MCP Integration

The service registers the following MCP capabilities:

**Resources:**

| URI                         | Description                          |
|-----------------------------|--------------------------------------|
| `personalization://profile` | Full personalization profile as JSON |

**Tools:**

| Tool Name                 | Description                                                                             |
|---------------------------|-----------------------------------------------------------------------------------------|
| `get_current_location`    | Returns latitude, longitude, and location name.                                         |
| `get_timezone`            | Returns the current IANA timezone identifier.                                           |
| `get_locale`              | Returns the current system locale string.                                               |
| `get_personalization`     | Returns the full personalization profile as JSON.                                       |
| `set_current_location`    | Sets a runtime override for the user's location. Persists until a refresh is triggered. |
| `set_locale`              | Sets a runtime override for the user's locale. Persists until a refresh is triggered.   |
| `refresh_personalization` | Clears all runtime overrides and re-queries system APIs.                                |

---

## 6. Dependencies

```toml
[dependencies]
ashpd = { version = "0.13", default-features = false, features = ["location", "settings", "tokio"] }
futures-util = "0.3"
iana-time-zone = "0.1"
sys-locale = "0.3"
stabby = { workspace = true }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
paste = { workspace = true }
miette = { workspace = true }
thiserror = { workspace = true }
smearor-model-mcp = { path = "../../model/mcp" }
smearor-model-personalization = { path = "../../model/personalization" }
smearor-swipe-launcher-plugin-api = { path = "../../plugin-api" }
```

---

## 7. Phase Implementation Plan

### Phase 1: Model Crate (`model/personalization`) — ✅ Completed

**Goal:** Create the shared data structures and message formats.

**Implemented:**

- `model/personalization/Cargo.toml` with `stabby`, `serde`, `serde_json` dependencies.
- `model/personalization/src/lib.rs` with:
    - Topic constants (`TOPIC_COMMAND`, `TOPIC_STATUS`).
    - `GeoCoordinates` struct.
    - Enum types: `TemperatureUnit`, `WindSpeedUnit`, `TimeFormat`, `DateFormat`, `FirstDayOfWeek`, `MeasurementSystem`, `ColorScheme`.
    - `PersonalizationStatusMessage` struct.
    - `PersonalizationCommandMessage` and `PersonalizationCommandAction` (with `Refresh`, `UpdateLocation`, `UpdateLocale`, `RequestStatus`).
        - All FFI-relevant types annotated with `#[stabby::stabby]`.
        - `impl_json_convertible!` macro for `PersonalizationStatusMessage` and `PersonalizationCommandMessage`.
    - `register_json_converters()` function.
- Each type in its own file under `model/personalization/src/messages/`.
- MCP tool/resource enums in `model/personalization/src/mcp/`.
- Added to workspace `Cargo.toml`.

**Verification:** `cargo build -p smearor-model-personalization` succeeds.

---

### Phase 2: Service Crate (`services/personalization`) — ✅ Completed

**Goal:** Implement the personalization service with system API queries.

**Implemented:**

- `services/personalization/Cargo.toml` with dependencies.
- `services/personalization/src/config.rs` — `PersonalizationServiceConfig` with override fields and defaults.
- `services/personalization/src/service.rs`:
    - `PersonalizationService` struct with `meta`, `core_context`, `config`, `command_sender`, `latest_state`.
        - `new()` constructor: parse config, spawn update loop thread.
    - `register_mcp_capabilities()`: registers 1 resource + 7 tools.
    - Update loop via `tokio::select!` on interval tick + command channel.
    - `build_status()`: queries timezone/locale, derives units from locale, applies config + runtime overrides.
        - `MessageHandler` impls for command, tool, and resource messages.
        - `ServicePlugin` impl with `on_message` routing.
- `services/personalization/src/command.rs` — Internal `PersonalizationCommand` enum for async loop.
- `services/personalization/src/state.rs` — `LatestPersonalizationState` shared between loop and MCP handlers.
- `services/personalization/src/mcp/` — MCP capabilities registration and handlers (tools, resources).
- `services/personalization/src/lib.rs` — `service_plugin!(PersonalizationService);`
- Added to workspace `Cargo.toml`.

**Data Query Logic (implemented):**

- Timezone: `iana_time_zone::get_timezone()` → `Ok("Europe/Berlin")` / `Err` → fallback `"UTC"`.
- Locale: `sys_locale::get_locale()` → `Some("de-DE")` / `None` → fallback `"en-US"`.
- Derived values: `Locale::from_str()` + `FromLocale` trait determines defaults (e.g. `de-*` → Celsius, 24h, DMY, Metric; `en-US` → Fahrenheit, 12h, MDY,
  Imperial).
- Unit/format overrides parsed via `FromStr` on each enum type.

**ashpd integration (implemented):**

- `services/personalization/src/portal.rs` — Async helpers for XDG Desktop Portal queries:
    - `query_location()`: Creates a one-shot `LocationProxy` session with street-level accuracy, waits for the first location update, then closes the session.
      Returns `Option<GeoCoordinates>`.
    - `query_color_scheme()`: Queries the system's preferred color scheme via `Settings::color_scheme()`. Maps `NoPreference` → `System`, `PreferDark` → `Dark`,
      `PreferLight` → `Light`.
    - `reverse_geocode()`: Reverse geocodes coordinates to a location name via the Photon API (photon.komoot.io/reverse). Preference order: city > name >
      street + housenumber > state > country.
- `build_status()` is now `async` and calls `portal::query_location()` when `enable_location` is true and no runtime/config override exists.
- After coordinates are resolved, `portal::reverse_geocode()` is called if no `location_name` is set.
- Color scheme queries `portal::query_color_scheme()` when no config override is set.
- `enable_location` config field is now wired up (removed `#[allow(dead_code)]`).

**Not yet implemented:**

- None — all planned features are implemented.

**Verification:** `cargo build -p smearor-personalization-service` succeeds. Service loads and broadcasts initial status.

---

### Phase 3: Clock Widget Integration — ✅ Completed

**Goal:** Clock widget consumes `PersonalizationStatusMessage` for timezone and locale.

**Implemented:**

- `smearor-model-personalization` dependency added to `plugins/clock/Cargo.toml`.
- `plugins/clock/src/widget.rs` — `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` impl, stores personalization status, overrides timezone and
  weekday language on update.
- `plugins/clock/src/atomic.rs` — Atomic widget also handles `PersonalizationStatusMessage`.
- `plugins/clock/src/clock.rs` — Uses personalization timezone if available, falls back to config.
- Config `timezone` field remains as fallback when personalization service is not running.

**Behavior:**

- If personalization service is running: Clock uses detected timezone and locale.
- If personalization service is not running: Clock falls back to `config.timezone` and German weekdays.

**Verification:** Clock displays correct time for detected timezone. Weekday language matches system locale.

---

### Phase 4: Weather Service Integration — ✅ Completed

**Goal:** Weather service consumes `PersonalizationStatusMessage` for coordinates.

**Implemented:**

- `smearor-model-personalization` dependency added to `services/weather/Cargo.toml`.
- `services/weather/src/service.rs` — `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` impl.
- `services/weather/src/personalization_coordinates.rs` — `PersonalizationCoordinates` struct for storing coords.
- On personalization update: if coordinates changed, triggers immediate weather refresh via `WeatherCommandAction::Refresh`.
- Weather widget (`plugins/weather`) also handles `PersonalizationStatusMessage` for locale-aware display.

**Behavior:**

- If personalization service is running: Weather uses detected coordinates.
- If personalization service is not running: Weather falls back to config coordinates.

**Verification:** Weather widget shows data for the user's actual location. Temperature and wind speed use preferred units.

---

### Phase 5: Voice Assistant Integration — ✅ Completed

**Goal:** Voice Assistant service consumes `PersonalizationStatusMessage` for locale-aware responses.

**Implemented:**

- `smearor-model-personalization` dependency added to `services/voice_assistant/Cargo.toml`.
- `services/voice_assistant/src/service.rs` — `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` impl, stores personalization status.
- `services/voice_assistant/src/tool_catalog.rs` — Personalization data used in tool catalog.
- Voice Assistant widget (`plugins/voice_assistant`) also handles `PersonalizationStatusMessage`.

**Behavior:**

- Voice Assistant responds in the user's preferred language.
- Time-related answers use the user's timezone.
- Location-related queries use the user's coordinates without explicit configuration.

**Verification:** Voice Assistant responds in German when locale is `de-DE`. Time queries return correct local time. Weather queries use detected location.

---

## 8. Consumer Widget Integration Status

The following widgets have been integrated with personalization data:

### 8.1 Sysinfo Widget — ✅ Integrated

- All sub-widgets (CPU, Memory, Disks, Network, Temperature, Uptime) handle `PersonalizationStatusMessage`.
- **Locale:** Used for label localization and unit formatting.
- **Measurement System:** Switches between metric and imperial units for disk/network speeds.
- **Temperature Unit:** Celsius vs Fahrenheit for temperature widget.

### 8.2 Notifications Widget — ✅ Integrated

- Both main widget and atomic widget handle `PersonalizationStatusMessage`.
- **Locale:** Used for timestamp formatting and label localization.

### 8.3 Power Widget — ✅ Integrated

- Both main widget and atomic widget handle `PersonalizationStatusMessage`.
- **Locale:** Used for countdown text localization.
- **Time Format:** 12h/24h for scheduled action timestamps.

### 8.4 MPRIS Widget — ✅ Integrated

- Both main widget and atomic widget handle `PersonalizationStatusMessage`.
- **Locale:** Used for progress bar timestamp formatting and fallback string localization.

### 8.5 Wallpaper Widget — ✅ Integrated

- Both main widget and atomic widget handle `PersonalizationStatusMessage`.
- **Locale:** Used for label localization.
- **Color Scheme:** Automatic light/dark wallpaper selection based on `color_scheme` — now powered by `ashpd` Settings integration.

### 8.6 Network Widget — ✅ Integrated

- Both main widget and atomic widget handle `PersonalizationStatusMessage`.
- **Measurement System:** Bandwidth display in metric or imperial units.
- **Locale:** Connection status string localization.

### 8.7 App Launcher Widget — ✅ Integrated

- Main widget handles `PersonalizationStatusMessage`.
- **Locale:** Used for label localization.

### 8.8 Audio Widget — ✅ Integrated

- Both main widget and atomic widget handle `PersonalizationStatusMessage`.
- **Locale:** Used for label localization.

### 8.9 Weather Widget — ✅ Integrated

- Both main widget and atomic widget handle `PersonalizationStatusMessage`.
- **Locale:** Used for temperature unit and wind speed unit display.

### 8.10 Voice Assistant Widget — ✅ Integrated

- Both main widget and atomic widget handle `PersonalizationStatusMessage`.
- **Locale:** Used for response language selection.

### 8.11 Workspace Switcher Widget — ✅ Integrated

- Main widget handles `PersonalizationStatusMessage`.
- **Locale:** Locale-aware sorting of workspace names (via `sort_workspaces` helper).

### 8.12 Button Widget — ✅ Integrated

- Main widget handles `PersonalizationStatusMessage`.
- **Time Format / Date Format:** Locale-aware formatting for dynamic label values.
- **Locale:** Available for label localization.

---

## 9. Configuration Example

```toml
[[services]]
type = "personalization"
id = "personalization"
display_name = "Personalization"

[personalization]
# Automatic detection (no overrides needed for most users)
enable_location = false  # Set to true to enable XDG Desktop Portal location queries
update_interval_seconds = 300
location_update_interval_seconds = 1800  # Re-query location every 30 minutes (min 300)
location_change_threshold = 0.01  # Skip update if coords change < ~1.1 km

# Optional overrides (uncomment to pin specific values)
# latitude = 47.5031
# longitude = 9.7471
# location_name = "Bregenz"
# timezone = "Europe/Vienna"
# locale = "de-DE"
# temperature_unit = "celsius"
# time_format = "24h"
# date_format = "dmy"
# first_day_of_week = "monday"
# measurement_system = "metric"
# color_scheme = "system"
# wind_speed_unit = "kmh"
```

---

## 10. Advantages of This Design

- **Single Source of Truth:** All personalization data is managed in one service, eliminating redundant configuration.
- **Automatic Adaptation:** The system detects timezone and language automatically. Users do not need to configure each plugin separately.
- **Graceful Degradation:** If the personalization service is not running, each consumer falls back to its own config values. No hard dependency.
- **Privacy-Conscious:** Location queries use XDG Desktop Portal (requires user consent). Location can be disabled entirely via config
  (`enable_location = false`).
- **MCP-Integrated:** AI clients can query and override personalization data via 7 MCP tools and 1 resource.
- **Extensible:** New personalization fields can be added to `PersonalizationStatusMessage` without breaking existing consumers (all fields are `Option` or have
  defaults).
- **ABI-Stable:** All FFI-relevant types use `#[stabby::stabby]`, ensuring stable cross-plugin communication.

