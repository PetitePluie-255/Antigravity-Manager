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
