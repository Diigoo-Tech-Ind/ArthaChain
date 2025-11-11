/// Advanced Features Module
/// Model retraining orchestration, deprecation feed API, CORS middleware

pub mod model_retraining;
pub mod deprecation_feed;
pub mod cors_middleware;

use axum::{
    http::{header, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

/// Build CORS layer for API
pub fn build_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::ACCEPT,
            header::CONTENT_TYPE,
            header::HeaderName::from_static("x-artha-session"),
            header::HeaderName::from_static("x-artha-did"),
        ])
        .expose_headers([
            header::CONTENT_LENGTH,
            header::CONTENT_TYPE,
            header::HeaderName::from_static("x-ratelimit-remaining"),
            header::HeaderName::from_static("x-ratelimit-reset"),
        ])
        .max_age(std::time::Duration::from_secs(3600))
}

/// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

