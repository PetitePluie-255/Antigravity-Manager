use axum::{
    extract::{Json, State},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::common::{into_response, ApiResponse};
use crate::core::models::ProxyConfig;
use crate::state::AppState;

#[derive(Serialize)]
pub struct ProxyStatus {
    pub running: bool,
    pub port: u16, // Configured port (or simulated)
    pub base_url: String,
    pub active_accounts: usize,
}

#[derive(Deserialize)]
pub struct StartProxyRequest {
    pub config: ProxyConfig,
}

pub async fn start_proxy(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StartProxyRequest>,
) -> Response {
    // 1. Update Configuration
    let config = req.config;

    // Check if we want to allow re-start or just update?
    // User might click start when stopped.

    // Update State
    state.proxy_enabled.store(config.enabled, Ordering::SeqCst);
    state.proxy_port.store(config.port, Ordering::SeqCst);
    state
        .request_timeout
        .store(config.request_timeout, Ordering::SeqCst);

    // Update Mappings
    *state.anthropic_mapping.write().await = config.anthropic_mapping.clone();
    *state.openai_mapping.write().await = config.openai_mapping.clone();
    *state.custom_mapping.write().await = config.custom_mapping.clone();

    // Update Upstream
    let upstream_cfg = crate::proxy::config::UpstreamProxyConfig {
        enabled: config.upstream_proxy.enabled,
        url: config.upstream_proxy.url.clone(),
    };
    *state.upstream_proxy.write().await = upstream_cfg.clone();
    // Note: UpstreamClient is recreated when proxy restarts, no update_config needed

    // Reload Tokens
    match state.token_manager.load_accounts().await {
        Ok(count) => {
            tracing::info!("Proxy started/updated. Loaded {} accounts.", count);
            let status = ProxyStatus {
                running: true,
                port: config.port,
                base_url: format!("http://127.0.0.1:{}", config.port), // Mimic existing response
                active_accounts: count,
            };
            ApiResponse::ok(status).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to load accounts: {}", e);
            ApiResponse::err(format!("Starting proxy failed: {}", e)).into_response()
        }
    }
}

pub async fn stop_proxy(State(state): State<Arc<AppState>>) -> Response {
    state.proxy_enabled.store(false, Ordering::SeqCst);
    state.proxy_port.store(0, Ordering::SeqCst);
    ApiResponse::ok("Proxy stopped").into_response()
}

pub async fn get_proxy_status(State(state): State<Arc<AppState>>) -> Response {
    let running = state.proxy_enabled.load(Ordering::SeqCst);
    let port = state.proxy_port.load(Ordering::SeqCst);
    let active_accounts = state.token_manager.len();

    let status = ProxyStatus {
        running,
        port,
        base_url: if running {
            format!("http://127.0.0.1:{}", port)
        } else {
            "".to_string()
        },
        active_accounts,
    };
    ApiResponse::ok(status).into_response()
}

pub async fn generate_api_key(State(_state): State<Arc<AppState>>) -> Response {
    let key = format!("sk-{}", uuid::Uuid::new_v4().simple());
    ApiResponse::ok(key).into_response()
}

pub async fn update_model_mapping(
    State(state): State<Arc<AppState>>,
    Json(proxy_config): Json<ProxyConfig>,
) -> Response {
    use crate::core::storage::ConfigStorage;
    // use crate::core::traits::StorageConfig;

    match ConfigStorage::load(&state.db_pool, &state.storage).await {
        Ok(mut config) => {
            // Update in-memory state as well if running
            if state.proxy_enabled.load(Ordering::Relaxed) {
                *state.anthropic_mapping.write().await = proxy_config.anthropic_mapping.clone();
                *state.openai_mapping.write().await = proxy_config.openai_mapping.clone();
                *state.custom_mapping.write().await = proxy_config.custom_mapping.clone();
            }

            config.proxy.anthropic_mapping = proxy_config.anthropic_mapping;
            config.proxy.openai_mapping = proxy_config.openai_mapping;
            config.proxy.custom_mapping = proxy_config.custom_mapping;

            into_response(ConfigStorage::save(&state.db_pool, &config).await)
        }
        Err(e) => ApiResponse::err(format!("加载配置失败: {}", e)).into_response(),
    }
}
