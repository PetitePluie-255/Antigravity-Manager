use crate::core::models::QuotaData;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error, info, warn};

const USER_AGENT: &str = "antigravity/1.11.3 Darwin/arm64";
const QUOTA_API_URL: &str = "https://cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels";
const CLOUD_CODE_BASE_URL: &str = "https://cloudcode-pa.googleapis.com";

const NEAR_READY_THRESHOLD: i32 = 80;
const MAX_RETRIES: i32 = 2;
const RETRY_DELAY_SECS: u64 = 15;

#[derive(Debug, Deserialize)]
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
    #[serde(rename = "currentTier")]
    current_tier: Option<Tier>,
    #[serde(rename = "paidTier")]
    paid_tier: Option<Tier>,
}

#[derive(Debug, Deserialize)]
struct Tier {
    id: Option<String>,
}

/// 获取项目 ID 和订阅类型
pub async fn fetch_project_id(access_token: &str, email: &str) -> (Option<String>, Option<String>) {
    let client = reqwest::Client::new();
    let meta = json!({"metadata": {"ideType": "ANTIGRAVITY"}});

    let res = client
        .post(format!("{}/v1internal:loadCodeAssist", CLOUD_CODE_BASE_URL))
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", access_token),
        )
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .json(&meta)
        .send()
        .await;

    match res {
        Ok(res) => {
            if res.status().is_success() {
                if let Ok(data) = res.json::<LoadProjectResponse>().await {
                    let project_id = data.project_id.clone();

                    // 核心逻辑：优先从 paid_tier 获取订阅 ID
                    let subscription_tier = data
                        .paid_tier
                        .and_then(|t| t.id)
                        .or_else(|| data.current_tier.and_then(|t| t.id));

                    if let Some(ref tier) = subscription_tier {
                        info!("[{}] 订阅识别成功: {}", email, tier);
                    }

                    return (project_id, subscription_tier);
                }
            } else {
                warn!("[{}] loadCodeAssist 失败: Status: {}", email, res.status());
            }
        }
        Err(e) => {
            error!("[{}] loadCodeAssist 网络错误: {}", email, e);
        }
    }

    (None, None)
}

/// 查询账号配额
pub async fn fetch_quota(
    access_token: &str,
    email: &str,
) -> AppResult<(QuotaData, Option<String>)> {
    // 1. 获取 Project ID 和订阅类型
    let (project_id, subscription_tier) = fetch_project_id(access_token, email).await;

    let final_project_id = project_id.as_deref().unwrap_or("bamboo-precept-lgxtn");

    let client = reqwest::Client::new();
    let payload = json!({
        "project": final_project_id
    });

    let max_retries = 3;
    let mut last_error: Option<AppError> = None;

    for attempt in 1..=max_retries {
        match client
            .post(QUOTA_API_URL)
            .bearer_auth(access_token)
            .header("User-Agent", USER_AGENT)
            .json(&payload)
            .send()
            .await
        {
            Ok(response) => {
                if !response.status().is_success() {
                    let status = response.status();

                    // 处理 403 Forbidden
                    if status == reqwest::StatusCode::FORBIDDEN {
                        warn!("[{}] 账号无权限 (403 Forbidden)", email);
                        let mut q = QuotaData::new();
                        q.is_forbidden = true;
                        q.subscription_tier = subscription_tier.clone();
                        return Ok((q, project_id.clone()));
                    }

                    if attempt < max_retries {
                        let text = response.text().await.unwrap_or_default();
                        warn!(
                            "API 错误: {} - {} (尝试 {}/{})",
                            status, text, attempt, max_retries
                        );
                        last_error = Some(AppError::Unknown(format!("HTTP {} - {}", status, text)));
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        continue;
                    } else {
                        let text = response.text().await.unwrap_or_default();
                        return Err(AppError::Unknown(format!(
                            "API 错误: {} - {}",
                            status, text
                        )));
                    }
                }

                let quota_response: QuotaResponse = response.json().await?;

                let mut quota_data = QuotaData::new();
                debug!("Quota API 返回了 {} 个模型", quota_response.models.len());

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

                quota_data.subscription_tier = subscription_tier.clone();
                return Ok((quota_data, project_id.clone()));
            }
            Err(e) => {
                warn!("请求失败: {} (尝试 {}/{})", e, attempt, max_retries);
                last_error = Some(AppError::Network(e));
                if attempt < max_retries {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| AppError::Unknown("配额查询失败".to_string())))
}

/// 维护性预热请求客户端
fn create_warmup_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_default()
}

/// 通过代理内部 API 发送预热请求
pub async fn warmup_model_directly(
    access_token: &str,
    model_name: &str,
    project_id: &str,
    email: &str,
    percentage: i32,
    port: u16,
) -> bool {
    let warmup_url = format!("http://127.0.0.1:{}/internal/warmup", port);
    let body = json!({
        "email": email,
        "model": model_name,
        "access_token": access_token,
        "project_id": project_id
    });

    let client = create_warmup_client();
    let resp = client
        .post(&warmup_url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await;

    match resp {
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                info!(
                    "[Warmup] ✓ Triggered {} for {} (was {}%)",
                    model_name, email, percentage
                );
                true
            } else {
                let text = response.text().await.unwrap_or_default();
                warn!(
                    "[Warmup] ✗ {} for {} (was {}%): HTTP {} - {}",
                    model_name, email, percentage, status, text
                );
                false
            }
        }
        Err(e) => {
            warn!(
                "[Warmup] ✗ {} for {} (was {}%): {}",
                model_name, email, percentage, e
            );
            false
        }
    }
}

/// 准备用于预热的有效 Token
pub async fn get_valid_token_for_warmup(
    pool: &sqlx::SqlitePool,
    account: &crate::core::models::Account,
) -> AppResult<(String, String)> {
    let mut account = account.clone();

    // 检查并自动刷新 token
    let new_token = crate::core::services::oauth::ensure_fresh_token(&account.token).await?;

    // 如果 token 改变了（意味着刷新了），保存它
    if new_token.access_token != account.token.access_token {
        account.token = new_token;
        if let Err(e) = crate::core::services::account::AccountService::upsert_account(
            pool,
            &crate::core::traits::NoopEmitter,
            account.email.clone(),
            account.name.clone(),
            account.token.clone(),
        )
        .await
        {
            warn!("[Warmup] 保存刷新后的 Token 失败: {}", e);
        } else {
            info!("[Warmup] 成功为 {} 刷新并保存了新 Token", account.email);
        }
    }

    // 获取 project_id
    let (project_id, _) = fetch_project_id(&account.token.access_token, &account.email).await;
    let final_pid = project_id.unwrap_or_else(|| "bamboo-precept-lgxtn".to_string());

    Ok((account.token.access_token, final_pid))
}

/// 智能预热所有账号 (此版本由 scheduler 调用，或由前端手动触发)
pub async fn warm_up_all_accounts(pool: &sqlx::SqlitePool, port: u16) -> AppResult<String> {
    let mut retry_count = 0;

    loop {
        let target_accounts = crate::core::services::account::AccountService::list_accounts(pool)
            .await
            .map_err(|e| AppError::Account(e))?;

        if target_accounts.is_empty() {
            return Ok("没有可用账号".to_string());
        }

        info!(
            "[Warmup] 开始筛选 {} 个账号的模型...",
            target_accounts.len()
        );

        let mut warmup_items = Vec::new();
        let mut has_near_ready_models = false;

        for account in &target_accounts {
            // 跳过已禁用的账号
            if account.disabled {
                continue;
            }

            let (token, pid) = match get_valid_token_for_warmup(pool, account).await {
                Ok(t) => t,
                Err(e) => {
                    warn!("[Warmup] 账号 {} 准备失败: {}", account.email, e);
                    continue;
                }
            };

            // 获取最新实时配额
            if let Ok((fresh_quota, _)) = fetch_quota(&token, &account.email).await {
                let mut account_warmed_series = std::collections::HashSet::new();
                for m in fresh_quota.models {
                    if m.percentage >= 100 {
                        // 1. 映射逻辑
                        let model_to_ping = if m.name == "gemini-2.5-flash" {
                            "gemini-3-flash".to_string()
                        } else {
                            m.name.clone()
                        };

                        // 2. 严格白名单过滤
                        match model_to_ping.as_str() {
                            "gemini-3-flash" | "claude-sonnet-4-5" | "gemini-3-pro-high"
                            | "gemini-3-pro-image" => {
                                if !account_warmed_series.contains(&model_to_ping) {
                                    warmup_items.push((
                                        account.email.clone(),
                                        model_to_ping.clone(),
                                        token.clone(),
                                        pid.clone(),
                                        m.percentage,
                                    ));
                                    account_warmed_series.insert(model_to_ping);
                                }
                            }
                            _ => continue,
                        }
                    } else if m.percentage >= NEAR_READY_THRESHOLD {
                        has_near_ready_models = true;
                    }
                }
            }
        }

        if !warmup_items.is_empty() {
            let total = warmup_items.len();
            tokio::spawn(async move {
                let mut success = 0;
                let round_total = warmup_items.len();
                for (idx, (email, model, token, pid, pct)) in warmup_items.into_iter().enumerate() {
                    if warmup_model_directly(&token, &model, &pid, &email, pct, port).await {
                        success += 1;
                    }
                    if idx < round_total - 1 {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    }
                }
                info!("[Warmup] 预热任务完成: 成功 {}/{}", success, total);
            });
            return Ok(format!("已启动 {} 个模型的预热任务", total));
        }

        if has_near_ready_models && retry_count < MAX_RETRIES {
            retry_count += 1;
            info!(
                "[Warmup] 检测到临界恢复模型，等待 {}s 后重试 ({}/{})",
                RETRY_DELAY_SECS, retry_count, MAX_RETRIES
            );
            tokio::time::sleep(tokio::time::Duration::from_secs(RETRY_DELAY_SECS)).await;
            continue;
        }

        return Ok("没有模型需要预热".to_string());
    }
}
