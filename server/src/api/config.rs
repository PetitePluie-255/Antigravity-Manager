use axum::{
    extract::{Json, State},
    response::Response,
};
use std::sync::Arc;

use super::common::into_response;
use crate::core::models::AppConfig;
use crate::core::storage::ConfigStorage;
use crate::state::AppState;

pub async fn load_config(State(state): State<Arc<AppState>>) -> Response {
    into_response(ConfigStorage::load(&state.db_pool, &state.storage).await)
}

pub async fn save_config(
    State(state): State<Arc<AppState>>,
    Json(config): Json<AppConfig>,
) -> Response {
    into_response(ConfigStorage::save(&state.db_pool, &config).await)
}
