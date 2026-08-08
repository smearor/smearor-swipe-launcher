use crate::web::routes::lifecycle::LoadInstanceRequest;
use crate::web::routes::lifecycle::await_mcp_response;
use crate::web::routes::lifecycle::send_error_response;
use crate::web::state::WebAppState;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use std::sync::Arc;

/// POST `/api/instances` — load a new launcher instance.
pub async fn api_load_instance(State(state): State<Arc<WebAppState>>, Json(request): Json<LoadInstanceRequest>) -> impl IntoResponse {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let command: smearor_mcp_server::McpCommand = smearor_mcp_server::CommandResponseWrapper::builder()
        .params(
            smearor_mcp_server::LoadInstanceParams::builder()
                .instance_id(request.instance_id)
                .config_path(request.config_path)
                .instance_type(request.instance_type)
                .persist(request.persist)
                .build(),
        )
        .response(tx)
        .build()
        .into();
    if state.mcp_command_sender.send(command).await.is_err() {
        return send_error_response();
    }
    await_mcp_response(rx, StatusCode::BAD_REQUEST).await
}
