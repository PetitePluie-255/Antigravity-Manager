use super::common::ApiResponse;
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response},
};
use std::sync::Arc;

#[derive(serde::Deserialize)]
pub struct LogQueryParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(serde::Serialize)]
pub struct LogsResponse {
    pub logs: Vec<crate::proxy::log_store::ProxyLogEntry>,
    pub total: usize,
}

pub async fn get_proxy_logs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<LogQueryParams>,
) -> Response {
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);
    let logs = state.log_store.get_logs(limit, offset);
    let total = state.log_store.len();

    ApiResponse::ok(LogsResponse { logs, total }).into_response()
}

pub async fn clear_proxy_logs(State(state): State<Arc<AppState>>) -> Response {
    state.log_store.clear();
    ApiResponse::ok(()).into_response()
}
