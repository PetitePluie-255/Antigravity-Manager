use crate::core::models::QuotaData;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error, info, warn};

const USER_AGENT: &str = "antigravity/windows/amd64";
const QUOTA_API_URL: &str = "https://cloudcode-pa.googleapis.com/v1internal:loadModelQuotas";
const CLOUD_CODE_BASE_URL: &str = "https://cloudcode-pa.googleapis.com";

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
