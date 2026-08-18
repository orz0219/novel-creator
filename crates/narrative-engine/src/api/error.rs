//! API Error handling

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub struct AppError(pub anyhow::Error);

impl<E: Into<anyhow::Error>> From<E> for AppError {
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let message = self.0.to_string();
        tracing::error!("API error: {}", message);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": message }))).into_response()
    }
}
