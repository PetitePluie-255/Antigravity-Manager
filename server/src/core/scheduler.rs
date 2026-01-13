use crate::core::quota;
use crate::state::AppState;
use chrono::Utc;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::time::{self, Duration};
use tracing::{debug, info};

// 预热历史记录：key = "email:model_name:100", value = 预热时间戳
static WARM_HISTORY: Lazy<Mutex<HashMap<String, i64>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub fn start_scheduler(state: Arc<AppState>) {
    tokio::spawn(async move {
        info!("Smart Warmup Scheduler started. Monitoring quota at 100%...");

        // 每 10 分钟扫描一次
        let mut interval = time::interval(Duration::from_secs(600));

        loop {
            interval.tick().await;

            // 获取配置
            let warmup_enabled = state
                .warmup_enabled
                .load(std::sync::atomic::Ordering::Relaxed);

            if !warmup_enabled {
                continue;
            }

            // 获取所有账号
            let accounts =
                match crate::core::services::account::AccountService::list_accounts(&state.db_pool)
                    .await
                {
                    Ok(a) => a,
                    Err(e) => {
                        debug!("Failed to list accounts in scheduler: {}", e);
                        continue;
                    }
                };

            if accounts.is_empty() {
                continue;
            }

            info!(
                "[Scheduler] Scanning {} accounts for 100% quota models...",
                accounts.len()
            );

            let mut warmup_tasks = Vec::new();

            // 扫描每个账号的每个模型
            for account in &accounts {
                // 获取有效 token (这里直接使用 account 中的 token)
                let access_token = account.token.access_token.clone();
                let project_id = account
                    .token
                    .project_id
                    .clone()
                    .unwrap_or_else(|| "bamboo-precept-lgxtn".to_string());

                // 获取实时配额
                let (fresh_quota, _) = match quota::fetch_quota(&access_token, &account.email).await
                {
                    Ok(q) => q,
                    Err(_) => continue,
                };

                let now_ts = Utc::now().timestamp();

                for model in fresh_quota.models {
                    let history_key = format!("{}:{}:100", account.email, model.name);

                    // 核心逻辑：检测 100% 额度
                    if model.percentage == 100 {
                        // 检查是否已经在本周期预热过
                        {
                            let history = WARM_HISTORY.lock().unwrap();
                            if history.contains_key(&history_key) {
                                // 已经预热过这个 100% 周期，跳过
                                continue;
                            }
                        }

                        // 记录到历史
                        {
                            let mut history = WARM_HISTORY.lock().unwrap();
                            history.insert(history_key.clone(), now_ts);
                        }

                        // 模型名称映射
                        let model_to_ping = if model.name == "gemini-2.5-flash" {
                            "gemini-3-flash".to_string()
                        } else {
                            model.name.clone()
                        };

                        // 严格白名单过滤
                        match model_to_ping.as_str() {
                            "gemini-3-flash" | "claude-sonnet-4-5" | "gemini-3-pro-high"
                            | "gemini-3-pro-image" => {
                                warmup_tasks.push((
                                    account.email.clone(),
                                    model_to_ping.clone(),
                                    access_token.clone(),
                                    project_id.clone(),
                                    model.percentage,
                                ));

                                info!(
                                    "[Scheduler] ✓ Scheduled warmup: {} @ {} (quota at 100%)",
                                    model_to_ping, account.email
                                );
                            }
                            _ => continue,
                        }
                    } else if model.percentage < 100 {
                        // 额度未满，清除历史记录，允许下次 100% 时再预热
                        let mut history = WARM_HISTORY.lock().unwrap();
                        if history.remove(&history_key).is_some() {
                            info!(
                                "[Scheduler] Cleared history for {} @ {} (quota: {}%)",
                                model.name, account.email, model.percentage
                            );
                        }
                    }
                }
            }

            // 执行预热任务
            if !warmup_tasks.is_empty() {
                let total = warmup_tasks.len();
                info!("[Scheduler] 🔥 Triggering {} warmup tasks...", total);

                let port = state.proxy_port.load(std::sync::atomic::Ordering::Relaxed);

                for (idx, (email, model, token, pid, pct)) in warmup_tasks.into_iter().enumerate() {
                    info!(
                        "[Warmup {}/{}] {} @ {} ({}%)",
                        idx + 1,
                        total,
                        model,
                        email,
                        pct
                    );

                    quota::warmup_model_directly(&token, &model, &pid, &email, pct, port).await;

                    // 间隔 2 秒，避免请求过快
                    if idx < total - 1 {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    }
                }

                info!("[Scheduler] ✅ Warmup completed");
            }

            // 定期清理历史记录（保留最近 24 小时）
            {
                let now_ts = Utc::now().timestamp();
                let mut history = WARM_HISTORY.lock().unwrap();
                let cutoff = now_ts - 86400; // 24 小时前
                history.retain(|_, &mut ts| ts > cutoff);
            }
        }
    });
}
