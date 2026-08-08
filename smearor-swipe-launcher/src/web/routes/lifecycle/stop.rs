use crate::web::routes::lifecycle::await_mcp_response;
use crate::web::routes::lifecycle::send_error_response;
use crate::web::state::WebAppState;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use smearor_mcp_server::CommandResponseWrapper;
use smearor_mcp_server::McpCommand;
use smearor_mcp_server::StopInstanceParams;
use std::sync::Arc;

/// POST `/api/instances/{id}/stop` — stop a running instance.
pub async fn api_stop_instance(Path(instance_id): Path<String>, State(state): State<Arc<WebAppState>>) -> impl IntoResponse {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let command: McpCommand = CommandResponseWrapper::builder()
        .params(StopInstanceParams::builder().instance_id(instance_id).build())
        .response(tx)
        .build()
        .into();
    if state.mcp_command_sender.send(command).await.is_err() {
        return send_error_response();
    }
    await_mcp_response(rx, StatusCode::CONFLICT).await
}
