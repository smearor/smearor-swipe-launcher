# http (Service)

HTTP client service for making outbound HTTP requests from plugins.

## Description

The HTTP service provides a generic HTTP client that plugins can use to make GET, POST, PUT, and DELETE requests. It uses `reqwest` for HTTP communication and
supports request/response correlation via correlation IDs.

## Topics

| Topic                                    | Direction        | Description                      |
|------------------------------------------|------------------|----------------------------------|
| `service.http.request`                   | Widget → Service | HTTP request with correlation ID |
| `service.http.response.{correlation_id}` | Service → Widget | HTTP response                    |

## MCP Tools

| Tool        | Description               |
|-------------|---------------------------|
| `http_get`  | Make an HTTP GET request  |
| `http_post` | Make an HTTP POST request |

## Configuration

```toml
[[services]]
id = "http"
path = "target/release/libsmearor_http_service.so"
```

## Crate

- **Path**: `services/http/`
- **Library**: `libsmearor_http_service.so`
- **Model**: `model/http/`
