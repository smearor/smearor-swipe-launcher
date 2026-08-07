use axum::http::StatusCode;
use axum::response::IntoResponse;

pub async fn serve_static_js() -> impl IntoResponse {
    let js = include_str!("../../../../../resources/web/app.js");
    (StatusCode::OK, [("content-type", "application/javascript")], js)
}
