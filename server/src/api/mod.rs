use crate::state::AppState;
use axum::{
    routing::{get, post, put},
    Router,
};
use std::sync::Arc;

mod account;
pub mod common;
mod config;
mod import;
mod logs;
mod proxy;

pub fn build_routes(state: Arc<AppState>) -> Router {
    Router::new()
        // Account
        .route(
            "/api/accounts",
            get(account::list_accounts).post(account::add_account),
        )
        .route(
            "/api/accounts/:id",
            axum::routing::delete(account::delete_account),
        )
        .route("/api/accounts/:id/quota", get(account::get_account_quota)) // Special route for quota
        .route("/api/accounts/batch-delete", post(account::delete_accounts))
        .route("/api/accounts/current", get(account::get_current_account))
        .route("/api/accounts/switch", post(account::switch_account))
        .route(
            "/api/accounts/quota/refresh/:id",
            get(account::refresh_account_quota),
        )
        .route(
            "/api/accounts/quota/refresh",
            post(account::refresh_all_quotas),
        )
        // Device Fingerprint
        .route(
            "/api/accounts/:id/device-profiles",
            get(account::get_device_profiles),
        )
        .route(
            "/api/accounts/:id/device-profiles/bind",
            post(account::bind_device_profile),
        )
        .route(
            "/api/accounts/:id/device-profiles/restore/:version_id",
            post(account::restore_device_version),
        )
        .route(
            "/api/accounts/:id/device-profiles/:version_id",
            axum::routing::delete(account::delete_device_version),
        )
        .route(
            "/api/device/preview-generate",
            get(account::preview_generate_profile),
        )
        .route(
            "/api/device/restore-original",
            post(account::restore_original_device),
        )
        // Import
        .route("/api/import/json", post(import::import_accounts_json))
        // Config
        .route(
            "/api/config",
            get(config::load_config).put(config::save_config),
        )
        // Proxy
        .route("/api/proxy/start", post(proxy::start_proxy))
        .route("/api/proxy/stop", post(proxy::stop_proxy))
        .route("/api/proxy/status", get(proxy::get_proxy_status))
        .route("/api/proxy/key/generate", post(proxy::generate_api_key))
        .route("/api/proxy/mapping", put(proxy::update_model_mapping))
        // Logs
        .route("/api/proxy/logs", get(logs::get_proxy_logs))
        .route("/api/proxy/logs/clear", post(logs::clear_proxy_logs))
        // Health
        .route("/healthz", get(|| async { "ok" }))
        .with_state(state)
}
