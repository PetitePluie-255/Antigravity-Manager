use axum::{
    extract::{Json, Path, State},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::sync::Arc;

use super::common::{into_response, ApiResponse};
use crate::core::services::{AccountService, QuotaService};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct AddAccountRequest {
    pub email: String,
    pub name: Option<String>,
    pub refresh_token: String,
}

#[derive(Deserialize)]
pub struct SwitchAccountRequest {
    pub account_id: String,
}

#[derive(Deserialize)]
pub struct BatchDeleteRequest {
    pub account_ids: Vec<String>,
}

pub async fn list_accounts(State(state): State<Arc<AppState>>) -> Response {
    into_response(AccountService::list_accounts(&state.db_pool).await)
}

pub async fn add_account(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddAccountRequest>,
) -> Response {
    match create_account_from_token(&req.refresh_token).await {
        Ok((token_data, actual_email)) => {
            let final_email = if req.email.is_empty() {
                actual_email
            } else {
                req.email
            };
            into_response(
                AccountService::add_account(
                    &state.db_pool,
                    &state.emitter,
                    final_email,
                    req.name,
                    token_data,
                )
                .await,
            )
        }
        Err(e) => ApiResponse::err(format!("Token 刷新失败: {}", e)).into_response(),
    }
}

pub async fn delete_account(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    into_response(AccountService::delete_account(&state.db_pool, &state.emitter, &id).await)
}

pub async fn delete_accounts(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BatchDeleteRequest>,
) -> Response {
    into_response(
        AccountService::delete_accounts(&state.db_pool, &state.emitter, &req.account_ids).await,
    )
}

pub async fn get_current_account(State(state): State<Arc<AppState>>) -> Response {
    into_response(AccountService::get_current_account(&state.db_pool).await)
}

pub async fn switch_account(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SwitchAccountRequest>,
) -> Response {
    into_response(
        AccountService::switch_account(&state.db_pool, &state.emitter, &req.account_id).await,
    )
}

pub async fn get_account_quota(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    match AccountService::load_account(&state.db_pool, &id).await {
        Ok(account) => ApiResponse::ok(account.quota).into_response(),
        Err(e) => ApiResponse::err(e).into_response(),
    }
}

pub async fn refresh_account_quota(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    into_response(QuotaService::refresh_account_quota(&state.db_pool, &id).await)
}

pub async fn refresh_all_quotas(State(state): State<Arc<AppState>>) -> Response {
    match QuotaService::refresh_all_quotas(&state.db_pool).await {
        Ok((success, errors)) => ApiResponse::ok(serde_json::json!({
            "success_count": success,
            "error_count": errors
        }))
        .into_response(),
        Err(e) => ApiResponse::err(e).into_response(),
    }
}

async fn create_account_from_token(
    refresh_token: &str,
) -> Result<(crate::core::models::TokenData, String), String> {
    use crate::core::models::TokenData;
    let token_response = crate::core::services::oauth::refresh_access_token(refresh_token).await?;
    let user_info =
        crate::core::services::oauth::get_user_info(&token_response.access_token).await?;

    let token_data = TokenData::new(
        token_response.access_token,
        refresh_token.to_string(),
        token_response.expires_in,
        Some(user_info.email.clone()),
        None,
        None,
    );
    Ok((token_data, user_info.email))
}

// ========== Device Fingerprint APIs ==========

/// Get device profiles for an account (current, history, and baseline)
pub async fn get_device_profiles(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    match AccountService::load_account(&state.db_pool, &id).await {
        Ok(account) => {
            let response = serde_json::json!({
                "current_storage": account.device_profile,
                "history": account.device_history,
                "baseline": crate::core::device::load_global_original()
            });
            ApiResponse::ok(response).into_response()
        }
        Err(e) => ApiResponse::err(e).into_response(),
    }
}

/// Preview a newly generated device profile (without saving)
pub async fn preview_generate_profile() -> Response {
    let profile = crate::core::device::generate_profile();
    ApiResponse::ok(profile).into_response()
}

#[derive(Deserialize)]
pub struct BindDeviceProfileRequest {
    pub profile: crate::core::models::DeviceProfile,
}

/// Bind a specific device profile to an account
pub async fn bind_device_profile(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<BindDeviceProfileRequest>,
) -> Response {
    match AccountService::load_account(&state.db_pool, &id).await {
        Ok(mut account) => {
            use crate::core::models::DeviceProfileVersion;
            use chrono::Utc;
            use uuid::Uuid;

            // Create a new version entry
            let version = DeviceProfileVersion {
                id: Uuid::new_v4().to_string(),
                label: format!("绑定于 {}", Utc::now().format("%Y-%m-%d %H:%M")),
                profile: req.profile.clone(),
                created_at: Utc::now().timestamp(),
                is_current: true,
            };

            // Mark all previous versions as not current
            let mut history = account.device_history.clone();
            for v in history.iter_mut() {
                v.is_current = false;
            }
            history.push(version);

            // Update account device info
            match AccountService::update_account_device(
                &state.db_pool,
                &id,
                Some(req.profile.clone()),
                history,
            )
            .await
            {
                Ok(_) => ApiResponse::ok(req.profile).into_response(),
                Err(e) => ApiResponse::err(e).into_response(),
            }
        }
        Err(e) => ApiResponse::err(e).into_response(),
    }
}

/// Restore a specific device profile version
pub async fn restore_device_version(
    State(state): State<Arc<AppState>>,
    Path((id, version_id)): Path<(String, String)>,
) -> Response {
    match AccountService::load_account(&state.db_pool, &id).await {
        Ok(mut account) => {
            let mut found_profile: Option<crate::core::models::DeviceProfile> = None;

            // Find the version and mark it as current
            for v in account.device_history.iter_mut() {
                if v.id == version_id {
                    v.is_current = true;
                    found_profile = Some(v.profile.clone());
                } else {
                    v.is_current = false;
                }
            }

            match found_profile {
                Some(profile) => {
                    match AccountService::update_account_device(
                        &state.db_pool,
                        &id,
                        Some(profile.clone()),
                        account.device_history,
                    )
                    .await
                    {
                        Ok(_) => ApiResponse::ok(profile).into_response(),
                        Err(e) => ApiResponse::err(e).into_response(),
                    }
                }
                None => ApiResponse::err(format!("未找到版本 {}", version_id)).into_response(),
            }
        }
        Err(e) => ApiResponse::err(e).into_response(),
    }
}

/// Delete a device profile version
pub async fn delete_device_version(
    State(state): State<Arc<AppState>>,
    Path((id, version_id)): Path<(String, String)>,
) -> Response {
    match AccountService::load_account(&state.db_pool, &id).await {
        Ok(mut account) => {
            // Remove the version from history
            let original_len = account.device_history.len();
            account.device_history.retain(|v| v.id != version_id);

            if account.device_history.len() == original_len {
                return ApiResponse::err(format!("未找到版本 {}", version_id)).into_response();
            }

            match AccountService::update_account_device(
                &state.db_pool,
                &id,
                account.device_profile,
                account.device_history,
            )
            .await
            {
                Ok(_) => ApiResponse::ok(serde_json::json!({"success": true})).into_response(),
                Err(e) => ApiResponse::err(e).into_response(),
            }
        }
        Err(e) => ApiResponse::err(e).into_response(),
    }
}

/// Restore the global original (baseline) device profile
pub async fn restore_original_device(State(state): State<Arc<AppState>>) -> Response {
    match crate::core::device::load_global_original() {
        Some(baseline) => {
            // Get current account and bind baseline to it
            match AccountService::get_current_account(&state.db_pool).await {
                Ok(Some(mut account)) => {
                    use crate::core::models::DeviceProfileVersion;
                    use chrono::Utc;
                    use uuid::Uuid;

                    let version = DeviceProfileVersion {
                        id: Uuid::new_v4().to_string(),
                        label: "恢复原始指纹".to_string(),
                        profile: baseline.clone(),
                        created_at: Utc::now().timestamp(),
                        is_current: true,
                    };

                    for v in account.device_history.iter_mut() {
                        v.is_current = false;
                    }
                    account.device_history.push(version);

                    match AccountService::update_account_device(
                        &state.db_pool,
                        &account.id,
                        Some(baseline.clone()),
                        account.device_history,
                    )
                    .await
                    {
                        Ok(_) => ApiResponse::ok("已恢复原始指纹").into_response(),
                        Err(e) => ApiResponse::err(e).into_response(),
                    }
                }
                Ok(None) => ApiResponse::err("未选择当前账号").into_response(),
                Err(e) => ApiResponse::err(e).into_response(),
            }
        }
        None => ApiResponse::err("未找到原始指纹").into_response(),
    }
}
