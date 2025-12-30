//! 配额服务
//! 账户配额查询

use crate::core::models::{Account, QuotaData};
use crate::core::services::AccountService;
use crate::core::traits::StorageConfig;
use serde::{Deserialize, Serialize};
use serde_json::json;

const QUOTA_API_URL: &str = "https://cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels";
const LOAD_PROJECT_API_URL: &str = "https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist";
const USER_AGENT: &str = "antigravity/3.2.0 Darwin/arm64";

#[derive(Debug, Serialize, Deserialize)]
struct QuotaResponse {
    models: std::collections::HashMap<String, ModelInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ModelInfo {
    #[serde(rename = "quotaInfo")]
    quota_info: Option<QuotaInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct QuotaInfo {
    #[serde(rename = "remainingFraction")]
    remaining_fraction: Option<f64>,
    #[serde(rename = "resetTime")]
    reset_time: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LoadProjectResponse {
    #[serde(rename = "cloudaicompanionProject")]
    project_id: Option<String>,
}

/// 配额服务
pub struct QuotaService;

impl QuotaService {
    /// 创建 HTTP 客户端
    fn create_client() -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default()
    }

    /// 获取 Project ID
    async fn fetch_project_id(access_token: &str) -> Option<String> {
        let client = Self::create_client();
        let body = json!({
            "metadata": {
                "ideType": "ANTIGRAVITY"
            }
        });

        for _ in 0..2 {
            match client
                .post(LOAD_PROJECT_API_URL)
                .bearer_auth(access_token)
                .header("User-Agent", USER_AGENT)
                .json(&body)
                .send()
                .await
            {
                Ok(res) => {
                    if res.status().is_success() {
                        if let Ok(data) = res.json::<LoadProjectResponse>().await {
                            return data.project_id;
                        }
                    }
                }
                Err(_) => {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
        None
    }

    /// 查询账户配额 (调用外部 API)
    pub async fn fetch_quota(account: &Account) -> Result<QuotaData, String> {
        let client = Self::create_client();

        // 获取有效的 access_token（如果过期则刷新）
        let access_token = if account.token.is_expired() {
            tracing::info!("账户 {} 的 Token 已过期，正在刷新...", account.email);
            let token_response =
                crate::core::services::oauth::refresh_access_token(&account.token.refresh_token)
                    .await?;
            token_response.access_token
        } else {
            account.token.access_token.clone()
        };

        // 1. 获取 Project ID
        let project_id = Self::fetch_project_id(&access_token).await;
        tracing::debug!("Project ID: {:?}", project_id);

        // 2. 构建请求体
        let mut payload = serde_json::Map::new();
        if let Some(pid) = project_id {
            payload.insert("project".to_string(), json!(pid));
        }

        // 3. 发送配额请求
        let max_retries = 3;
        let mut last_error: Option<String> = None;

        for attempt in 1..=max_retries {
            match client
                .post(QUOTA_API_URL)
                .bearer_auth(&access_token)
                .header("User-Agent", USER_AGENT)
                .json(&json!(payload))
                .send()
                .await
            {
                Ok(response) => {
                    let status = response.status();

                    if !status.is_success() {
                        // 403 Forbidden - 账户被禁止
                        if status.as_u16() == 403 {
                            tracing::warn!("账号无权限 (403 Forbidden)");
                            let mut q = QuotaData::new();
                            q.is_forbidden = true;
                            return Ok(q);
                        }

                        if attempt < max_retries {
                            let text = response.text().await.unwrap_or_default();
                            tracing::warn!(
                                "API 错误: {} (尝试 {}/{})",
                                status,
                                attempt,
                                max_retries
                            );
                            last_error = Some(format!("HTTP {} - {}", status, text));
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                            continue;
                        } else {
                            let text = response.text().await.unwrap_or_default();
                            return Err(format!("API 错误: {} - {}", status, text));
                        }
                    }

                    // 解析响应
                    let quota_response: QuotaResponse = response
                        .json()
                        .await
                        .map_err(|e| format!("解析配额响应失败: {}", e))?;

                    let mut quota_data = QuotaData::new();

                    tracing::info!("配额 API 返回了 {} 个模型", quota_response.models.len());

                    for (name, info) in quota_response.models {
                        if let Some(quota_info) = info.quota_info {
                            let percentage = quota_info
                                .remaining_fraction
                                .map(|f| (f * 100.0) as i32)
                                .unwrap_or(0);

                            let reset_time = quota_info.reset_time.unwrap_or_default();

                            // 只保存我们关心的模型
                            if name.contains("gemini") || name.contains("claude") {
                                quota_data.add_model(name, percentage, reset_time);
                            }
                        }
                    }

                    return Ok(quota_data);
                }
                Err(e) => {
                    tracing::warn!("请求失败: {} (尝试 {}/{})", e, attempt, max_retries);
                    last_error = Some(e.to_string());
                    if attempt < max_retries {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| "配额查询失败".to_string()))
    }

    /// 通过 access_token 和 project_id 查询配额
    /// 用于后台刷新任务，不需要完整的 Account 对象
    pub async fn fetch_quota_by_token(
        access_token: &str,
        project_id: Option<&str>,
    ) -> Result<QuotaData, String> {
        let client = Self::create_client();

        // 如果没有 project_id，尝试获取
        let pid = if let Some(p) = project_id {
            Some(p.to_string())
        } else {
            Self::fetch_project_id(access_token).await
        };

        // 构建请求体
        let mut payload = serde_json::Map::new();
        if let Some(p) = pid {
            payload.insert("project".to_string(), json!(p));
        }

        // 发送请求
        let response = client
            .post(QUOTA_API_URL)
            .bearer_auth(access_token)
            .header("User-Agent", USER_AGENT)
            .json(&json!(payload))
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        let status = response.status();

        if !status.is_success() {
            if status.as_u16() == 403 {
                let mut q = QuotaData::new();
                q.is_forbidden = true;
                return Ok(q);
            }
            let text = response.text().await.unwrap_or_default();
            return Err(format!("API 错误: {} - {}", status, text));
        }

        // 解析响应
        let quota_response: QuotaResponse = response
            .json()
            .await
            .map_err(|e| format!("解析配额响应失败: {}", e))?;

        let mut quota_data = QuotaData::new();

        for (name, info) in quota_response.models {
            if let Some(quota_info) = info.quota_info {
                let percentage = quota_info
                    .remaining_fraction
                    .map(|f| (f * 100.0) as i32)
                    .unwrap_or(0);

                let reset_time = quota_info.reset_time.unwrap_or_default();

                if name.contains("gemini") || name.contains("claude") {
                    quota_data.add_model(name, percentage, reset_time);
                }
            }
        }

        Ok(quota_data)
    }

    /// 刷新账户配额并保存
    pub async fn refresh_account_quota<S: StorageConfig>(
        storage: &S,
        account_id: &str,
    ) -> Result<QuotaData, String> {
        // 加载账户
        let account = AccountService::load_account(storage, account_id)?;

        // 查询配额
        let quota = Self::fetch_quota(&account).await?;

        // 保存配额
        AccountService::update_account_quota(storage, account_id, quota.clone())?;

        Ok(quota)
    }

    /// 刷新所有账户配额
    pub async fn refresh_all_quotas<S: StorageConfig>(
        storage: &S,
    ) -> Result<(usize, usize), String> {
        let accounts = AccountService::list_accounts(storage)?;

        let mut success_count = 0;
        let mut error_count = 0;

        for account in accounts {
            match Self::refresh_account_quota(storage, &account.id).await {
                Ok(_) => success_count += 1,
                Err(e) => {
                    tracing::warn!("刷新账户 {} 配额失败: {}", account.email, e);
                    error_count += 1;
                }
            }
        }

        Ok((success_count, error_count))
    }
}
