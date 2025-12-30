//! Web API 处理器

use axum::{
    extract::{Json, Path, Query, State},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::server::WebAppState;
use crate::core::models::{Account, AppConfig, ProxyConfig, TokenData};
use crate::core::services::{AccountService, QuotaService};
use crate::core::storage::ConfigStorage;
use crate::core::traits::StorageConfig;

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

/// 添加账户请求
#[derive(Deserialize)]
pub struct AddAccountRequest {
    pub email: String,
    pub name: Option<String>,
    pub refresh_token: String,
}

/// 切换账户请求
#[derive(Deserialize)]
pub struct SwitchAccountRequest {
    pub account_id: String,
}

/// 批量删除请求
#[derive(Deserialize)]
pub struct BatchDeleteRequest {
    pub account_ids: Vec<String>,
}

// ===== 账户管理 API =====

/// 列出所有账户
pub async fn list_accounts(State(state): State<Arc<WebAppState>>) -> Response {
    match AccountService::list_accounts(&state.storage) {
        Ok(accounts) => ApiResponse::ok(accounts).into_response(),
        Err(e) => ApiResponse::err(e).into_response(),
    }
}

/// 添加账户
pub async fn add_account(
    State(state): State<Arc<WebAppState>>,
    Json(req): Json<AddAccountRequest>,
) -> Response {
    // 刷新 token 获取 access_token 和邮箱
    match create_account_from_token(&req.refresh_token).await {
        Ok((token_data, actual_email)) => {
            // 使用传入的邮箱或从 token 获取的邮箱
            let final_email = if req.email.is_empty() {
                actual_email
            } else {
                req.email
            };

            match AccountService::add_account(
                &state.storage,
                &state.emitter,
                final_email,
                req.name,
                token_data,
            ) {
                Ok(account) => ApiResponse::ok(account).into_response(),
                Err(e) => ApiResponse::err(e).into_response(),
            }
        }
        Err(e) => ApiResponse::err(format!("Token 刷新失败: {}", e)).into_response(),
    }
}

/// 删除账户
pub async fn delete_account(
    State(state): State<Arc<WebAppState>>,
    Path(id): Path<String>,
) -> Response {
    match AccountService::delete_account(&state.storage, &state.emitter, &id) {
        Ok(()) => ApiResponse::ok(()).into_response(),
        Err(e) => ApiResponse::err(e).into_response(),
    }
}

/// 批量删除账户
pub async fn delete_accounts(
    State(state): State<Arc<WebAppState>>,
    Json(req): Json<BatchDeleteRequest>,
) -> Response {
    match AccountService::delete_accounts(&state.storage, &state.emitter, &req.account_ids) {
        Ok(()) => ApiResponse::ok(()).into_response(),
        Err(e) => ApiResponse::err(e).into_response(),
    }
}

/// 获取当前账户
pub async fn get_current_account(State(state): State<Arc<WebAppState>>) -> Response {
    match AccountService::get_current_account(&state.storage) {
        Ok(account) => ApiResponse::ok(account).into_response(),
        Err(e) => ApiResponse::err(e).into_response(),
    }
}

/// 切换当前账户
pub async fn switch_account(
    State(state): State<Arc<WebAppState>>,
    Json(req): Json<SwitchAccountRequest>,
) -> Response {
    match AccountService::switch_account(&state.storage, &state.emitter, &req.account_id) {
        Ok(()) => ApiResponse::ok(()).into_response(),
        Err(e) => ApiResponse::err(e).into_response(),
    }
}

/// 获取账户配额
pub async fn get_account_quota(
    State(state): State<Arc<WebAppState>>,
    Path(id): Path<String>,
) -> Response {
    match AccountService::load_account(&state.storage, &id) {
        Ok(account) => ApiResponse::ok(account.quota).into_response(),
        Err(e) => ApiResponse::err(e).into_response(),
    }
}

/// 刷新账户配额
pub async fn refresh_account_quota(
    State(state): State<Arc<WebAppState>>,
    Path(id): Path<String>,
) -> Response {
    match QuotaService::refresh_account_quota(&state.storage, &id).await {
        Ok(quota) => ApiResponse::ok(quota).into_response(),
        Err(e) => ApiResponse::err(e).into_response(),
    }
}

/// 刷新所有账户配额
pub async fn refresh_all_quotas(State(state): State<Arc<WebAppState>>) -> Response {
    match QuotaService::refresh_all_quotas(&state.storage).await {
        Ok((success, errors)) => ApiResponse::ok(serde_json::json!({
            "success_count": success,
            "error_count": errors
        }))
        .into_response(),
        Err(e) => ApiResponse::err(e).into_response(),
    }
}

/// 导出账户
pub async fn export_accounts(State(state): State<Arc<WebAppState>>) -> Response {
    match AccountService::export_accounts(&state.storage) {
        Ok(tokens) => ApiResponse::ok(tokens).into_response(),
        Err(e) => ApiResponse::err(e).into_response(),
    }
}

// ===== 配置管理 API =====

/// 加载配置
pub async fn load_config(State(state): State<Arc<WebAppState>>) -> Response {
    match ConfigStorage::load(&state.storage) {
        Ok(config) => ApiResponse::ok(config).into_response(),
        Err(e) => ApiResponse::err(e).into_response(),
    }
}

/// 保存配置
pub async fn save_config(
    State(state): State<Arc<WebAppState>>,
    Json(config): Json<AppConfig>,
) -> Response {
    match ConfigStorage::save(&state.storage, &config) {
        Ok(()) => ApiResponse::ok(()).into_response(),
        Err(e) => ApiResponse::err(e).into_response(),
    }
}

// ===== 代理服务控制 API =====

/// 启动代理请求包装
#[derive(Deserialize)]
pub struct StartProxyRequest {
    pub config: ProxyConfig,
}

/// 启动代理服务
pub async fn start_proxy(
    State(state): State<Arc<WebAppState>>,
    Json(req): Json<StartProxyRequest>,
) -> Response {
    match state.proxy_manager.start(req.config).await {
        Ok(status) => ApiResponse::ok(status).into_response(),
        Err(e) => ApiResponse::err(e).into_response(),
    }
}

/// 停止代理服务
pub async fn stop_proxy(State(state): State<Arc<WebAppState>>) -> Response {
    match state.proxy_manager.stop().await {
        Ok(()) => ApiResponse::ok(()).into_response(),
        Err(e) => ApiResponse::err(e).into_response(),
    }
}

/// 获取代理状态
pub async fn get_proxy_status(State(state): State<Arc<WebAppState>>) -> Response {
    let status = state.proxy_manager.status().await;
    ApiResponse::ok(status).into_response()
}

/// 生成 API Key
pub async fn generate_api_key(State(_state): State<Arc<WebAppState>>) -> Response {
    let key = format!("sk-{}", uuid::Uuid::new_v4().simple());
    ApiResponse::ok(key).into_response()
}

/// 更新模型映射
pub async fn update_model_mapping(
    State(state): State<Arc<WebAppState>>,
    Json(proxy_config): Json<ProxyConfig>,
) -> Response {
    // 加载现有配置
    match ConfigStorage::load(&state.storage) {
        Ok(mut config) => {
            // 更新模型映射相关字段
            config.proxy.anthropic_mapping = proxy_config.anthropic_mapping;
            config.proxy.openai_mapping = proxy_config.openai_mapping;
            config.proxy.custom_mapping = proxy_config.custom_mapping;

            // 保存配置
            match ConfigStorage::save(&state.storage, &config) {
                Ok(()) => ApiResponse::ok(()).into_response(),
                Err(e) => ApiResponse::err(e).into_response(),
            }
        }
        Err(e) => ApiResponse::err(format!("加载配置失败: {}", e)).into_response(),
    }
}

// ===== 健康检查 =====

/// 健康检查
pub async fn health_check() -> Response {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
    .into_response()
}

// ===== 系统相关 API =====

/// 获取数据目录路径
pub async fn get_data_dir_path(State(state): State<Arc<WebAppState>>) -> Response {
    let path = state.storage.data_dir().to_string_lossy().to_string();
    ApiResponse::ok(path).into_response()
}

/// 检查更新
pub async fn check_for_updates() -> Response {
    // TODO: 实现版本检查逻辑，暂时返回当前版本无更新
    let current = env!("CARGO_PKG_VERSION");
    ApiResponse::ok(serde_json::json!({
        "has_update": false,
        "latest_version": current,
        "current_version": current,
        "download_url": "https://github.com/lbjlaq/Antigravity-Manager/releases"
    }))
    .into_response()
}

/// 清除日志缓存
pub async fn clear_log_cache() -> Response {
    // Web 模式下不需要清理本地日志缓存
    ApiResponse::ok(()).into_response()
}

// ===== OAuth 登录 API =====

/// 开始 OAuth 授权
#[derive(Serialize)]
pub struct StartOAuthResponse {
    pub auth_url: String,
    pub redirect_uri: String,
}

pub async fn start_oauth(State(state): State<Arc<WebAppState>>) -> Response {
    // 从环境变量获取回调基础 URL，默认为本地开发地址
    let base_url = std::env::var("OAUTH_CALLBACK_BASE")
        .or_else(|_| std::env::var("BASE_URL"))
        .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
    let redirect_uri = format!("{}/api/oauth/callback", base_url.trim_end_matches('/'));

    // 生成授权 URL
    let auth_url = crate::core::services::oauth::get_auth_url(&redirect_uri);

    // 保存待处理状态
    {
        let mut pending = state.oauth_pending.write().await;
        *pending = Some(super::server::PendingOAuth {
            redirect_uri: redirect_uri.clone(),
            created_at: chrono::Utc::now().timestamp(),
        });
    }

    // 重置结果状态
    {
        let mut result = state.oauth_result.write().await;
        *result = super::server::OAuthResult::Pending;
    }

    ApiResponse::ok(StartOAuthResponse {
        auth_url,
        redirect_uri,
    })
    .into_response()
}

/// OAuth 回调处理 (Google 重定向到这里)
#[derive(Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: Option<String>,
    pub error: Option<String>,
}

pub async fn oauth_callback(
    State(state): State<Arc<WebAppState>>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Response {
    // 检查是否有错误
    if let Some(error) = query.error {
        let mut result = state.oauth_result.write().await;
        *result = super::server::OAuthResult::Error(error.clone());
        return axum::response::Html(format!(
            r#"<!DOCTYPE html>
<html><body style="font-family: sans-serif; text-align: center; padding: 50px;">
<h1 style="color: red;">❌ 授权失败</h1>
<p>{}</p>
<script>setTimeout(function() {{ window.close(); }}, 3000);</script>
</body></html>"#,
            error
        ))
        .into_response();
    }

    // 获取 code
    let code = match query.code {
        Some(c) => c,
        None => {
            let mut result = state.oauth_result.write().await;
            *result = super::server::OAuthResult::Error("未收到授权码".to_string());
            return axum::response::Html(
                r#"<!DOCTYPE html>
<html><body style="font-family: sans-serif; text-align: center; padding: 50px;">
<h1 style="color: red;">❌ 授权失败</h1>
<p>未收到授权码</p>
</body></html>"#,
            )
            .into_response();
        }
    };

    // 获取 redirect_uri
    let redirect_uri = {
        let pending = state.oauth_pending.read().await;
        match pending.as_ref() {
            Some(p) => p.redirect_uri.clone(),
            None => {
                let base_url = std::env::var("OAUTH_CALLBACK_BASE")
                    .or_else(|_| std::env::var("BASE_URL"))
                    .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
                format!("{}/api/oauth/callback", base_url.trim_end_matches('/'))
            }
        }
    };

    // 交换 token
    let token_response =
        match crate::core::services::oauth::exchange_code(&code, &redirect_uri).await {
            Ok(t) => t,
            Err(e) => {
                let mut result = state.oauth_result.write().await;
                *result = super::server::OAuthResult::Error(e.clone());
                return axum::response::Html(format!(
                    r#"<!DOCTYPE html>
<html><body style="font-family: sans-serif; text-align: center; padding: 50px;">
<h1 style="color: red;">❌ Token 交换失败</h1>
<p>{}</p>
</body></html>"#,
                    e
                ))
                .into_response();
            }
        };

    // 检查 refresh_token
    let refresh_token = match token_response.refresh_token {
        Some(rt) => rt,
        None => {
            let mut result = state.oauth_result.write().await;
            *result = super::server::OAuthResult::Error("未获取到 Refresh Token".to_string());
            return axum::response::Html(
                r#"<!DOCTYPE html>
<html><body style="font-family: sans-serif; text-align: center; padding: 50px;">
<h1 style="color: orange;">⚠️ 未获取到 Refresh Token</h1>
<p>可能您之前已授权过此应用。请访问 
<a href="https://myaccount.google.com/permissions" target="_blank">Google 账号权限</a>
撤销 'Antigravity Tools' 后重试。</p>
</body></html>"#,
            )
            .into_response();
        }
    };

    // 获取用户信息
    let user_info =
        match crate::core::services::oauth::get_user_info(&token_response.access_token).await {
            Ok(u) => u,
            Err(e) => {
                let mut result = state.oauth_result.write().await;
                *result = super::server::OAuthResult::Error(e.clone());
                return axum::response::Html(format!(
                    r#"<!DOCTYPE html>
<html><body style="font-family: sans-serif; text-align: center; padding: 50px;">
<h1 style="color: red;">❌ 获取用户信息失败</h1>
<p>{}</p>
</body></html>"#,
                    e
                ))
                .into_response();
            }
        };

    // 构造 TokenData 并保存账号
    let token_data = TokenData {
        access_token: token_response.access_token,
        refresh_token,
        expires_in: token_response.expires_in,
        expiry_timestamp: chrono::Utc::now().timestamp() + token_response.expires_in,
        token_type: "Bearer".to_string(),
        email: Some(user_info.email.clone()),
        project_id: None,
        session_id: None,
    };

    let account = match crate::core::services::AccountService::add_account(
        &state.storage,
        &state.emitter,
        user_info.email.clone(),
        user_info.get_display_name(),
        token_data,
    ) {
        Ok(acc) => acc,
        Err(e) => {
            let mut result = state.oauth_result.write().await;
            *result = super::server::OAuthResult::Error(e.clone());
            return axum::response::Html(format!(
                r#"<!DOCTYPE html>
<html><body style="font-family: sans-serif; text-align: center; padding: 50px;">
<h1 style="color: red;">❌ 保存账号失败</h1>
<p>{}</p>
</body></html>"#,
                e
            ))
            .into_response();
        }
    };

    // 更新结果状态
    {
        let mut result = state.oauth_result.write().await;
        *result = super::server::OAuthResult::Success(account.clone());
    }

    // 清除待处理状态
    {
        let mut pending = state.oauth_pending.write().await;
        *pending = None;
    }

    // 返回成功页面
    axum::response::Html(format!(
        r#"<!DOCTYPE html>
<html><body style="font-family: sans-serif; text-align: center; padding: 50px;">
<h1 style="color: green;">✅ 授权成功!</h1>
<p>账号 <strong>{}</strong> 已添加</p>
<p>您可以关闭此窗口返回应用。</p>
<script>setTimeout(function() {{ window.close(); }}, 2000);</script>
</body></html>"#,
        account.email
    ))
    .into_response()
}

/// 获取 OAuth 授权状态
#[derive(Serialize)]
pub struct OAuthStatusResponse {
    pub status: String, // "pending", "success", "error"
    pub account: Option<Account>,
    pub error: Option<String>,
}

pub async fn get_oauth_status(State(state): State<Arc<WebAppState>>) -> Response {
    let result = state.oauth_result.read().await;

    match &*result {
        super::server::OAuthResult::Pending => ApiResponse::ok(OAuthStatusResponse {
            status: "pending".to_string(),
            account: None,
            error: None,
        })
        .into_response(),
        super::server::OAuthResult::Success(account) => ApiResponse::ok(OAuthStatusResponse {
            status: "success".to_string(),
            account: Some(account.clone()),
            error: None,
        })
        .into_response(),
        super::server::OAuthResult::Error(e) => ApiResponse::ok(OAuthStatusResponse {
            status: "error".to_string(),
            account: None,
            error: Some(e.clone()),
        })
        .into_response(),
    }
}

// ==================== 文件导入 API ====================

/// 导入结果
#[derive(Serialize)]
pub struct ImportResult {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

/// 导入账号的数据结构
#[derive(Deserialize)]
struct ImportAccount {
    email: Option<String>,
    refresh_token: String,
    #[serde(default)]
    name: Option<String>,
}

/// JSON 导入格式 (支持数组或 {accounts: [...]} 结构)
#[derive(Deserialize)]
#[serde(untagged)]
enum ImportData {
    Array(Vec<ImportAccount>),
    Wrapped { accounts: Vec<ImportAccount> },
}

/// 批量导入账号 (JSON Body)
pub async fn import_accounts_json(
    State(state): State<Arc<WebAppState>>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    // 解析 JSON
    let accounts: Vec<ImportAccount> = match serde_json::from_value::<ImportData>(body) {
        Ok(ImportData::Array(arr)) => arr,
        Ok(ImportData::Wrapped { accounts }) => accounts,
        Err(e) => {
            return ApiResponse::err(format!("无效的 JSON 格式: {}", e)).into_response();
        }
    };

    if accounts.is_empty() {
        return ApiResponse::err("未找到账号数据").into_response();
    }

    let result = do_import_accounts(&state, accounts).await;
    ApiResponse::ok(result).into_response()
}

/// 文件导入请求 (JSON body 包含文件内容)
#[derive(Deserialize)]
pub struct ImportFileRequest {
    /// 文件内容
    pub content: String,
    /// 文件格式: "json" 或 "csv"
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "json".to_string()
}

/// 文件导入 (JSON body 包含文件内容和格式)
pub async fn import_accounts_file(
    State(state): State<Arc<WebAppState>>,
    Json(req): Json<ImportFileRequest>,
) -> Response {
    if req.content.is_empty() {
        return ApiResponse::err("文件内容为空").into_response();
    }

    // 根据格式解析
    let accounts: Vec<ImportAccount> = if req.format == "csv" {
        parse_csv(&req.content)
    } else {
        // JSON 格式
        match serde_json::from_str::<ImportData>(&req.content) {
            Ok(ImportData::Array(arr)) => arr,
            Ok(ImportData::Wrapped { accounts }) => accounts,
            Err(e) => {
                return ApiResponse::err(format!("无效的 JSON 格式: {}", e)).into_response();
            }
        }
    };

    if accounts.is_empty() {
        return ApiResponse::err("未找到有效账号数据").into_response();
    }

    let result = do_import_accounts(&state, accounts).await;
    ApiResponse::ok(result).into_response()
}

/// 解析 CSV 格式
fn parse_csv(content: &str) -> Vec<ImportAccount> {
    let mut accounts = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.to_lowercase().starts_with("email") {
            continue; // 跳过空行、注释和标题行
        }

        let parts: Vec<&str> = line.split(',').collect();

        if parts.len() >= 2 {
            // email,refresh_token 格式
            let email = parts[0].trim().to_string();
            let refresh_token = parts[1].trim().to_string();
            if refresh_token.starts_with("1//") {
                accounts.push(ImportAccount {
                    email: Some(email),
                    refresh_token,
                    name: None,
                });
            }
        } else if parts.len() == 1 {
            // 仅 refresh_token 格式
            let token = parts[0].trim().to_string();
            if token.starts_with("1//") {
                accounts.push(ImportAccount {
                    email: None,
                    refresh_token: token,
                    name: None,
                });
            }
        }
    }

    accounts
}

/// 执行批量导入
async fn do_import_accounts(state: &WebAppState, accounts: Vec<ImportAccount>) -> ImportResult {
    let mut result = ImportResult {
        total: accounts.len(),
        success: 0,
        failed: 0,
        errors: Vec::new(),
    };

    for acc in accounts {
        let email = acc.email.clone().unwrap_or_default();

        // 创建 TokenData
        match create_account_from_token(&acc.refresh_token).await {
            Ok((token_data, actual_email)) => {
                let final_email = if email.is_empty() {
                    actual_email
                } else {
                    email
                };

                // 使用 AccountService 添加账号
                match AccountService::add_account(
                    &state.storage,
                    &state.emitter,
                    final_email.clone(),
                    acc.name.clone(),
                    token_data,
                ) {
                    Ok(_) => result.success += 1,
                    Err(e) => {
                        result.failed += 1;
                        result.errors.push(format!("{}: {}", final_email, e));
                    }
                }
            }
            Err(e) => {
                result.failed += 1;
                let identifier = if email.is_empty() {
                    format!(
                        "token:{}...",
                        &acc.refresh_token[..12.min(acc.refresh_token.len())]
                    )
                } else {
                    email
                };
                result.errors.push(format!("{}: {}", identifier, e));
            }
        }
    }

    result
}

/// 从 refresh_token 创建账号
async fn create_account_from_token(refresh_token: &str) -> Result<(TokenData, String), String> {
    // 刷新 token 获取 access_token
    let token_response = crate::core::services::oauth::refresh_access_token(refresh_token).await?;

    // 获取用户信息
    let user_info =
        crate::core::services::oauth::get_user_info(&token_response.access_token).await?;

    let token_data = TokenData::new(
        token_response.access_token,
        refresh_token.to_string(),
        token_response.expires_in,
        Some(user_info.email.clone()),
        None, // project_id
        None, // session_id
    );

    Ok((token_data, user_info.email))
}

// ==================== 数据库导入 API ====================

/// 数据库导入请求
#[derive(Deserialize)]
pub struct ImportDatabaseRequest {
    /// 数据库连接 URL (postgres:// 或 sqlite:)
    pub url: String,
    /// 表名 (可选, 默认 "accounts")
    #[serde(default)]
    pub table: Option<String>,
    /// email 列名 (可选, 默认 "email")
    #[serde(default)]
    pub email_column: Option<String>,
    /// refresh_token 列名 (可选, 默认 "refresh_token")
    #[serde(default)]
    pub token_column: Option<String>,
}

/// 从数据库导入账号
#[cfg(feature = "web-server")]
pub async fn import_from_database(
    State(state): State<Arc<WebAppState>>,
    Json(req): Json<ImportDatabaseRequest>,
) -> Response {
    use crate::core::services::{DatabaseImporter, ImportConfig};

    // 构建导入配置
    let config = ImportConfig {
        table: req.table.unwrap_or_else(|| "accounts".to_string()),
        email_column: req.email_column.unwrap_or_else(|| "email".to_string()),
        token_column: req
            .token_column
            .unwrap_or_else(|| "refresh_token".to_string()),
    };

    // 从数据库导入
    let imported = match DatabaseImporter::import_from_url(&req.url, &config).await {
        Ok(accounts) => accounts,
        Err(e) => {
            return ApiResponse::err(e).into_response();
        }
    };

    if imported.is_empty() {
        return ApiResponse::err("数据库中未找到账号数据").into_response();
    }

    // 转换为 ImportAccount 格式
    let accounts: Vec<ImportAccount> = imported
        .into_iter()
        .map(|a| ImportAccount {
            email: a.email,
            refresh_token: a.refresh_token,
            name: None,
        })
        .collect();

    // 批量导入
    let result = do_import_accounts(&state, accounts).await;
    ApiResponse::ok(result).into_response()
}

// ==================== 日志 API ====================

/// 日志查询请求参数
#[derive(Deserialize)]
pub struct LogQueryParams {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    100
}

/// 日志查询响应
#[derive(Serialize)]
pub struct LogQueryResponse {
    pub logs: Vec<crate::proxy::ProxyLogEntry>,
    pub total: usize,
}

/// 获取代理日志
pub async fn get_proxy_logs(
    State(state): State<Arc<WebAppState>>,
    Query(params): Query<LogQueryParams>,
) -> Response {
    let logs = state.log_store.get_logs(params.limit, params.offset);
    let total = state.log_store.len();

    ApiResponse::ok(LogQueryResponse { logs, total }).into_response()
}

/// 清除代理日志
pub async fn clear_proxy_logs(State(state): State<Arc<WebAppState>>) -> Response {
    state.log_store.clear();
    ApiResponse::ok("日志已清除").into_response()
}
