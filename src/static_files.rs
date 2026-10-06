use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    Router,
};
use tower_http::services::ServeDir;

pub fn static_router() -> Router {
    Router::new().nest_service("/assets", ServeDir::new("assets"))
}

pub async fn spa_fallback(req: Request<Body>) -> Response {
    let path = req.uri().path();
    if path.starts_with("/api") || path.starts_with("/assets") {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    }
    match tokio::fs::read_to_string("assets/index.html").await {
        Ok(content) => (StatusCode::OK, [("content-type", "text/html")], content).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}
