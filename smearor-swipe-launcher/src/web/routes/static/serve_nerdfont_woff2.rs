use axum::http::StatusCode;
use axum::response::IntoResponse;

pub async fn serve_static_nerdfont_woff2() -> impl IntoResponse {
    let font = include_bytes!("../../../../../resources/web/nerdfont.woff2");
    (StatusCode::OK, [("content-type", "font/woff2")], font.as_slice())
}
