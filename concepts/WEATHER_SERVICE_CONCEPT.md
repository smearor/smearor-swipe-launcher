# Concept: Weather Service & Widget

This document describes the concept for a **Weather Service** and a **Weather Widget** in the *Smearor Swipe Launcher*. The service fetches weather, forecast,
and air quality data from the [Open-Meteo API](https://open-meteo.com/) using the [`openmeteo-rs`](https://crates.io/crates/openmeteo-rs) crate and publishes
status updates via the central message broker. The widget displays the data in a compact, rotatable view format inspired
by [Mousam](https://github.com/amit9838/mousam), but adapted for the swipe launcher's tile-based layout.

The system follows the decoupled SOA architecture:

1. **Model Crate (`model/weather`):** Shared structs, enums, topics, and message formats.
2. **Service Crate (`services/weather`):** Singleton background service that polls Open-Meteo and broadcasts weather status.
3. **Widget Crate (`plugins/weather`):** Pure GTK4 UI that subscribes to weather status and rotates through configurable views on click.

---

## 1. System Architecture & Data Flow

```
+--------------------------+                 +----------------------------+
| Weather Widget           |                 | Weather Service            |
| (subscribed to           |                 | (Singleton)                |
|  service.weather.status) |                 |                            |
+--------------------------+                 +----------------------------+
             |                                             |
             |  1. Optional Command Message                |
             |  (refresh request, view change)             |
             |===========================================> |
             |  Topic: "service.weather.command"           |
             |                                             |
             |                                             |  2. openmeteo_rs::Client
             |                                             |     .forecast(lat, lon)
             |                                             |     .current(...)
             |                                             |     .hourly(...)
             |                                             |     .daily(...)
             |                                             |     .air_quality(...)
             |                                             |
             |                                             |  3. Status Broadcast
             | <===========================================|     Topic: "service.weather.status"
             |                                             |     Payload: WeatherStatusMessage { ... }
+--------------------------+                 +----------------------------+
                             \               /
                              \             /
                          +---------------------+
                          |  Open-Meteo API     |
                          |  api.open-meteo.com |
                          +---------------------+
```

The service also registers **MCP resources** so that AI clients can query the current weather state at any time without subscribing to the message bus.

---

## 2. Crate Structure

Following the workspace conventions (`AGENTS.md`), the feature is split into three crates:

| Crate       | Path                | Responsibility                                                       |
|-------------|---------------------|----------------------------------------------------------------------|
| **Model**   | `model/weather/`    | Shared structs, enums, topics, and message formats                   |
| **Service** | `services/weather/` | Backend logic, Open-Meteo API calls, periodic polling, MCP resources |
| **Widget**  | `plugins/weather/`  | GTK4 user interface, view rotation, click/longpress handling         |

---

## 3. Model Crate (`model/weather`)

### 3.1 Message Topics

```rust
pub const TOPIC_COMMAND: &str = "service.weather.command";
pub const TOPIC_STATUS: &str = "service.weather.status";
```

### 3.2 Weather Views Enum

The widget rotates through a configurable list of views on each click. Each view corresponds to a data category.

```rust
/// Available weather views that the widget can display.
/// Each variant corresponds to a data category rendered in the widget tile.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum WeatherView {
    /// Current temperature and cloud cover.
    #[default]
    Current,
    /// Today's forecast: temperature range and cloud cover.
    ForecastToday,
    /// Tomorrow's forecast: temperature range and cloud cover.
    ForecastTomorrow,
    /// Wind speed and direction.
    Wind,
    /// Relative humidity.
    Humidity,
    /// Sunrise and sunset times.
    SunriseSunset,
    /// Air quality index and pollutants.
    AirPollution,
    /// Atmospheric pressure.
    Pressure,
    /// UV index.
    UvIndex,
}
```

### 3.3 Command Message (Widget -> Service)

```rust
/// Actions the weather service can perform on request.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub enum WeatherCommandAction {
    /// Force an immediate refresh of weather data.
    #[default]
    Refresh,
}

/// Command message sent by widgets to the weather service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct WeatherCommandMessage {
    /// The action to execute.
    pub action: WeatherCommandAction,
}
```

### 3.4 Current Weather Data

```rust
/// Current weather conditions snapshot.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct CurrentWeather {
    /// Air temperature at 2 m above ground in degrees Celsius.
    pub temperature: Option<f32>,
    /// Cloud cover percentage (0.0 - 100.0).
    pub cloud_cover: Option<f32>,
    /// Relative humidity percentage (0.0 - 100.0).
    pub relative_humidity: Option<f32>,
    /// Wind speed at 10 m in km/h.
    pub wind_speed: Option<f32>,
    /// Wind direction at 10 m in degrees (0 = North, 180 = South).
    pub wind_direction: Option<f32>,
    /// Surface pressure in hPa.
    pub pressure: Option<f32>,
    /// UV index (0.0 - 11.0+).
    pub uv_index: Option<f32>,
    /// Weather interpretation code (WMO code).
    pub weather_code: Option<u16>,
    /// Whether it is currently day or night.
    pub is_day: Option<bool>,
}
```

### 3.5 Daily Forecast Data

```rust
/// Forecast data for a single day.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct DailyForecast {
    /// Maximum air temperature at 2 m in degrees Celsius.
    pub temperature_max: Option<f32>,
    /// Minimum air temperature at 2 m in degrees Celsius.
    pub temperature_min: Option<f32>,
    /// Cloud cover percentage (0.0 - 100.0).
    pub cloud_cover: Option<f32>,
    /// Sunrise time as ISO-8601 string.
    pub sunrise: stabby::option::Option<stabby::string::String>,
    /// Sunset time as ISO-8601 string.
    pub sunset: stabby::option::Option<stabby::string::String>,
    /// UV index max for the day.
    pub uv_index_max: Option<f32>,
    /// Maximum wind speed in km/h.
    pub wind_speed_max: Option<f32>,
    /// Precipitation sum in mm.
    pub precipitation_sum: Option<f32>,
    /// Weather interpretation code (WMO code).
    pub weather_code: Option<u16>,
}

/// Container for today's and tomorrow's daily forecast.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct DailyForecastData {
    /// Forecast for the current day.
    pub today: DailyForecast,
    /// Forecast for the next day.
    pub tomorrow: DailyForecast,
}
```

### 3.6 Air Quality Data

```rust
/// Air quality measurements.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct AirQualityData {
    /// European Air Quality Index (0 - 100).
    pub european_aqi: Option<f32>,
    /// PM10 concentration in micrograms per cubic metre.
    pub pm10: Option<f32>,
    /// PM2.5 concentration in micrograms per cubic metre.
    pub pm2_5: Option<f32>,
    /// Ozone concentration in micrograms per cubic metre.
    pub ozone: Option<f32>,
    /// Nitrogen dioxide concentration in micrograms per cubic metre.
    pub nitrogen_dioxide: Option<f32>,
    /// Sulphur dioxide concentration in micrograms per cubic metre.
    pub sulphur_dioxide: Option<f32>,
    /// Carbon monoxide concentration in micrograms per cubic metre.
    pub carbon_monoxide: Option<f32>,
}
```

### 3.7 Status Message (Service -> Widget)

```rust
/// Complete weather status message broadcast by the service.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct WeatherStatusMessage {
    /// Latitude of the configured location.
    pub latitude: f64,
    /// Longitude of the configured location.
    pub longitude: f64,
    /// Current weather conditions.
    pub current: CurrentWeather,
    /// Daily forecast for today and tomorrow.
    pub daily: DailyForecastData,
    /// Air quality data.
    pub air_quality: Option<AirQualityData>,
    /// Timestamp of the last successful update as ISO-8601 string.
    pub last_updated: stabby::string::String,
    /// Whether the last update succeeded.
    pub success: bool,
    /// Whether the data is stale (last fetch failed, but cached data is retained).
    pub is_stale: bool,
    /// Error message when the last update failed.
    pub error_message: stabby::option::Option<stabby::string::String>,
}
```

### 3.8 WMO Weather Code Mapping

The WMO weather interpretation codes from Open-Meteo are mapped to human-readable descriptions and Nerd Font icons in the widget. The mapping is defined in the
model crate as a utility function.

| WMO Code | Description            | Nerd Font Icon                  |
|----------|------------------------|---------------------------------|
| 0        | Clear sky              | `nf-weather-day_sunny`          |
| 1        | Mainly clear           | `nf-weather-day_sunny_overcast` |
| 2        | Partly cloudy          | `nf-weather-day_cloudy`         |
| 3        | Overcast               | `nf-weather_cloudy`             |
| 45       | Fog                    | `nf-weather_fog`                |
| 48       | Depositing rime fog    | `nf-weather_fog`                |
| 51       | Light drizzle          | `nf-weather_rain_mix`           |
| 53       | Moderate drizzle       | `nf-weather_rain`               |
| 55       | Dense drizzle          | `nf-weather_rain_wind`          |
| 56       | Light freezing drizzle | `nf-weather_rain_mix`           |
| 57       | Dense freezing drizzle | `nf-weather_rain_wind`          |
| 61       | Slight rain            | `nf-weather_rain`               |
| 63       | Moderate rain          | `nf-weather_rain`               |
| 65       | Heavy rain             | `nf-weather_rain_wind`          |
| 66       | Light freezing rain    | `nf-weather_rain_mix`           |
| 67       | Heavy freezing rain    | `nf-weather_rain_wind`          |
| 71       | Slight snow fall       | `nf-weather_snow`               |
| 73       | Moderate snow fall     | `nf-weather_snow`               |
| 75       | Heavy snow fall        | `nf-weather_snow_wind`          |
| 77       | Snow grains            | `nf-weather_snow`               |
| 80       | Slight rain showers    | `nf-weather_showers`            |
| 81       | Moderate rain showers  | `nf-weather_showers`            |
| 82       | Violent rain showers   | `nf-weather_showers_wind`       |
| 85       | Slight snow showers    | `nf-weather_snow`               |
| 86       | Heavy snow showers     | `nf-weather_snow_wind`          |
| 95       | Thunderstorm           | `nf-weather_storm_showers`      |
| 96       | Thunderstorm with hail | `nf-weather_storm_showers`      |
| 99       | Heavy thunderstorm     | `nf-weather_storm_showers`      |

### 3.9 Model Crate `lib.rs`

```rust
mod json_converters;
mod messages;

pub use json_converters::register_json_converters;
pub use messages::air_quality::AirQualityData;
pub use messages::command::WeatherCommandAction;
pub use messages::command::WeatherCommandMessage;
pub use messages::current::CurrentWeather;
pub use messages::daily::DailyForecast;
pub use messages::daily::DailyForecastData;
pub use messages::status::WeatherStatusMessage;
pub use messages::view::WeatherView;
pub use messages::wmo::weather_code_description;
pub use messages::wmo::weather_code_icon;
```

---

## 4. Service Crate (`services/weather`)

### 4.1 File Structure

- `service.rs` - `WeatherService` struct and trait implementations
- `config.rs` - `WeatherServiceConfig` struct and parsing
- `fetcher.rs` - Open-Meteo API call logic using `openmeteo-rs`
- `lib.rs` - `service_plugin!` macro invocation

### 4.2 Service Implementation

```rust
pub struct WeatherService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: WeatherServiceConfig,
    pub state: Arc<RwLock<WeatherStatusMessage>>,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<WeatherCommand>,
}

/// Internal command union for the service event loop.
pub enum WeatherCommand {
    Refresh,
    Fetch,
}
```

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<WeatherCommandMessage>>` - Processes refresh commands from widgets
- `MessageBroadcaster` - Broadcasts status messages to the broker
- `MessageTopicBroadcaster` - Broadcasts to topic subscribers
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to the core context
- `Service` - Routes raw FFI envelopes to the typed handler

### 4.3 Configuration

```rust
/// Configuration for the weather service.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WeatherServiceConfig {
    /// Latitude of the location to fetch weather for.
    pub latitude: f64,
    /// Longitude of the location to fetch weather for.
    pub longitude: f64,
    /// Update interval in minutes. Minimum is 10 minutes to respect Open-Meteo fair use.
    pub update_interval_minutes: u64,
    /// Whether to fetch air quality data.
    pub enable_air_quality: bool,
    /// Temperature unit (Celsius or Fahrenheit).
    pub temperature_unit: TemperatureUnit,
    /// Wind speed unit (km/h, mph, m/s, knots).
    pub wind_speed_unit: WindSpeedUnit,
    /// Timezone (auto or explicit IANA timezone).
    pub timezone: String,
}
```

```rust
impl Default for WeatherServiceConfig {
    fn default() -> Self {
        Self {
            latitude: 52.52,
            longitude: 13.41,
            update_interval_minutes: 30,
            enable_air_quality: true,
            temperature_unit: TemperatureUnit::Celsius,
            wind_speed_unit: WindSpeedUnit::Kmh,
            timezone: "auto".to_string(),
        }
    }
}
```

### 4.4 Open-Meteo API Integration

The service uses `openmeteo_rs::Client` to fetch data. On each update cycle, the service performs two API calls:

1. **Forecast API** (`/v1/forecast`): Current weather, hourly, and daily variables.
2. **Air Quality API** (`/v1/air-quality`): Current air quality index and pollutants (when enabled).

```rust
async fn fetch_weather(config: &WeatherServiceConfig) -> Result<WeatherStatusMessage, openmeteo_rs::Error> {
    let client = openmeteo_rs::Client::new();

    // Forecast request: current + daily variables
    let forecast_response = client
        .forecast(config.latitude, config.longitude)
        .current([
            openmeteo_rs::HourlyVar::Temperature2m,
            openmeteo_rs::HourlyVar::CloudCover,
            openmeteo_rs::HourlyVar::RelativeHumidity2m,
            openmeteo_rs::HourlyVar::WindSpeed10m,
            openmeteo_rs::HourlyVar::WindDirection10m,
            openmeteo_rs::HourlyVar::SurfacePressure,
            openmeteo_rs::HourlyVar::UvIndex,
            openmeteo_rs::HourlyVar::WeatherCode,
            openmeteo_rs::HourlyVar::IsDay,
        ])
        .daily([
            openmeteo_rs::DailyVar::Temperature2mMax,
            openmeteo_rs::DailyVar::Temperature2mMin,
            openmeteo_rs::DailyVar::CloudCoverMax,
            openmeteo_rs::DailyVar::Sunrise,
            openmeteo_rs::DailyVar::Sunset,
            openmeteo_rs::DailyVar::UvIndexMax,
            openmeteo_rs::DailyVar::WindSpeed10mMax,
            openmeteo_rs::DailyVar::PrecipitationSum,
            openmeteo_rs::DailyVar::WeatherCode,
        ])
        .forecast_days(2)
        .send()
        .await?;

    // Map openmeteo_rs response to WeatherStatusMessage
    let mut status = map_forecast_response(&forecast_response, config);

    // Air quality request (optional)
    if config.enable_air_quality {
        if let Ok(aq_response) = client
            .air_quality(config.latitude, config.longitude)
            .current([
                openmeteo_rs::AirQualityHourlyVar::EuropeanAqi,
                openmeteo_rs::AirQualityHourlyVar::Pm10,
                openmeteo_rs::AirQualityHourlyVar::Pm2_5,
                openmeteo_rs::AirQualityHourlyVar::Ozone,
                openmeteo_rs::AirQualityHourlyVar::NitrogenDioxide,
                openmeteo_rs::AirQualityHourlyVar::SulphurDioxide,
                openmeteo_rs::AirQualityHourlyVar::CarbonMonoxide,
            ])
            .send()
            .await
        {
            status.air_quality = Some(map_air_quality_response(&aq_response));
        }
    }

    status.success = true;
    status.last_updated = current_iso8601();
    Ok(status)
}
```

### 4.5 Background Update Loop

On initialization, the service spawns a dedicated OS thread with a single-threaded Tokio runtime. The runtime runs an update loop that fetches weather data at
the configured interval and broadcasts the result.

```rust
async fn run_update_loop(
    config: WeatherServiceConfig,
    state: Arc<RwLock<WeatherStatusMessage>>,
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<WeatherCommand>,
    broadcaster: Box<dyn MessageTopicBroadcaster>,
) {
    // Initial fetch on startup
    let status = fetch_weather(&config).await.unwrap_or_else(|error| {
        tracing::error!("Weather Service: initial fetch failed: {error}");
        error_status(&config, &error)
    });
    state.write().await.clone_from(&status);
    broadcaster.broadcast_topic(TOPIC_STATUS, status);

    let mut interval = tokio::time::interval(Duration::from_mins(config.update_interval_minutes));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                match fetch_weather(&config).await {
                    Ok(new_status) => {
                        state.write().await.clone_from(&new_status);
                        broadcaster.broadcast_topic(TOPIC_STATUS, new_status);
                    }
                    Err(error) => {
                        tracing::error!("Weather Service: fetch failed: {error}");
                        // Preserve cached data, mark as stale
                        let mut current = state.write().await;
                        current.is_stale = true;
                        current.error_message = stabby::option::Option::Some(
                            stabby::string::String::from(error.to_string())
                        );
                        let stale_status = current.clone();
                        drop(current);
                        broadcaster.broadcast_topic(TOPIC_STATUS, stale_status);
                    }
                }
            }
            Some(command) = command_receiver.recv() => {
                match command {
                    WeatherCommand::Refresh | WeatherCommand::Fetch => {
                        match fetch_weather(&config).await {
                            Ok(new_status) => {
                                state.write().await.clone_from(&new_status);
                                broadcaster.broadcast_topic(TOPIC_STATUS, new_status);
                            }
                            Err(error) => {
                                tracing::error!("Weather Service: refresh fetch failed: {error}");
                                let mut current = state.write().await;
                                current.is_stale = true;
                                current.error_message = stabby::option::Option::Some(
                                    stabby::string::String::from(error.to_string())
                                );
                                let stale_status = current.clone();
                                drop(current);
                                broadcaster.broadcast_topic(TOPIC_STATUS, stale_status);
                            }
                        }
                    }
                }
            }
        }
    }
}
```

### 4.6 MCP Resources

The service registers the following MCP resources via the Plugin-Resource-Registry:

| URI                            | Description                                       | Source type            |
|--------------------------------|---------------------------------------------------|------------------------|
| `plugin://weather/status`      | Complete weather status (current, forecast, AQI). | `WeatherStatusMessage` |
| `plugin://weather/current`     | Current weather conditions only.                  | `CurrentWeather`       |
| `plugin://weather/forecast`    | Daily forecast for today and tomorrow.            | `DailyForecastData`    |
| `plugin://weather/air_quality` | Air quality index and pollutants.                 | `AirQualityData`       |

The service also registers the following MCP tools:

| Tool                   | Description                                            | Parameters                                              |
|------------------------|--------------------------------------------------------|---------------------------------------------------------|
| `weather_refresh`      | Forces an immediate refresh of weather data.           | -                                                       |
| `weather_get_forecast` | Fetches a one-time forecast for arbitrary coordinates. | `latitude: f64`, `longitude: f64` (optional `days: u8`) |

> **MCP tool naming convention:** Tool names use `snake_case` with underscores, never dots. Dots in tool names cause schema validation failures in LLM
> gateways (Windsurf, Claude, etc.). This is consistent with existing tools like `sysinfo_refresh` and `get_current_time`.

The `weather_get_forecast` tool allows AI agents to query weather for arbitrary locations beyond the configured `services.toml` coordinates. The service
performs a one-time fetch for the requested coordinates and returns the result directly to the caller without updating the broadcast state.

---

## 5. Widget Crate (`plugins/weather`)

### 5.1 Overview

The Weather Widget is a compact GTK4 tile that displays weather data. On each click, it rotates through a configurable list of views. On long-press, it triggers
a configurable action (like the Clock Widget). The widget subscribes to `service.weather.status` and updates its display whenever a new status message arrives.

The default configuration includes **4 core views** (`Current`, `ForecastToday`, `Wind`, `AirPollution`) to keep click-rotation practical in a compact 120x80
pixel tile. Less frequently accessed data (`Humidity`, `SunriseSunset`, `Pressure`, `UvIndex`, `ForecastTomorrow`) can be added to the `views` list by the user,
or exposed in a dedicated **Weather Detail Area** that opens on long-press. This avoids excessive clicking through 9 views in a small tile.

### 5.2 File Structure

- `widget.rs` - `WeatherWidget` struct and trait implementations
- `config.rs` - `WeatherWidgetConfig` struct and parsing
- `views.rs` - View rendering functions for each `WeatherView` variant
- `lib.rs` - `widget_plugin!` macro invocation

### 5.3 Widget Configuration

```rust
/// Configuration for the weather widget.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WeatherWidgetConfig {
    /// Ordered list of views to cycle through on click.
    pub views: Vec<WeatherView>,

    /// Width of the widget in pixels.
    pub width: i32,

    /// Height of the widget in pixels.
    pub height: i32,

    /// Spacing between child widgets inside the weather widget.
    pub spacing: i32,

    /// Background color of the widget.
    pub background_color: Option<String>,

    /// Whether to show the weather icon.
    pub show_icon: bool,

    /// Whether to show the location name label.
    pub show_location: bool,

    /// Temperature unit label (e.g., "°C" or "°F").
    pub temperature_unit_label: String,

    /// Message topic for single-click.
    #[serde(default)]
    pub click_topic: Option<String>,

    /// Message payload for single-click (JSON/TOML).
    #[serde(default)]
    pub click_payload: Option<Value>,

    /// Message topic for long-press.
    #[serde(default)]
    pub longpress_topic: Option<String>,

    /// Message payload for long-press (JSON/TOML).
    #[serde(default)]
    pub longpress_payload: Option<Value>,
}

impl Default for WeatherWidgetConfig {
    fn default() -> Self {
        Self {
            views: vec![
                WeatherView::Current,
                WeatherView::ForecastToday,
                WeatherView::Wind,
                WeatherView::AirPollution,
            ],
            width: 120,
            height: 80,
            spacing: 4,
            background_color: None,
            show_icon: true,
            show_location: false,
            temperature_unit_label: "°C".to_string(),
            click_topic: None,
            click_payload: None,
            longpress_topic: None,
            longpress_payload: None,
        }
    }
}
```

### 5.4 Widget Implementation

```rust
pub struct WeatherWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: WeatherWidgetConfig,
    pub current_view: WeatherView,
    pub current_status: Option<WeatherStatusMessage>,
    pub view_index: usize,
}
```

> **GTK widget references:** GTK4 widgets (`gtk4::Box`, `gtk4::Image`, `gtk4::Label`) are **not** `Send` or `Sync`. They must not be stored in
`Arc<RwLock<...>>` inside the plugin struct. Instead, widget references are captured inside `glib::clone!` closures or passed directly to
`glib::MainContext::spawn_local` closures. The plugin struct only holds non-GTK state (`config`, `current_view`, `current_status`, `view_index`).

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<WeatherStatusMessage>>` - Receives status updates from the service
- `MessageBroadcaster` - Sends commands to the service
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to the core context
- `WidgetBuilder` - Builds the GTK4 widget UI

### 5.5 View Rotation

On each click, the widget advances to the next view in the configured `views` list. The rotation wraps around to the first view after the last one.

```rust
fn advance_view(&self) {
    let mut index = self.view_index.write();
    *index = (*index + 1) % self.config.views.len();
    let view = self.config.views[*index].clone();
    *self.current_view.write() = view.clone();
    self.render_view(&view);
}
```

If `click_topic` is configured, the click also publishes the configured message in addition to rotating the view. If `click_topic` is `None`, the click only
rotates the view.

### 5.6 View Rendering

Each view renders an icon, a primary label (large text), and a secondary label (small text) into the widget container. Rendering is done via
`glib::MainContext::spawn_local` to ensure GTK thread safety.

| View               | Icon                     | Primary Label | Secondary Label       |
|--------------------|--------------------------|---------------|-----------------------|
| `Current`          | WMO code icon            | `23.4°C`      | Cloud cover: `45%`    |
| `ForecastToday`    | WMO code icon (today)    | `18° / 25°`   | Cloud cover: `30%`    |
| `ForecastTomorrow` | WMO code icon (tomorrow) | `15° / 22°`   | Cloud cover: `60%`    |
| `Wind`             | `nf-weather-wind`        | `12.3 km/h`   | Direction: `180° (S)` |
| `Humidity`         | `nf-weather-humidity`    | `65%`         | -                     |
| `SunriseSunset`    | `nf-weather-sunrise`     | `06:12`       | `Sunset: 20:45`       |
| `AirPollution`     | AQI category icon        | `AQI 42`      | `PM2.5: 12.3 µg/m³`   |
| `Pressure`         | `nf-weather-barometer`   | `1013 hPa`    | -                     |
| `UvIndex`          | UV category icon         | `UV 6.2`      | `High`                |

### 5.7 Long-Press Action

On long-press, the widget publishes the configured `longpress_topic` and `longpress_payload` to the message broker, identical to the Clock Widget pattern. This
allows the user to trigger arbitrary actions such as opening a detailed weather area or sending a refresh command.

```toml
# Example: long-press refreshes weather data
longpress_topic = "service.weather.command"
longpress_payload = { action = "Refresh" }
```

### 5.8 State Synchronization

The widget subscribes to `service.weather.status`. When a new `WeatherStatusMessage` arrives:

1. The message is deserialized and stored in `current_status`.
2. The current view is re-rendered with the new data.
3. All GTK updates happen via `glib::MainContext::spawn_local`.

If the status message indicates a failure (`success = false`), the widget displays an error icon and the error message in the secondary label.

If the status message indicates stale data (`is_stale = true` but `success = true` from a previous fetch), the widget renders the cached weather data with a
dimmed opacity or a small warning indicator (e.g., a faded `nf-weather-alien` icon overlay). This ensures the user always sees useful weather information even
during network outages, rather than a blank error state.

---

## 6. Message Flow

```
+-------------------+         +-------------------+         +-------------------+
| Weather Widget    |<--------|                   |-------->| Weather Service   |
| (tile in scroll   |  Status |   Event Broker    | Command | (Singleton)       |
|  band)            | Broadcast                  Broadcast +-------------------+
+---------+---------+         +-------------------+         |                   |
          |                                                 | openmeteo_rs::    |
          | Click: advance_view()                           | Client            |
          | Longpress: publish longpress_payload            |                   |
          v                                                 v
+-------------------+                               +-------------------+
| View rotation     |                               | Open-Meteo API    |
| (local state)     |                               | api.open-meteo.com|
+-------------------+                               +-------------------+
```

---

## 7. Configuration Example

### 7.1 Service Registration in `services.toml`

```toml
[[services]]
id = "weather"
path = "target/release/libsmearor_weather_service.so"

[weather]
latitude = 52.52
longitude = 13.41
update_interval_minutes = 30
enable_air_quality = true
temperature_unit = "Celsius"
wind_speed_unit = "Kmh"
timezone = "auto"
```

### 7.2 Widget Configuration in `config.toml`

```toml
[[scroll_band.plugins]]
id = "weather_widget"
path = "target/release/libsmearor_weather_widget.so"

[weather_widget]
width = 120
height = 80
show_icon = true
show_location = false
temperature_unit_label = "°C"
views = [
    "Current",
    "ForecastToday",
    "Wind",
    "AirPollution",
]

# Long-press triggers a weather data refresh
longpress_topic = "service.weather.command"
longpress_payload = { action = "Refresh" }
```

### 7.3 Minimal Widget Configuration (only current + forecast)

```toml
[[scroll_band.plugins]]
id = "weather_widget"
path = "target/release/libsmearor_weather_widget.so"

[weather_widget]
views = ["Current", "ForecastToday", "ForecastTomorrow"]
show_icon = true
```

---

## 8. Roadmap

This roadmap defines the recommended order, dependencies, and deliverables for implementing the Weather feature. The order is chosen so that each layer is built
on top of already-tested foundations.

### Phase 1: Foundation — Model Crate (`model/weather`)

**Goal:** Define all shared messages, topics, views, and configuration types.

**Order:**

1. Create the crate `model/weather` with a `Cargo.toml` that depends on `serde`, `stabby`, and the project plugin API.
2. Create `src/topics.rs` and declare `TOPIC_COMMAND` and `TOPIC_STATUS`.
3. Create one file per message struct:
    - `src/messages/view.rs` -> `WeatherView` enum
    - `src/messages/command.rs` -> `WeatherCommandAction` and `WeatherCommandMessage`
    - `src/messages/current.rs` -> `CurrentWeather`
    - `src/messages/daily.rs` -> `DailyForecast` and `DailyForecastData`
    - `src/messages/air_quality.rs` -> `AirQualityData`
    - `src/messages/status.rs` -> `WeatherStatusMessage`
    - `src/messages/wmo.rs` -> WMO code to description and icon mapping functions
4. Add `#[stabby::stabby]` to all FFI-relevant types.
5. Re-export all public types in `src/lib.rs`.
6. Run `cargo check` and `cargo test` for the model crate.

**Exit criteria:**

- The crate compiles without warnings.
- Every public struct and enum has English rustdoc documentation.
- `cargo test` passes with serialization/deserialization tests for each message.
- WMO code mapping functions return correct descriptions and icon names for all codes.

---

### Phase 2: Backend — Service Crate (`services/weather`)

**Goal:** Fetch weather data from Open-Meteo and publish it on the status topic.

**Dependencies:** Phase 1 must be complete.

**Order:**

1. Create the crate `services/weather` with a `Cargo.toml` that depends on the `model/weather` crate, the project plugin API, `openmeteo-rs`, `tokio`, and
   `tracing`.
2. Create `src/config.rs` with `WeatherServiceConfig` and its default values.
3. Create `src/fetcher.rs` and implement `fetch_weather`:
    - Map `openmeteo_rs` forecast response to `WeatherStatusMessage`.
    - Map `openmeteo_rs` air quality response to `AirQualityData`.
    - Handle API errors gracefully and produce an error status message.
4. Create `src/service.rs` with `WeatherService` and all required trait implementations.
5. Implement `run_update_loop` to fetch at the configured interval and broadcast on `TOPIC_STATUS`.
6. Handle `WeatherCommandMessage::Refresh` to trigger an immediate fetch.
7. Register MCP resources (`plugin://weather/status`, `plugin://weather/current`, `plugin://weather/forecast`, `plugin://weather/air_quality`) and MCP tools (
   `weather_refresh`, `weather_get_forecast`).
8. Wire `service_plugin!` in `src/lib.rs`.
9. Add unit tests for the fetcher response mapping.

**Exit criteria:**

- The service compiles and loads as a plugin.
- Unit tests for response mapping produce plausible values from a mock or recorded response.
- Running the service broadcasts `TOPIC_STATUS` at least once per interval.
- MCP resources are registered and return JSON when queried.
- The `weather_refresh` tool triggers an immediate fetch.
- The `weather_get_forecast` tool returns weather data for arbitrary coordinates.
- On fetch failure, the service preserves cached data and sets `is_stale = true` instead of wiping the state.

---

### Phase 3: Display — Widget Crate (`plugins/weather`)

**Goal:** Provide a compact weather tile with click-to-rotate views and long-press action.

**Dependencies:** Phase 1 and Phase 2 must be complete.

**Order:**

1. Create the crate `plugins/weather` with a `Cargo.toml` that depends on `model/weather`, the project plugin API, `gtk4`, and `glib`.
2. Create `src/config.rs` with `WeatherWidgetConfig` including `views`, `longpress_topic`, and `longpress_payload`.
3. Create `src/views.rs` and implement a render function for each `WeatherView` variant:
    - `Current`: temperature + cloud cover + WMO icon
    - `ForecastToday`: today's min/max + cloud cover + WMO icon
    - `ForecastTomorrow`: tomorrow's min/max + cloud cover + WMO icon
    - `Wind`: wind speed + direction
    - `Humidity`: relative humidity percentage
    - `SunriseSunset`: sunrise and sunset times
    - `AirPollution`: AQI + PM2.5
    - `Pressure`: surface pressure in hPa
    - `UvIndex`: UV index value + category label
4. Create `src/widget.rs` with `WeatherWidget` and all required trait implementations.
5. Implement click handling: advance to the next view in `config.views` and re-render.
6. Implement long-press handling: publish `longpress_topic` / `longpress_payload` to the broker.
7. Subscribe to `TOPIC_STATUS` and update `current_status` + re-render on every message.
8. Wire `widget_plugin!` in `src/lib.rs`.
9. Add an integration test that verifies the widget accepts `TOPIC_STATUS` and rotates views on click.

**Exit criteria:**

- The widget compiles and can be loaded as a plugin.
- The widget displays the current view and rotates through all configured views on click.
- The widget updates its display when a new `WeatherStatusMessage` arrives.
- Long-press publishes the configured message to the broker.
- Error status messages display an error indicator.

---

### Phase 4: Wiring — Configuration and Registration

**Goal:** Connect all new crates to the main application.

**Dependencies:** Phase 2 and Phase 3 must be complete.

**Order:**

1. Add the `model/weather` and `services/weather` crates to the workspace `Cargo.toml`.
2. Register the service in `services.toml`.
3. Add a sample configuration block for `weather` in `config.toml`.
4. Add a sample widget configuration for the weather widget.

**Exit criteria:**

- The workspace compiles with `cargo build`.
- The service is loaded at application startup.
- The weather widget receives messages and renders correctly.

---

### Phase 5: Validation — Integration and Tests

**Goal:** Verify end-to-end behavior and stability.

**Dependencies:** Phase 4 must be complete.

**Order:**

1. Run the application and verify that `TOPIC_STATUS` appears on the message broker.
2. Verify the widget rotates through all configured views on click.
3. Verify long-press triggers the configured action.
4. Verify MCP resources return valid JSON.
5. Verify the `weather_refresh` tool triggers an immediate refresh.
6. Verify the `weather_get_forecast` tool returns weather for arbitrary coordinates.
7. Verify stale data handling: simulate a network failure and confirm the widget shows cached data with a stale indicator.
8. Run `cargo test` for all three crates.
9. Run `cargo clippy` and `cargo fmt` and fix any issues.
10. Verify the service handles API failures gracefully (e.g., network down).

**Exit criteria:**

- All tests pass.
- The widget renders correctly for all views.
- No `unwrap`, `expect`, or `panic` remains in the new code.
- `rustfmt` and `clippy` are clean.
- API failures display cached data with a stale indicator without crashing.
- The `weather_get_forecast` tool returns valid JSON for arbitrary coordinates.

---

### Summary of Order

```
Phase 1: model/weather
    |
    v
Phase 2: services/weather
    |
    v
Phase 3: plugins/weather
    |
    v
Phase 4: workspace wiring and config
    |
    v
Phase 5: integration and tests
```

### Rationale

- **Model first:** Message formats and view definitions must exist before the service or widget can use them.
- **Service second:** The widget needs a running publisher to test against.
- **Widget third:** The display widget depends on the service's status topic.
- **Wiring fourth:** Final integration only makes sense when all components are ready.
- **Tests last:** End-to-end validation closes the loop.

---

## 9. Technical Notes

- **No polling in the widget:** The widget updates exclusively through incoming messages. Periodic polling only happens in the service.
- **Fair use:** Open-Meteo is free for non-commercial use. The default update interval of 30 minutes respects the fair use policy. The minimum allowed interval
  is 10 minutes.
- **Attribution:** Open-Meteo data is licensed under CC BY 4.0. The widget or documentation should include attribution: `Weather data by Open-Meteo.com`.
- **Error handling:** If the API call fails (network error, invalid response), the service preserves the last successful weather data in `state`, sets
  `is_stale = true`, and attaches the error message. The widget renders cached data with a dimmed opacity or warning indicator. This ensures the user always
  sees useful weather information during transient network issues (e.g., laptop waking from suspend). Only if no data has ever been fetched successfully does
  the service broadcast `success = false` with an empty status.
- **Air quality optional:** Air quality data is fetched only when `enable_air_quality` is set. If the air quality API call fails, the forecast data is still
  broadcast with `air_quality = None`.
- **Units:** The service configures temperature and wind speed units via `openmeteo_rs` enums. The widget displays the unit label from the configuration.
- **Timezone:** The service uses `timezone = "auto"` by default, which makes Open-Meteo return times in the local timezone of the requested coordinates.
- **Mousam inspiration:** The view concept is inspired by Mousam's compact mode, but the swipe launcher widget has significantly more vertical space than a
  phone widget, allowing larger text and icons. The rotation-on-click pattern is well-suited for the swipe launcher's touch-first interaction model.
- **Default view count:** The default configuration includes 4 views (`Current`, `ForecastToday`, `Wind`, `AirPollution`) to keep click-rotation practical.
  Users can add more views via configuration. A dedicated Weather Detail Area (opened via long-press) can aggregate all 9 views in a larger layout for users who
  want full weather details without excessive clicking.
- **GTK widget ownership:** GTK4 widgets are not `Send` or `Sync`. They must not be stored in `Arc<RwLock<...>>` inside the plugin struct. Instead, widget
  references are captured in `glib::clone!` closures or `glib::MainContext::spawn_local` closures. The plugin struct only holds non-GTK state.
- **MCP tool naming:** Tool names use `snake_case` with underscores, never dots. Dots cause schema validation failures in LLM gateways. This is consistent with
  existing tools (`sysinfo_refresh`, `get_current_time`).
- **FFI string types:** All `String` and `Option<String>` fields in `#[stabby::stabby]` structs use `stabby::string::String` and
  `stabby::option::Option<stabby::string::String>` respectively, to maintain ABI stability across compiler invocations. This is consistent with the existing
  pattern in `model/notifications`, `model/audio`, and `model/app-launcher`.

---

## 10. Compliance with `AGENTS.md`

The proposed implementation follows the project guidelines in `AGENTS.md`:

- **Crate separation:** The feature is split into `model/weather`, `services/weather`, and `plugins/weather`.
- **One struct per file:** Each message struct and each view lives in its own file.
- **Service traits:** The service implements `MessageHandler`, `MessageBroadcaster`, `MessageTopicBroadcaster`, `PluginMetaGetter`, and
  `AsRef<Option<FfiCoreContext>>`.
- **Widget traits:** The widget implements `MessageHandler`, `MessageBroadcaster`, `PluginMetaGetter`, `AsRef<Option<FfiCoreContext>>`, and `WidgetBuilder`.
- **Async runtime:** The service uses `tokio::sync::mpsc` and spawns async tasks via the `PluginExecutor`.
- **GTK updates:** The widget uses `glib::MainContext::spawn_local` for GTK updates and `tokio::sync::mpsc` for message reception.
- **Event-driven:** The widget is updated by incoming messages, not by polling loops.
- **FFI stability:** All FFI-relevant types in the model carry `#[stabby::stabby]`. String fields use `stabby::string::String` and optional strings use
  `stabby::option::Option<stabby::string::String>` to maintain ABI stability across compiler invocations.
- **No panic:** The implementation uses `Result` and `Option` for error handling; no `unwrap()`, `expect()`, or `panic!`.
- **Naming:** All names are descriptive and follow Rust naming conventions.
- **Documentation:** All public structs, enums, and fields are documented in English.
- **Formatting:** Code is formatted with `rustfmt` and checked with `clippy`.
- **Dependencies:** The model uses `serde` and `stabby`; the service uses `openmeteo-rs`, `tokio`, and `tracing`; the widget uses `gtk4` and `glib`.
- **Long-press pattern:** The widget's `longpress_topic` and `longpress_payload` follow the same pattern as the Clock Widget (`plugins/clock/src/config.rs`).

---

*End of document.*
