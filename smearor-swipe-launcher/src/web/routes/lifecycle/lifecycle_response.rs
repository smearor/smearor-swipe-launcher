use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;

/// Standard response for all lifecycle API endpoints.
#[derive(Serialize)]
pub struct LifecycleResponse {
    /// Whether the operation succeeded.
    pub ok: bool,
    /// Human-readable status or error message.
    pub message: String,
}

/// Response when the MCP command channel is closed.
pub fn send_error_response() -> axum::response::Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(LifecycleResponse {
            ok: false,
            message: "Failed to send command".to_string(),
        }),
    )
        .into_response()
}

/// Response when the MCP command times out.
pub fn timeout_response() -> axum::response::Response {
    (
        StatusCode::GATEWAY_TIMEOUT,
        Json(LifecycleResponse {
            ok: false,
            message: "Timeout".to_string(),
        }),
    )
        .into_response()
}

/// Await a oneshot MCP response with a 10-second timeout.
///
/// `error_status` is the status code returned when the MCP command itself
/// reports an error (e.g. `CONFLICT` for start/stop, `NOT_FOUND` for unload).
pub async fn await_mcp_response(rx: tokio::sync::oneshot::Receiver<Result<String, String>>, error_status: StatusCode) -> axum::response::Response {
    match tokio::time::timeout(std::time::Duration::from_secs(10), rx).await {
        Ok(Ok(Ok(msg))) => (StatusCode::OK, Json(LifecycleResponse { ok: true, message: msg })).into_response(),
        Ok(Ok(Err(e))) => (error_status, Json(LifecycleResponse { ok: false, message: e })).into_response(),
        _ => timeout_response(),
    }
}
