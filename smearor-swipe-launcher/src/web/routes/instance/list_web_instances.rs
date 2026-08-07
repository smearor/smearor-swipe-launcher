use crate::instance::InstanceType;
use crate::web::routes::instance::WebInstanceInfo;
use crate::web::state::WebAppState;
use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use std::sync::Arc;

/// GET `/instances` — list all web instances as JSON.
pub async fn list_web_instances(State(state): State<Arc<WebAppState>>) -> impl IntoResponse {
    let instances = state.instances.lock();
    let Ok(instances) = instances else {
        return Json(Vec::<WebInstanceInfo>::new());
    };

    let list: Vec<WebInstanceInfo> = instances
        .values()
        .filter(|i| i.instance_type == InstanceType::Web)
        .map(|i| WebInstanceInfo {
            instance_id: i.instance_id.clone(),
            instance_type: i.instance_type.as_str().to_string(),
        })
        .collect();

    Json(list)
}
