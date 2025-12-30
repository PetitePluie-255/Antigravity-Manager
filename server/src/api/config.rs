use axum::{
    extract::{State, Json},
    response::Response,
};
use std::sync::Arc;

use crate::state::AppState;
use crate::core::storage::ConfigStorage;
use crate::core::models::AppConfig;
use super::common::into_response;

pub async fn load_config(State(state): State<Arc<AppState>>) -> Response {
    into_response(ConfigStorage::load(&state.storage))
}

pub async fn save_config(
    State(state): State<Arc<AppState>>,
    Json(config): Json<AppConfig>,
) -> Response {
    into_response(ConfigStorage::save(&state.storage, &config))
}
