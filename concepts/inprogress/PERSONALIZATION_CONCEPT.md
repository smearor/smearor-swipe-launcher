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
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub enum PersonalizationCommandAction {
    /// Force an immediate refresh of all personalization data.
    #[default]
    Refresh,
}

/// Command message sent by consumers to the personalization service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct PersonalizationCommandMessage {
    /// The action to execute.
    pub action: PersonalizationCommandAction,
}
```

---

## 5. Service Crate (`services/personalization`)

### 5.1 Configuration

```rust
/// Configuration for the personalization service.
#[derive(Clone, Debug, Deserialize)]
pub struct PersonalizationServiceConfig {
    /// Override the automatically detected latitude.
    pub override_latitude: Option<f64>,
    /// Override the automatically detected longitude.
    pub override_longitude: Option<f64>,
    /// Override the automatically detected timezone (IANA identifier).
    pub override_timezone: Option<String>,
    /// Override the automatically detected locale (e.g. "de-DE").
    pub override_locale: Option<String>,
    /// Override the location name (human-readable).
    pub override_location_name: Option<String>,
    /// Override the temperature unit.
    pub override_temperature_unit: Option<TemperatureUnit>,
    /// Override the wind speed unit.
    pub override_wind_speed_unit: Option<WindSpeedUnit>,
    /// Override the time format.
    pub override_time_format: Option<TimeFormat>,
    /// Override the date format.
    pub override_date_format: Option<DateFormat>,
    /// Override the first day of week.
    pub override_first_day_of_week: Option<FirstDayOfWeek>,
    /// Override the measurement system.
    pub override_measurement_system: Option<MeasurementSystem>,
    /// Override the color scheme.
    pub override_color_scheme: Option<ColorScheme>,
    /// Location accuracy for XDG Desktop Portal (e.g. "Street", "City").
    /// Defaults to "City" to minimize privacy impact.
    #[serde(default = "default_location_accuracy")]
    pub location_accuracy: String,
    /// Update interval for location queries in seconds.
    /// Timezone and locale are checked at start and on each refresh;
    /// location is polled at this interval.
    #[serde(default = "default_update_interval_seconds")]
    pub update_interval_seconds: u64,
    /// Whether to enable XDG Desktop Portal location queries.
    /// If false, only timezone and locale are queried; coordinates remain None.
    #[serde(default = "default_enable_location")]
    pub enable_location: bool,
}
```

### 5.2 Data Sources & Priority

The service queries data in the following priority order:

1. **Config Override** — If a config field is set, it takes absolute precedence.
2. **System API** — Automatic detection via the appropriate library.
3. **Fallback Default** — Sensible defaults if detection fails.

| Data          | Library              | Sync/Async | Fallback     |
|---------------|----------------------|------------|--------------|
| Timezone      | `iana_time_zone`     | Sync       | `UTC`        |
| Locale        | `sys_locale`         | Sync       | `en-US`      |
| Coordinates   | `ashpd` (XDG Portal) | Async      | `None`       |
| Location Name | Reverse geocoding    | Async      | `None`       |
| Units/Formats | Derived from locale  | Sync       | Metric / 24h |
| Color Scheme  | `ashpd` (Settings)   | Async      | `System`     |

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
   d. If enable_location: start async location session via ashpd
   e. Broadcast initial PersonalizationStatusMessage

2. Periodic (every update_interval_seconds):
   a. Re-query timezone and locale (detect system changes)
   b. If enable_location: re-query coordinates via ashpd
   c. If any value changed: broadcast updated PersonalizationStatusMessage

3. On Refresh command:
   a. Immediately re-query all sources
   b. Broadcast updated PersonalizationStatusMessage
```

### 5.5 MCP Integration

The service registers the following MCP capabilities:

**Resources:**

| URI                         | Description                          |
|-----------------------------|--------------------------------------|
| `personalization://profile` | Full personalization profile as JSON |

**Tools:**

| Tool Name              | Description                                     |
|------------------------|-------------------------------------------------|
| `get_current_location` | Returns latitude, longitude, and location name. |
| `get_timezone`         | Returns the current IANA timezone identifier.   |
| `get_locale`           | Returns the current system locale string.       |
| `get_personalization`  | Returns the full personalization profile.       |

---

## 6. Dependencies

```toml
[dependencies]
ashpd = "0.9"
iana-time-zone = "0.1"
sys-locale = "0.3"
stabby = { workspace = true }
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

### Phase 1: Model Crate (`model/personalization`)

**Goal:** Create the shared data structures and message formats.

**Tasks:**

- Create `model/personalization/Cargo.toml` with `stabby`, `serde`, `serde_json` dependencies.
- Create `model/personalization/src/lib.rs` with:
    - Topic constants (`TOPIC_COMMAND`, `TOPIC_STATUS`).
    - `GeoCoordinates` struct.
    - Enum types: `TemperatureUnit`, `WindSpeedUnit`, `TimeFormat`, `DateFormat`, `FirstDayOfWeek`, `MeasurementSystem`, `ColorScheme`.
    - `PersonalizationStatusMessage` struct.
    - `PersonalizationCommandMessage` and `PersonalizationCommandAction` structs.
    - All FFI-relevant types annotated with `#[stabby::stabby]`.
    - `impl_json_convertible!` macro for `PersonalizationStatusMessage` and `PersonalizationCommandMessage`.
- Add `model/personalization` to workspace `Cargo.toml`.
- Add `register_json_converters()` function.

**Verification:** `cargo build -p smearor-model-personalization` succeeds.

---

### Phase 2: Service Crate (`services/personalization`)

**Goal:** Implement the personalization service with system API queries.

**Tasks:**

- Create `services/personalization/Cargo.toml` with dependencies (see Section 6).
- Create `services/personalization/src/config.rs`:
    - `PersonalizationServiceConfig` struct with override fields and defaults.
- Create `services/personalization/src/service.rs`:
    - `PersonalizationService` struct with `meta`, `core_context`, `config`, `latest_state`.
    - `new()` constructor: parse config, spawn update loop thread.
    - `register_mcp_capabilities()`: register resources and tools.
    - `start()`: initial data query and broadcast.
    - Update loop: periodic re-query, change detection, broadcast on change.
    - `MessageHandler` impls for command, tool, and resource messages.
    - `ServicePlugin` impl with `on_message` routing.
- Create `services/personalization/src/lib.rs`:
    - Module declarations.
    - `service_plugin!(PersonalizationService);`
- Add `services/personalization` to workspace `Cargo.toml`.

**Data Query Logic:**

- Timezone: `iana_time_zone::get_timezone()` → `Ok("Europe/Berlin")` / `Err` → fallback `"UTC"`.
- Locale: `sys_locale::get_locale()` → `Some("de-DE")` / `None` → fallback `"en-US"`.
- Coordinates: `ashpd::desktop::location::LocationProxy::new().await` → create session → `locate()` → extract lat/lon.
- Derived values: locale prefix determines defaults (e.g. `de-*` → Celsius, 24h, DMY, Metric; `en-US` → Fahrenheit, 12h, MDY, Imperial).

**Verification:** `cargo build -p smearor-personalization-service` succeeds. Service loads and broadcasts initial status.

---

### Phase 3: Clock Widget Integration

**Goal:** Clock widget consumes `PersonalizationStatusMessage` for timezone and locale.

**Tasks:**

- Add `smearor-model-personalization` dependency to `plugins/clock/Cargo.toml`.
- Update `plugins/clock/src/widget.rs`:
    - Add `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` impl.
    - Store latest personalization status in `Arc<RwLock<Option<PersonalizationStatusMessage>>>`.
    - On status update: override `config.timezone` with personalization timezone.
    - On status update: use `locale` to determine weekday language (German for `de-*`, English for `en-*`, etc.).
- Update `plugins/clock/src/localized_weekday.rs`:
    - Add `from_locale(locale: &str) -> Self` method to `LocalizedWeekday`.
    - Map locale prefixes to supported languages.
    - Add additional languages as needed (e.g. French, Spanish, Italian).
- Update `plugins/clock/src/clock.rs`:
    - `get_timezone()` uses personalization timezone if available, falls back to config timezone.
    - `get_weekday_localized()` uses locale from personalization status.
- Update `plugins/clock/src/config.rs`:
    - `timezone` field becomes a fallback/override (still respected if personalization service is not running).

**Behavior:**

- If personalization service is running: Clock uses detected timezone and locale.
- If personalization service is not running: Clock falls back to `config.timezone` and German weekdays (current behavior).

**Verification:** Clock displays correct time for detected timezone. Weekday language matches system locale.

---

### Phase 4: Weather Service Integration

**Goal:** Weather service consumes `PersonalizationStatusMessage` for coordinates.

**Tasks:**

- Add `smearor-model-personalization` dependency to `services/weather/Cargo.toml`.
- Update `services/weather/src/service.rs`:
    - Add `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` impl.
    - Store latest personalization status in `Arc<RwLock<Option<PersonalizationStatusMessage>>>`.
    - In `run_update_loop`: use personalization coordinates if available, fall back to config coordinates.
    - On personalization update: if coordinates changed, trigger immediate weather refresh.
- Update `services/weather/src/config.rs`:
    - `latitude`, `longitude`, `location_name`, `timezone` become fallback values.
    - Add `use_personalization` flag (default: `true`) to allow disabling auto-detection.

**Behavior:**

- If personalization service is running and `use_personalization = true`: Weather uses detected coordinates.
- If personalization service is not running: Weather falls back to config coordinates (current behavior).
- Weather response data uses `temperature_unit` and `wind_speed_unit` from personalization for display formatting.

**Verification:** Weather widget shows data for the user's actual location. Temperature and wind speed use preferred units.

---

### Phase 5: Voice Assistant Integration

**Goal:** Voice Assistant service consumes `PersonalizationStatusMessage` for locale-aware responses.

**Tasks:**

- Add `smearor-model-personalization` dependency to `services/voice_assistant/Cargo.toml`.
- Update `services/voice_assistant/src/service.rs` (or relevant module):
    - Add `MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>>` impl.
    - Store latest personalization status.
    - Inject locale and timezone into LLM system prompts.
    - Use locale to select response language (e.g. `de-DE` → German responses, `en-US` → English responses).
    - Use timezone for time-related queries (e.g. "What time is it?").
    - Use coordinates for location-aware queries (e.g. "What's the weather?") by passing them to the weather service tools.

**Behavior:**

- Voice Assistant responds in the user's preferred language.
- Time-related answers use the user's timezone.
- Location-related queries use the user's coordinates without explicit configuration.

**Verification:** Voice Assistant responds in German when locale is `de-DE`. Time queries return correct local time. Weather queries use detected location.

---

## 8. Future Enhancements

The following widgets and services can benefit from personalization data in future phases:

### 8.1 Sysinfo Widget

- **Locale:** Format byte sizes (KB vs KiB), temperatures (Celsius vs Fahrenheit).
- **Measurement System:** Switch between metric and imperial units for disk/network speeds.
- **Locale:** Localize labels (e.g. "CPU" vs "Prozessor", "Memory" vs "Arbeitsspeicher").

### 8.2 Notifications Widget

- **Locale:** Format timestamps in notification list according to `time_format` and `date_format`.
- **Locale:** Localize relative time strings (e.g. "5 minutes ago" vs "vor 5 Minuten").

### 8.3 Power Widget

- **Locale:** Localize countdown text (e.g. "Shutting down in 30 seconds" vs "Herunterfahren in 30 Sekunden").
- **Time Format:** Use 12h/24h for scheduled action timestamps.

### 8.4 MPRIS Widget

- **Locale:** Format progress bar timestamps according to `time_format`.
- **Locale:** Localize "Unknown artist", "Unknown title" fallback strings.

### 8.5 Wallpaper Widget

- **Color Scheme:** Automatically select light or dark wallpaper themes based on `color_scheme` preference.
- **Color Scheme:** React to system dark mode changes (if `color_scheme = System`).

### 8.6 Network Widget

- **Measurement System:** Display bandwidth in metric (Mbps) or imperial units.
- **Locale:** Localize connection status strings.

### 8.7 App Launcher Widget

- **Locale:** Sort applications alphabetically according to locale collation rules.
- **Locale:** Localize category names (e.g. "Development" vs "Entwicklung").

### 8.8 Workspace Switcher Widget

- **First Day of Week:** Not directly applicable, but locale-aware sorting of workspace names.

### 8.9 Button Widget

- **Locale:** Localize `main_text` and `info_text` if they reference dynamic date/time values.
- **Time Format / Date Format:** Format any embedded timestamps.

---

## 9. Configuration Example

```toml
[[services]]
type = "personalization"
id = "personalization"
display_name = "Personalization"

[services.config]
# Automatic detection (no overrides needed for most users)
enable_location = true
location_accuracy = "City"
update_interval_seconds = 300

# Optional overrides (uncomment to pin specific values)
# override_latitude = 47.5031
# override_longitude = 9.7471
# override_location_name = "Bregenz"
# override_timezone = "Europe/Vienna"
# override_locale = "de-DE"
# override_temperature_unit = "celsius"
# override_time_format = "24h"
```

---

## 10. Advantages of This Design

- **Single Source of Truth:** All personalization data is managed in one service, eliminating redundant configuration.
- **Automatic Adaptation:** The system detects location, timezone, and language automatically. Users do not need to configure each plugin separately.
- **Graceful Degradation:** If the personalization service is not running, each consumer falls back to its own config values. No hard dependency.
- **Privacy-Conscious:** Location queries use XDG Desktop Portal, which requires user consent. Location can be disabled entirely via config.
- **MCP-Integrated:** AI clients can query personalization data to provide locale-aware responses without hardcoded assumptions.
- **Extensible:** New personalization fields can be added to `PersonalizationStatusMessage` without breaking existing consumers (all fields are `Option` or have
  defaults).
- **ABI-Stable:** All FFI-relevant types use `#[stabby::stabby]`, ensuring stable cross-plugin communication.
