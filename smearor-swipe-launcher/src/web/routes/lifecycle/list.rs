use crate::web::routes::lifecycle::LifecycleResponse;
use crate::web::routes::lifecycle::send_error_response;
use crate::web::routes::lifecycle::timeout_response;
use crate::web::state::WebAppState;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use std::sync::Arc;

/// GET `/api/instances` — list all instances with lifecycle state.
pub async fn api_list_instances(State(state): State<Arc<WebAppState>>) -> impl IntoResponse {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let command: smearor_mcp_server::McpCommand = smearor_mcp_server::CommandResponseWrapper::builder()
        .params(smearor_mcp_server::ListInstancesParams::builder().build())
        .response(tx)
        .build()
        .into();
    if state.mcp_command_sender.send(command).await.is_err() {
        return send_error_response();
    }
    match tokio::time::timeout(std::time::Duration::from_secs(10), rx).await {
        Ok(Ok(Ok(msg))) => {
            let parsed: serde_json::Value = serde_json::from_str(&msg).unwrap_or(serde_json::json!([]));
            (StatusCode::OK, Json(parsed)).into_response()
        }
        Ok(Ok(Err(e))) => (StatusCode::INTERNAL_SERVER_ERROR, Json(LifecycleResponse { ok: false, message: e })).into_response(),
        _ => timeout_response(),
    }
}
