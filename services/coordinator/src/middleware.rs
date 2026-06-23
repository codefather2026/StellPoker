//! Request logging middleware for the coordinator.
//!
//! Logs every HTTP request with:
//!   - UUID v4 request ID for log correlation
//!   - Method, path, status code, and duration
//!   - Session ID if extractable from the path
//!
//! Request bodies are never logged (may contain sensitive game state).
//!
//! Format is controlled by REQUEST_LOG_FORMAT=json|pretty (default: pretty).

use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use uuid::Uuid;

/// Extract a session/table ID from a path like `/api/table/42/...` or
/// `/api/session/some-uuid/...`.
fn extract_session_id(path: &str) -> Option<String> {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    // /api/table/:id/...
    if let Some(idx) = segments.iter().position(|&s| s == "table") {
        if let Some(id) = segments.get(idx + 1) {
            return Some(id.to_string());
        }
    }
    // /api/session/:id/...
    if let Some(idx) = segments.iter().position(|&s| s == "session") {
        if let Some(id) = segments.get(idx + 1) {
            return Some(id.to_string());
        }
    }
    None
}

pub async fn log_request(request: Request<Body>, next: Next) -> Response {
    let request_id = Uuid::new_v4().to_string();
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let session_id = extract_session_id(&path);
    let start = Instant::now();

    let response = next.run(request).await;

    let status = response.status();
    let duration_ms = start.elapsed().as_millis();

    match status {
        s if s.is_server_error() => {
            tracing::error!(
                request_id = %request_id,
                method = %method,
                path = %path,
                status = status.as_u16(),
                duration_ms = duration_ms,
                session_id = session_id.as_deref().unwrap_or("-"),
                "request completed"
            );
        }
        s if s == StatusCode::BAD_REQUEST
            || s == StatusCode::UNAUTHORIZED
            || s == StatusCode::FORBIDDEN
            || s == StatusCode::NOT_FOUND
            || s == StatusCode::TOO_MANY_REQUESTS =>
        {
            tracing::warn!(
                request_id = %request_id,
                method = %method,
                path = %path,
                status = status.as_u16(),
                duration_ms = duration_ms,
                session_id = session_id.as_deref().unwrap_or("-"),
                "request completed"
            );
        }
        _ => {
            tracing::info!(
                request_id = %request_id,
                method = %method,
                path = %path,
                status = status.as_u16(),
                duration_ms = duration_ms,
                session_id = session_id.as_deref().unwrap_or("-"),
                "request completed"
            );
        }
    }

    response
}
