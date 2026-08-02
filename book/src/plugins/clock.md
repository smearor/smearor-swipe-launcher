# clock (Plugin)

A clock widget that displays the current time and optionally a date. Supports configurable formats and timezones.

## Description

The clock widget updates periodically using `glib::MainContext::spawn_local` for GTK updates and `tokio::sync::mpsc` for message reception. It displays the time
in the configured format and timezone.

## Configuration

```toml
[clock_widget]
mode = "compact"
format = "[hour]:[minute]:[second]"
format_2 = "[day].[month].[year]"
timezone = "local"
click_topic = "area.open"
click_payload = { area_id = "clock_area" }
```

| Field       | Type             | Description                                 |
|-------------|------------------|---------------------------------------------|
| `mode`      | `WidgetMode`     | `compact` (vertical) or `wide` (horizontal) |
| `format`    | `Option<String>` | Time format string (first line)             |
| `format_2`  | `Option<String>` | Date format string (second line)            |
| `timezone`  | `String`         | `local` or a timezone identifier            |
| `icon_size` | `i32`            | Icon size in pixels                         |
| `icon_only` | `bool`           | Show only the time, no icon                 |
| `max_width` | `Option<i32>`    | Maximum widget width                        |

## Format Syntax

The format string uses bracketed tokens:

| Token       | Replaced with  |
|-------------|----------------|
| `[hour]`    | Hour (24-hour) |
| `[minute]`  | Minute         |
| `[second]`  | Second         |
| `[day]`     | Day of month   |
| `[month]`   | Month number   |
| `[year]`    | Year           |
| `[weekday]` | Weekday name   |

## Action Bindings

Supports all [action binding types](../features/action-bindings.md).

## Crate

- **Path**: `plugins/clock/`
- **Library**: `libsmearor_clock_widget.so`
- **Model**: `model/clock/`
