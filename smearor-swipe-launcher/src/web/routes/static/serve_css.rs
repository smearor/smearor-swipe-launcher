use axum::http::StatusCode;
use axum::response::IntoResponse;

pub async fn serve_static_css() -> impl IntoResponse {
    let css = include_str!("../../../../../resources/web/style.css");
    (StatusCode::OK, [("content-type", "text/css")], css)
}
