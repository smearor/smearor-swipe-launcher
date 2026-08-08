use crate::web::routes::lifecycle::await_mcp_response;
use crate::web::routes::lifecycle::send_error_response;
use crate::web::state::WebAppState;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use std::sync::Arc;

/// POST `/api/instances/{id}/reload` — reload an instance (stop + unload + load + restore state).
pub async fn api_reload_instance(Path(instance_id): Path<String>, State(state): State<Arc<WebAppState>>) -> impl IntoResponse {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let command: smearor_mcp_server::McpCommand = smearor_mcp_server::CommandResponseWrapper::builder()
        .params(smearor_mcp_server::ReloadInstanceParams::builder().instance_id(instance_id).build())
        .response(tx)
        .build()
        .into();
    if state.mcp_command_sender.send(command).await.is_err() {
        return send_error_response();
    }
    await_mcp_response(rx, StatusCode::BAD_REQUEST).await
}
