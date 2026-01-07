//! Web API 路由定义

use super::handlers;
use super::server::WebAppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;

/// 构建 API 路由
pub fn build_routes(state: Arc<WebAppState>) -> Router {
    let router = Router::new()
        // 账户管理 API
        .route(
            "/api/accounts",
            get(handlers::list_accounts).post(handlers::add_account),
        )
        .route("/api/accounts/:id", delete(handlers::delete_account))
        .route("/api/accounts/batch", delete(handlers::delete_accounts))
        .route(
            "/api/accounts/current",
            get(handlers::get_current_account).put(handlers::switch_account),
        )
        .route(
            "/api/accounts/:id/quota",
            get(handlers::get_account_quota).post(handlers::refresh_account_quota),
        )
        .route(
            "/api/accounts/quota/refresh",
            post(handlers::refresh_all_quotas),
        )
        .route("/api/accounts/export", get(handlers::export_accounts))
        // 配置管理 API
        .route(
            "/api/config",
            get(handlers::load_config).put(handlers::save_config),
        )
        // 代理服务控制 API
        .route("/api/proxy/start", post(handlers::start_proxy))
        .route("/api/proxy/stop", post(handlers::stop_proxy))
        .route("/api/proxy/status", get(handlers::get_proxy_status))
        .route("/api/proxy/key/generate", post(handlers::generate_api_key))
        .route("/api/proxy/mapping", put(handlers::update_model_mapping))
        // OAuth 登录 API
        .route("/api/oauth/start", post(handlers::start_oauth))
        .route("/api/oauth/callback", get(handlers::oauth_callback))
        .route("/api/oauth/status", get(handlers::get_oauth_status))
        // 导入 API
        .route("/api/import/json", post(handlers::import_accounts_json))
        .route("/api/import/file", post(handlers::import_accounts_file))
        // 日志 API
        .route("/api/proxy/logs", get(handlers::get_proxy_logs))
        .route("/api/proxy/logs/clear", post(handlers::clear_proxy_logs))
        // 系统相关 API
        .route("/api/system/data-dir", get(handlers::get_data_dir_path))
        .route(
            "/api/system/check-updates",
            get(handlers::check_for_updates),
        )
        .route("/api/logs/clear", post(handlers::clear_log_cache))
        // 健康检查
        .route("/healthz", get(handlers::health_check))
        // 数据库导入
        .route("/api/import/database", post(handlers::import_from_database));

    router.with_state(state)
}
