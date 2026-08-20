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

impl AppError {
    /// 构造一个携带显式 HTTP 状态码的错误（默认 500）。
    /// 用于把"未实现"等明确语义以正确的状态码返回，而不是静默成功或一律 500。
    pub fn with_status(status: StatusCode, err: anyhow::Error) -> Self {
        Self(err).set_status(status)
    }

    pub fn set_status(mut self, status: StatusCode) -> Self {
        // 把状态码编码进 anyhow 链，便于在 IntoResponse 中恢复。
        self.0 = anyhow::anyhow!("{}::{}", status.as_u16(), self.0);
        self
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let full = self.0.to_string();
        let status = full
            .split_once("::")
            .and_then(|(code, _)| code.parse::<u16>().ok())
            .and_then(|code| StatusCode::from_u16(code).ok())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let message = self.0.to_string();
        tracing::error!("API error ({}): {}", status.as_u16(), message);
        (status, Json(json!({ "error": message }))).into_response()
    }
}
