use axum::{
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// API 响应包装
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Json<Self> {
        Json(Self {
            success: true,
            data: Some(data),
            error: None,
        })
    }
}

impl ApiResponse<()> {
    pub fn err(message: impl Into<String>) -> Json<Self> {
        Json(Self {
            success: false,
            data: None,
            error: Some(message.into()),
        })
    }
}

// 统一错误处理辅助
pub fn into_response<T: Serialize>(result: Result<T, String>) -> Response {
    match result {
        Ok(data) => ApiResponse::ok(data).into_response(),
        Err(e) => ApiResponse::err(e).into_response(),
    }
}

pub async fn request_logger(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = std::time::Instant::now();
    let response = next.run(req).await;
    let duration = start.elapsed();
    tracing::info!(
        "{} {} - status: {}, latency: {}ms",
        method,
        uri,
        response.status(),
        duration.as_millis()
    );
    response
}
