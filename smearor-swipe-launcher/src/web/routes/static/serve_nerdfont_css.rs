use axum::http::StatusCode;
use axum::response::IntoResponse;

pub async fn serve_static_nerdfont_css() -> impl IntoResponse {
    let css = include_str!("../../../../../resources/web/nerdfont.css");
    (StatusCode::OK, [("content-type", "text/css")], css)
}
