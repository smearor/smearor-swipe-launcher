# weather (Service)

Weather data service that fetches forecasts from the Open-Meteo API.

## Description

The weather service fetches weather data from the [Open-Meteo API](https://open-meteo.com/) using the `openmeteo-rs` crate. It supports geocoding (city name →
coordinates), current conditions, forecasts, and various weather metrics. It broadcasts updates to all [weather widgets](../plugins/weather.md).

## Topics

| Topic                     | Direction         | Description                  |
|---------------------------|-------------------|------------------------------|
| `service.weather.command` | Widget → Service  | Refresh, query specific data |
| `service.weather.status`  | Service → Widgets | Weather data update          |

## MCP Tools

| Tool                         | Description                       |
|------------------------------|-----------------------------------|
| `weather_lookup_coordinates` | Get weather for given coordinates |
| `weather_get_forecast`       | Get forecast for current location |
| `weather_get_current`        | Get current conditions            |

## MCP Prompts

| Prompt                | Description                                              |
|-----------------------|----------------------------------------------------------|
| `weather_query_guide` | How to query weather (includes dynamic location context) |

## Configuration

```toml
[[services]]
id = "weather"
path = "target/release/libsmearor_weather_service.so"

[weather]
# Location can be set via geocoding or explicit coordinates
latitude = 52.52
longitude = 13.405
# Or use city name for geocoding
# city = "Berlin"
```

| Field       | Type             | Description             |
|-------------|------------------|-------------------------|
| `latitude`  | `Option<f64>`    | Explicit latitude       |
| `longitude` | `Option<f64>`    | Explicit longitude      |
| `city`      | `Option<String>` | City name for geocoding |

## Crate

- **Path**: `services/weather/`
- **Library**: `libsmearor_weather_service.so`
- **Model**: `model/weather/`
