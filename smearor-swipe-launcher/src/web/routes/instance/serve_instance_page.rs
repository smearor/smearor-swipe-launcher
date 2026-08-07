use crate::instance::InstanceType;
use crate::web::state::WebAppState;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Html;
use axum::response::IntoResponse;
use std::collections::HashMap;
use std::sync::Arc;

/// GET `/instances/{id}` — serve the composed HTML page for a web instance.
pub async fn serve_instance_page(Path(instance_id): Path<String>, State(state): State<Arc<WebAppState>>) -> impl IntoResponse {
    let widgets_html;
    let orientation;
    let template_path;

    {
        let instances = state.instances.lock();
        let Ok(instances) = instances else {
            return (StatusCode::INTERNAL_SERVER_ERROR, Html::from("Internal error")).into_response();
        };

        let Some(instance) = instances.get(&instance_id) else {
            return (StatusCode::NOT_FOUND, Html::from("Instance not found")).into_response();
        };

        if instance.instance_type != InstanceType::Web {
            return (StatusCode::BAD_REQUEST, Html::from("Instance is not a web instance")).into_response();
        }

        widgets_html = super::render::render_all_widgets_html(instance);
        orientation = match instance.config.layout.orientation {
            crate::config::area::orientation::Orientation::Horizontal => "horizontal",
            crate::config::area::orientation::Orientation::Vertical => "vertical",
        };
        template_path = instance.config.launcher.web_template.clone();
    }

    let mut placeholders = HashMap::new();
    placeholders.insert("instance_id".to_string(), instance_id);
    placeholders.insert("widgets".to_string(), widgets_html);
    placeholders.insert("orientation".to_string(), orientation.to_string());

    let html = state.template_engine.load_and_render(template_path.as_deref(), &placeholders);

    (StatusCode::OK, Html::from(html)).into_response()
}
