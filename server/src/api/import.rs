use axum::{
    extract::{Json, State},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::common::{into_response, ApiResponse};
use crate::core::models::TokenData;
use crate::core::services::AccountService;
use crate::state::AppState;

#[derive(Serialize)]
pub struct ImportResult {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

#[derive(Deserialize)]
struct ImportAccount {
    email: Option<String>,
    refresh_token: String,
    name: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ImportData {
    Array(Vec<ImportAccount>),
    Wrapped { accounts: Vec<ImportAccount> },
}

pub async fn import_accounts_json(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let accounts: Vec<ImportAccount> = match serde_json::from_value::<ImportData>(body) {
        Ok(ImportData::Array(arr)) => arr,
        Ok(ImportData::Wrapped { accounts }) => accounts,
        Err(e) => return ApiResponse::err(format!("无效 JSON: {}", e)).into_response(),
    };

    let mut result = ImportResult {
        total: accounts.len(),
        success: 0,
        failed: 0,
        errors: Vec::new(),
    };

    for acc in accounts {
        match process_import(&state, acc).await {
            Ok(_) => result.success += 1,
            Err(e) => {
                result.failed += 1;
                result.errors.push(e);
            }
        }
    }

    ApiResponse::ok(result).into_response()
}

async fn process_import(state: &AppState, acc: ImportAccount) -> Result<(), String> {
    let (token_data, actual_email) = get_token_info(&acc.refresh_token).await?;
    let final_email = acc.email.unwrap_or(actual_email);

    AccountService::add_account(
        &state.db_pool,
        &state.emitter,
        final_email,
        acc.name,
        token_data,
    )
    .await
    .map(|_| ())
}

async fn get_token_info(refresh_token: &str) -> Result<(TokenData, String), String> {
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
