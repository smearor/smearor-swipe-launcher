# Concept: HTTP Service Plugin

This document describes the concept for the **HTTP Service**, a singleton background service that executes outbound HTTP requests on behalf of other plugins and
returns the JSON response via the central message broker. It is a pure logic service and contains no GTK code.

Use cases include fetching weather data from online services and querying or controlling IoT devices in the local network. Concrete examples include
controlling Shelly devices such as turning a light on or off via a Shelly 1, changing the color of an RGBW stripe via a Shelly RGBW2, or toggling a fan via a
Shelly

1.

---

## 1. HTTP Service

### 1.1 Overview

The HTTP Service is a singleton background service that listens on a request topic, performs the configured HTTP call, and publishes the JSON response back to a
topic that the caller controls. It contains no GTK code and is therefore completely decoupled from the user interface.

The service enforces a URL whitelist to prevent accidental or malicious outbound requests. Only URLs matching one of the configured patterns are allowed. The
wildcard character `*` is supported for flexible host or path restrictions.

### 1.2 Message Flow

```
+-------------------------+        +-------------------------+        +-------------------------+
| Widget / Caller         |        | HTTP Service            |        | External Endpoint       |
|                         |        | (Singleton)             |        |                         |
|                         |        |                         |        |                         |
| 1. Publish request on   |=======>| 2. Validate URL against |=======>| 3. Execute HTTP call    |
|    service.http.request |        |    whitelist            |        |                         |
|                         |        |                         |        |                         |
| 4. Receive response on  |<=======| 5. Publish response JSON  |<=======|                         |
|    caller-defined topic |        |    to response topic    |        |                         |
+-------------------------+        +-------------------------+        +-------------------------+
```

### 1.3 Crate Structure

The HTTP functionality is split into two separate crates:

| Crate       | Path             | Responsibility                                                  |
|-------------|------------------|-----------------------------------------------------------------|
| **Model**   | `model/http/`    | Shared structs, enums, and message formats for request/response |
| **Service** | `services/http/` | Backend logic, HTTP client, whitelist validation, and broadcast |

### 1.4 Additional Use Cases

Beyond the Shelly device control described in the configuration examples, the HTTP service can drive many other integrations:

- **Home Assistant:** Query entity states or trigger automations and scenes via the Home Assistant REST API.
- **Weather and forecast services:** Fetch current conditions, hourly forecasts, or severe weather alerts from public APIs.
- **Smart thermostats and heating:** Read room temperatures or set target temperatures on devices like Tado, Bosch, or generic ESP-based controllers.
- **Smart blinds and shutters:** Open, close, or set positions via HTTP endpoints on motorized shutter controllers.
- **Media centers:** Control Kodi, Jellyfin, or other media players through their HTTP APIs (play, pause, volume, current title).
- **Printer and 3D printer status:** Query OctoPrint or Klipper for temperatures, progress, or job state.
- **Energy monitoring:** Read power consumption from smart meters, Shelly EM, or inverter APIs.
- **Server and NAS health:** Fetch system status, disk usage, or running service state from a local home server.
- **Calendar and task services:** Pull the next appointment or task list from a self-hosted CalDAV/CardDAV or task service.

Because the service exposes a generic HTTP client with a configurable URL whitelist, any local or remote HTTP endpoint can be integrated with a matching Button
Widget
or a dedicated future widget.

### 1.5 Weather and Astronomy Data Visualization

The circular-ring gauge style used elsewhere in the launcher (for example, for system metrics) is an excellent template for displaying weather and astronomy
data.
The HTTP service can fetch this data from free APIs and publish it on dedicated topics, so a future weather/astronomy widget can render it in the same visual
style.

#### 1.5.1 Visualizable Data

| Data                        | Widget Style              | Description                                                                                         |
|-----------------------------|---------------------------|-----------------------------------------------------------------------------------------------------|
| Sunrise / Sunset            | Text left/right of a ring | Fixed time values displayed around a circular gauge                                                 |
| Day length                  | Ring percentage           | 100% = 24 hours; ring fills to the actual day length (e.g., 16 hours in summer ≈ 67%)               |
| Current sun position        | Progress ring             | Ring fills over the course of the day; at noon the indicator is at the midpoint of the daylight arc |
| Moon illumination           | Percentage + ring         | Same style as CPU/memory gauges (e.g., 78%)                                                         |
| Moon phase                  | Icon in ring center       | Glowing icon changes with phase: crescent, half, full circle                                        |
| Moonrise / Moonset          | Text values               | Useful for night photography or sky observation                                                     |
| Rain probability / Humidity | Ring percentage           | Color-coded fill segment                                                                            |
| UV index                    | Filled ring with color    | Green (low) → Yellow → Red (extreme)                                                                |
| Wind speed                  | Ring relative to max      | Filled relative to a configurable maximum (e.g., 60 km/h)                                           |

#### 1.5.2 Recommended APIs

Two free APIs that require no registration or API key are recommended:

**Option A: Open-Meteo (all-in-one)**

- Combines weather, sun, and moon data in one JSON response.
- No key or account required.
- Example request: `https://api.open-meteo.com/v1/forecast?latitude=52.52&longitude=13.41&daily=sunrise,sunset,moon_phase&timezone=auto`
- Relevant parameters:
    - `daily=sunrise,sunset`: exact sunrise and sunset times.
    - `daily=moon_phase`: float between 0.0 and 1.0 (0.0/1.0 = new moon, 0.25 = first quarter, 0.5 = full moon, 0.75 = last quarter).

**Option B: Sunrise-Sunset.org (astronomy focused)**

- Specialized, stable, open API for sun data.
- Provides `day_length` in seconds directly.
- Example request: `https://api.sunrise-sunset.org/json?lat=52.52&lng=13.41&formatted=0`
- `formatted=0` returns ISO-8601 timestamps that are easy to parse in Rust.

#### 1.5.3 Example HTTP Service Request

A widget can request sunrise, sunset, and moon phase data via the HTTP service:

```json
Topic:   "service.http.request"
Payload: {
"method": "Get",
"url": "https://api.open-meteo.com/v1/forecast?latitude=52.52&longitude=13.41&daily=sunrise,sunset,moon_phase&timezone=auto",
"response_topic": "service.http.response.astronomy"
}
```

The response body is published on `service.http.response.astronomy` and can be parsed by the widget to update the ring gauges, text labels, and icons.

---

## 2. Model Crate (`model/http`)

### 2.1 Message Topics

The model crate declares the request topic and the response topic prefix. The response topic is provided by the caller in each request.

```rust
pub const TOPIC_HTTP_REQUEST: &str = "service.http.request";
pub const TOPIC_HTTP_RESPONSE_PREFIX: &str = "service.http.response";
```

### 2.2 HTTP Method

```rust
/// Supported HTTP methods for outbound requests.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub enum HttpMethod {
    /// Retrieve a resource.
    #[default]
    Get,
    /// Submit data to a resource.
    Post,
    /// Update a resource.
    Put,
    /// Delete a resource.
    Delete,
}
```

### 2.3 Request Message

```rust
/// Payload sent to the HTTP service to request an outbound HTTP call.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[stabby::stabby]
pub struct HttpRequestMessage {
    /// HTTP method to use.
    pub method: HttpMethod,
    /// Full URL to call. Must match the configured whitelist.
    pub url: String,
    /// Topic where the response should be published.
    pub response_topic: String,
    /// Optional JSON body for POST or PUT requests.
    pub body: Option<String>,
    /// Optional HTTP headers to send with the request.
    pub headers: Option<Vec<HttpHeader>>,
    /// Optional timeout in milliseconds. Defaults to the service configuration.
    pub timeout_ms: Option<u64>,
}

/// A single HTTP header name/value pair.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[stabby::stabby]
pub struct HttpHeader {
    /// Header name.
    pub name: String,
    /// Header value.
    pub value: String,
}
```

### 2.4 Response Message

```rust
/// Result status of an HTTP call.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub enum HttpResponseStatus {
    /// The call succeeded and the body is available.
    #[default]
    Success,
    /// The URL was not allowed by the whitelist.
    Forbidden,
    /// The HTTP call failed (network error, timeout, invalid response).
    Error,
}

/// Payload published by the HTTP service after a request completes.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct HttpResponseMessage {
    /// Final status of the request.
    pub status: HttpResponseStatus,
    /// HTTP status code returned by the server, when available.
    pub status_code: Option<u16>,
    /// Response body as a JSON string. Empty when the call failed or was forbidden.
    pub body: String,
    /// Human-readable error message when the status is not success.
    pub error_message: Option<String>,
    /// Original request URL, echoed back for correlation.
    pub url: String,
}
```

---

## 3. Service Crate (`services/http`)

### 3.1 File Structure

- `service.rs` - `HttpService` struct and trait implementations
- `config.rs` - `HttpServiceConfig` struct and parsing
- `whitelist.rs` - URL whitelist pattern matching
- `lib.rs` - `service_plugin!` macro invocation

### 3.2 Service Implementation

```rust
pub struct HttpService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: HttpServiceConfig,
    pub request_sender: tokio::sync::mpsc::UnboundedSender<HttpRequestMessage>,
}
```

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<HttpRequestMessage>>` - Processes incoming HTTP requests
- `MessageBroadcaster` - Broadcasts response messages to the broker
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to the core context
- `Service` - Routes raw FFI envelopes to the typed handler

### 3.3 Configuration

```rust
/// Configuration for the HTTP service.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct HttpServiceConfig {
    /// List of URL patterns that may be called. Wildcards `*` are supported.
    pub allowed_urls: Vec<String>,
    /// Default request timeout in milliseconds.
    pub default_timeout_ms: u64,
    /// Maximum allowed response body size in bytes.
    pub max_response_bytes: usize,
}

impl Default for HttpServiceConfig {
    fn default() -> Self {
        Self {
            allowed_urls: Vec::new(),
            default_timeout_ms: 10000,
            max_response_bytes: 1024 * 1024,
        }
    }
}
```

### 3.4 Whitelist Pattern Matching

Each requested URL must match at least one entry in `allowed_urls`. The service supports the wildcard `*`, which matches any sequence of characters. If the
whitelist is empty, all requests are rejected.

Examples:

| Whitelist Entry           | Matching URL                                 | Non-Matching URL                 |
|---------------------------|----------------------------------------------|----------------------------------|
| `192.168.178.*`           | `http://192.168.178.42/api/status`           | `http://10.0.0.1/api/status`     |
| `https://api.weather.*`   | `https://api.weather.example.com/v1/current` | `https://evil.example.com/`      |
| `http://localhost:8080/*` | `http://localhost:8080/lights/1`             | `http://localhost:9090/lights/1` |
| `*://*.home.local/*`      | `http://lights.home.local/api`               | `http://public.example.com/`     |

### 3.5 Background Request Handler

On construction, the service spawns a dedicated Tokio task. The task receives requests via a channel, validates the URL, executes the HTTP call, and broadcasts
the response to the caller's `response_topic`.

```rust
async fn run_request_handler(
    config: HttpServiceConfig,
    mut request_receiver: tokio::sync::mpsc::UnboundedReceiver<HttpRequestMessage>,
    broadcaster: Box<dyn MessageTopicBroadcaster>,
) {
    while let Some(request) = request_receiver.recv().await {
        if !is_url_allowed(&request.url, &config.allowed_urls) {
            let response = HttpResponseMessage {
                status: HttpResponseStatus::Forbidden,
                status_code: None,
                body: String::new(),
                error_message: Some(format!("URL is not whitelisted: {}", request.url)),
                url: request.url,
            };
            broadcaster.broadcast_topic(&request.response_topic, response);
            continue;
        }

        let timeout = Duration::from_millis(request.timeout_ms.unwrap_or(config.default_timeout_ms));
        let response = execute_http_request(&request, timeout, config.max_response_bytes).await;

        broadcaster.broadcast_topic(&request.response_topic, response);
    }
}
```

### 3.6 HTTP Request Execution

```rust
async fn execute_http_request(
    request: &HttpRequestMessage,
    timeout: Duration,
    max_response_bytes: usize,
) -> HttpResponseMessage {
    let client = match reqwest::Client::builder().timeout(timeout).build() {
        Ok(client) => client,
        Err(error) => {
            return HttpResponseMessage {
                status: HttpResponseStatus::Error,
                status_code: None,
                body: String::new(),
                error_message: Some(format!("Failed to build HTTP client: {error}")),
                url: request.url.clone(),
            };
        }
    };

    let mut builder = match request.method {
        HttpMethod::Get => client.get(&request.url),
        HttpMethod::Post => client.post(&request.url),
        HttpMethod::Put => client.put(&request.url),
        HttpMethod::Delete => client.delete(&request.url),
    };

    if let Some(headers) = &request.headers {
        for header in headers {
            builder = builder.header(&header.name, &header.value);
        }
    }

    if let Some(body) = &request.body {
        builder = builder.body(body.clone());
    }

    match builder.send().await {
        Ok(http_response) => {
            let status_code = http_response.status().as_u16();
            let body = match http_response.bytes().await {
                Ok(bytes) => {
                    let truncated = bytes.iter().take(max_response_bytes).copied().collect::<Vec<u8>>();
                    String::from_utf8_lossy(&truncated).to_string()
                }
                Err(error) => {
                    return HttpResponseMessage {
                        status: HttpResponseStatus::Error,
                        status_code: Some(status_code),
                        body: String::new(),
                        error_message: Some(format!("Failed to read response body: {error}")),
                        url: request.url.clone(),
                    };
                }
            };

            HttpResponseMessage {
                status: HttpResponseStatus::Success,
                status_code: Some(status_code),
                body,
                error_message: None,
                url: request.url.clone(),
            }
        }
        Err(error) => HttpResponseMessage {
            status: HttpResponseStatus::Error,
            status_code: None,
            body: String::new(),
            error_message: Some(format!("HTTP request failed: {error}")),
            url: request.url.clone(),
        },
    }
}
```

### 3.7 Whitelist Matcher

```rust
fn is_url_allowed(url: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }

    patterns.iter().any(|pattern| url_matches_pattern(url, pattern))
}

fn url_matches_pattern(url: &str, pattern: &str) -> bool {
    let regex_pattern = pattern.replace(".", r"\.").replace("*", ".*");
    let Ok(regex) = regex::Regex::new(&format!("^{regex_pattern}$")) else {
        return false;
    };
    regex.is_match(url)
}
```

### 3.8 Required Trait Implementations

```rust
impl MessageHandler<FfiEnvelopePayload<HttpRequestMessage>> for HttpService {
    fn handle_message(&self, message: FfiEnvelopePayload<HttpRequestMessage>, _sender_id: &str) {
        let _ = self.request_sender.send(message.into_inner());
    }
}

impl MessageBroadcaster for HttpService {}

impl PluginMetaGetter for HttpService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for HttpService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Service for HttpService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if message.is_null() {
            return;
        }
        unsafe {
            let envelope = &*(message as *mut FfiEnvelope);
            if envelope.type_id == FfiEnvelopePayload::<HttpRequestMessage>::TYPE_ID {
                MessageHandler::<FfiEnvelopePayload<HttpRequestMessage>>::handle_envelope_message(self, envelope);
            }
        }
    }
}
```

---

## 4. Configuration Example

### 4.1 Service Registration in `services.toml`

```toml
[[services]]
id = "http"
path = "target/release/libsmearor_http_service.so"

[http]
allowed_urls = [
    "192.168.178.*",
    "https://api.weather.example.com/*",
    "http://localhost:8080/*",
]
default_timeout_ms = 10000
max_response_bytes = 1048576
```

### 4.2 Weather Data Query

A widget sends a request to fetch current weather data:

```json
Topic:   "service.http.request"
Payload: {
"method": "Get",
"url": "https://api.weather.example.com/v1/current",
"response_topic": "service.http.response.weather",
"headers": [
{"name": "Accept", "value": "application/json"}
]
}
```

The service publishes the response:

```json
Topic:   "service.http.response.weather"
Payload: {
"status": "Success",
"status_code": 200,
"body": "{\"temperature\": 24.5, \"condition\": \"Sunny\"}",
"error_message": null,
"url": "https://api.weather.example.com/v1/current"
}
```

### 4.3 IoT Device Control

A widget sends a request to turn on a light in the local network:

```json
Topic:   "service.http.request"
Payload: {
"method": "Put",
"url": "http://192.168.178.42/api/lights/1",
"response_topic": "service.http.response.lights",
"body": "{\"state\": \"on\"}",
"headers": [
{"name": "Content-Type", "value": "application/json"}
]
}
```

### 4.4 Shelly Device Control

The HTTP service is designed to control Shelly devices in the local network. The following examples show the most common operations.

#### 4.4.1 Light On/Off via Shelly 1

A widget sends a request to turn on a light that is connected to the Shelly 1 at `192.168.178.71`:

```json
Topic:   "service.http.request"
Payload: {
"method": "Get",
"url": "http://192.168.178.71/relay/0?turn=on",
"response_topic": "service.http.response.shelly.light"
}
```

To turn the light off, the same request is sent with `turn=off`:

```json
Topic:   "service.http.request"
Payload: {
"method": "Get",
"url": "http://192.168.178.71/relay/0?turn=off",
"response_topic": "service.http.response.shelly.light"
}
```

#### 4.4.2 RGBW Stripe via Shelly RGBW2

A widget sends a request to turn on the RGBW stripe at `192.168.178.31` and set it to red via the Shelly RGBW2:

```json
Topic:   "service.http.request"
Payload: {
"method": "Get",
"url": "http://192.168.178.31/color/0?turn=on&red=255&green=0&blue=0&gain=100",
"response_topic": "service.http.response.shelly.rgbw"
}
```

To change the color to white with reduced brightness, the request updates the color channels:

```json
Topic:   "service.http.request"
Payload: {
"method": "Get",
"url": "http://192.168.178.31/color/0?turn=on&red=255&green=255&blue=255&gain=50",
"response_topic": "service.http.response.shelly.rgbw"
}
```

To turn the stripe off, the request uses `turn=off`:

```json
Topic:   "service.http.request"
Payload: {
"method": "Get",
"url": "http://192.168.178.31/color/0?turn=off",
"response_topic": "service.http.response.shelly.rgbw"
}
```

#### 4.4.3 Fan On/Off via Shelly Plug S

A widget sends a request to turn on the fan that is connected to the Shelly Plug S at `192.168.178.39`:

```json
Topic:   "service.http.request"
Payload: {
"method": "Get",
"url": "http://192.168.178.39/relay/0?turn=on",
"response_topic": "service.http.response.shelly.fan"
}
```

To turn the fan off, the same request is sent with `turn=off`:

```json
Topic:   "service.http.request"
Payload: {
"method": "Get",
"url": "http://192.168.178.39/relay/0?turn=off",
"response_topic": "service.http.response.shelly.fan"
}
```

### 4.5 Button Configuration in `config.toml`

Each Shelly device is controlled by a dedicated Button Widget in `config.toml`. The buttons are loaded in the scroll band and send the HTTP request message on
`service.http.request` when clicked. Long-press is used to turn the device off again.

```toml
[scroll_band]
area_type = "scroll"
plugins = [
    { id = "shelly_light_button", path = "target/release/libsmearor_button_widget.so" },
    { id = "shelly_rgbw_button", path = "target/release/libsmearor_button_widget.so" },
    { id = "shelly_fan_button", path = "target/release/libsmearor_button_widget.so" },
]

[shelly_light_button]
text = "Light"
icon = "nf-md-ceiling_light_multiple"
icon_only = true
click_topic = "service.http.request"
click_payload = { method = "Get", url = "http://192.168.178.71/relay/0?turn=on", response_topic = "service.http.response.shelly.light" }
longpress_topic = "service.http.request"
longpress_payload = { method = "Get", url = "http://192.168.178.71/relay/0?turn=off", response_topic = "service.http.response.shelly.light" }

[shelly_rgbw_button]
text = "RGBW"
icon = "nf-md-led_strip"
icon_only = true
click_topic = "service.http.request"
click_payload = { method = "Get", url = "http://192.168.178.31/color/0?turn=on&red=255&green=0&blue=0&gain=100", response_topic = "service.http.response.shelly.rgbw" }
longpress_topic = "service.http.request"
longpress_payload = { method = "Get", url = "http://192.168.178.31/color/0?turn=off", response_topic = "service.http.response.shelly.rgbw" }

[shelly_fan_button]
text = "Fan"
icon = "nf-md-fan"
icon_only = true
click_topic = "service.http.request"
click_payload = { method = "Get", url = "http://192.168.178.39/relay/0?turn=on", response_topic = "service.http.response.shelly.fan" }
longpress_topic = "service.http.request"
longpress_payload = { method = "Get", url = "http://192.168.178.39/relay/0?turn=off", response_topic = "service.http.response.shelly.fan" }
```

The `click_payload` and `longpress_payload` are inline TOML tables that map directly to the `HttpRequestMessage` fields. When the button is clicked, the Button
Widget publishes the payload on `service.http.request`. The HTTP Service validates the URL against the whitelist, executes the HTTP call, and publishes the
response
on the configured `response_topic`.

---

## 5. Roadmap

This roadmap defines the recommended order, dependencies, and deliverables for implementing the HTTP Service feature.

### Phase 1: Foundation — Model Crate (`model/http`)

**Goal:** Define all shared messages, topics, and configuration types.

**Order:**

1. Create the crate `model/http` with a `Cargo.toml` that depends on `serde`, `stabby`, and the project plugin API.
2. Create `src/topics.rs` and declare `TOPIC_HTTP_REQUEST` and `TOPIC_HTTP_RESPONSE_PREFIX`.
3. Create `src/messages/method.rs` with the `HttpMethod` enum.
4. Create `src/messages/header.rs` with the `HttpHeader` struct.
5. Create `src/messages/request.rs` with the `HttpRequestMessage` struct.
6. Create `src/messages/response.rs` with the `HttpResponseStatus` enum and `HttpResponseMessage` struct.
7. Add `#[stabby::stabby]` to all FFI-relevant types.
8. Re-export all public types in `src/lib.rs`.
9. Run `cargo check` and `cargo test` for the model crate.

**Exit criteria:**

- The crate compiles without warnings.
- Every public struct and enum has English rustdoc documentation.
- `cargo test` passes with serialization/deserialization tests for each message.

---

### Phase 2: Backend — Service Crate (`services/http`)

**Goal:** Validate requests against the whitelist, execute HTTP calls, and publish responses.

**Dependencies:** Phase 1 must be complete.

**Order:**

1. Create the crate `services/http` with a `Cargo.toml` that depends on the `model/http` crate, the project plugin API, `reqwest`, `regex`, and `tokio`.
2. Create `src/config.rs` with `HttpServiceConfig` and its default values.
3. Create `src/whitelist.rs` and implement `is_url_allowed` and `url_matches_pattern` with full wildcard support.
4. Create `src/service.rs` with `HttpService` and all required trait implementations.
5. Implement `run_request_handler` to process requests asynchronously and broadcast responses to the caller's `response_topic`.
6. Implement `execute_http_request` using `reqwest` with timeout and body size limits.
7. Wire `service_plugin!` in `src/lib.rs`.
8. Add unit tests for the whitelist matcher covering exact matches, wildcards, and rejection cases.

**Exit criteria:**

- The service compiles and loads as a plugin.
- Unit tests for the whitelist matcher pass.
- The service rejects non-whitelisted URLs and returns a `Forbidden` response.
- A successful HTTP request publishes a `Success` response with the body on the requested response topic.

---

### Phase 3: Integration with Widgets

**Goal:** Allow widgets to send HTTP requests and react to responses.

**Dependencies:** Phase 1 and Phase 2 must be complete.

**Order:**

1. Extend the Button Widget to support a click action that publishes an `HttpRequestMessage` on `service.http.request`.
2. Add an optional configuration block to the Button Widget that defines the request method, URL, body, and response topic.
3. Document how a widget can subscribe to its own response topic and update its label from the `body` field using `label_topic` and `label_format`.
4. Add an integration test that sends a request to a local mock HTTP server and verifies the response topic receives the expected payload.

**Exit criteria:**

- A widget can trigger an HTTP request on click.
- The widget receives and displays the response body.
- The integration test passes end-to-end.
