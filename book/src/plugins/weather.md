# weather (Plugin)

Weather forecast widget with 15 views: current conditions, forecast, wind, humidity, UV index, sunrise/sunset, and more.

## Description

The weather widget communicates with the [weather service](../services/weather.md) which fetches data from the Open-Meteo API. It cycles through views via swipe
up/down, each showing different weather data with state-dependent icons.

## Views

| View                      | Description                           |
|---------------------------|---------------------------------------|
| Current                   | Temperature, weather code, feels-like |
| Forecast Today            | Today's high/low, conditions          |
| Forecast Tomorrow         | Tomorrow's high/low, conditions       |
| Wind                      | Wind speed and direction              |
| Humidity                  | Relative humidity                     |
| UV Index                  | UV index with risk level              |
| Sunrise                   | Sunrise time                          |
| Sunset                    | Sunset time                           |
| Cloud Cover               | Cloud cover percentage                |
| Sunshine                  | Sunshine duration                     |
| Precipitation Probability | Chance of rain                        |
| Precipitation Amount      | Rain amount                           |
| Precipitation             | Current precipitation                 |
| Air Pollution             | Air quality index                     |
| Pressure                  | Atmospheric pressure                  |

## Configuration

```toml
[weather_widget]
path = "target/release/libsmearor_weather_widget.so"
widget = "weather"
icon_size = 32
icon_only = false
mode = "compact"
max_width = 200
```

| Field       | Type          | Description          |
|-------------|---------------|----------------------|
| `icon_size` | `i32`         | Icon size in pixels  |
| `icon_only` | `bool`        | Show only the icon   |
| `mode`      | `WidgetMode`  | `compact` or `wide`  |
| `max_width` | `Option<i32>` | Maximum widget width |

## Dynamic Icons

The Current and Forecast views use state-dependent icons derived from the WMO weather code. Other views use data-driven icons resolved via the
`WidgetIconRendering` trait.

## Action Bindings

Supports all [action binding types](../features/action-bindings.md).

## Related Service

- [weather (service)](../services/weather.md) — Open-Meteo API, geocoding, MCP tools

## Crate

- **Path**: `plugins/weather/`
- **Library**: `libsmearor_weather_widget.so`
- **Model**: `model/weather/`
