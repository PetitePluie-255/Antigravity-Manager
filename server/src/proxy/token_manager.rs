// 移除冗余的顶层导入，因为这些在代码中已由 full path 或局部导入处理
use dashmap::DashMap;
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::proxy::rate_limit::RateLimitTracker;
use crate::proxy::sticky_config::StickySessionConfig;

#[derive(Debug, Clone)]
pub struct ProxyToken {
    pub account_id: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub timestamp: i64,
    pub email: String,
    pub account_path: Option<PathBuf>, // 可选：旧的 JSON 路径
    pub project_id: Option<String>,
    pub subscription_tier: Option<String>, // "FREE" | "PRO" | "ULTRA"
    pub disabled: bool,
    pub disabled_reason: Option<String>,
}

pub struct TokenManager {
    tokens: Arc<DashMap<String, ProxyToken>>, // account_id -> ProxyToken
    current_index: Arc<AtomicUsize>,
    last_used_account: Arc<tokio::sync::Mutex<Option<(String, std::time::Instant)>>>,
    data_dir: PathBuf,
    pool: SqlitePool,                                             // 数据库连接池
    rate_limit_tracker: Arc<RateLimitTracker>,                    // 限流跟踪器
    sticky_config: Arc<tokio::sync::RwLock<StickySessionConfig>>, // 调度配置
    session_accounts: Arc<DashMap<String, String>>, // 会话与账号映射 (SessionID -> AccountID)
}

impl TokenManager {
    /// 创建新的 TokenManager
    pub fn new(data_dir: PathBuf, pool: SqlitePool) -> Self {
        Self {
            tokens: Arc::new(DashMap::new()),
            current_index: Arc::new(AtomicUsize::new(0)),
            last_used_account: Arc::new(tokio::sync::Mutex::new(None)),
            data_dir,
            pool,
            rate_limit_tracker: Arc::new(RateLimitTracker::new()),
            sticky_config: Arc::new(tokio::sync::RwLock::new(StickySessionConfig::default())),
            session_accounts: Arc::new(DashMap::new()),
        }
    }

    /// 从数据库加载所有账号，并同步 JSON 目录中的新账号
    pub async fn load_accounts(&self) -> Result<usize, String> {
        // 1. 同步 JSON 文件到数据库 (迁移逻辑)
        self.sync_json_to_db().await?;

        // 2. 从数据库加载所有未禁用的账号
        let rows = sqlx::query(
            "SELECT id, email, access_token, refresh_token, expires_in, expiry_timestamp, project_id, disabled, disabled_reason, subscription_tier 
             FROM accounts WHERE disabled = 0"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch accounts from DB: {}", e))?;

        self.tokens.clear();
        self.current_index.store(0, Ordering::SeqCst);
        {
            let mut last_used = self.last_used_account.lock().await;
            *last_used = None;
        }

        let mut count = 0;
        for row in rows {
            use sqlx::Row;
            let token = ProxyToken {
                account_id: row.get("id"),
                email: row.get("email"),
                access_token: row.get("access_token"),
                refresh_token: row.get("refresh_token"),
                expires_in: row.get("expires_in"),
                timestamp: row.get("expiry_timestamp"),
                project_id: row.get("project_id"),
                subscription_tier: row.get("subscription_tier"),
                disabled: row.get::<Option<i64>, _>("disabled").unwrap_or(0) != 0,
                disabled_reason: row.get("disabled_reason"),
                account_path: None, // 数据库加载的没有路径
            };
            self.tokens.insert(token.account_id.clone(), token);
            count += 1;
        }

        Ok(count)
    }

    /// 迁移逻辑：将 `accounts/` 目录下的 JSON 同步到数据库
    async fn sync_json_to_db(&self) -> Result<(), String> {
        let accounts_dir = self.data_dir.join("accounts");
        if !accounts_dir.exists() {
            return Ok(());
        }

        let entries =
            std::fs::read_dir(&accounts_dir).map_err(|e| format!("读取账号目录失败: {}", e))?;

        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(Some(token)) = self.load_single_account_file(&path).await {
                        // 插入或更新到数据库
                        sqlx::query(
                            "INSERT INTO accounts (id, email, access_token, refresh_token, expires_in, expiry_timestamp, project_id, subscription_tier) 
                             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                             ON CONFLICT(id) DO UPDATE SET 
                                email = excluded.email,
                                access_token = excluded.access_token,
                                refresh_token = excluded.refresh_token,
                                expires_in = excluded.expires_in,
                                expiry_timestamp = excluded.expiry_timestamp,
                                project_id = excluded.project_id,
                                subscription_tier = excluded.subscription_tier"
                        )
                        .bind(&token.account_id)
                        .bind(&token.email)
                        .bind(&token.access_token)
                        .bind(&token.refresh_token)
                        .bind(token.expires_in)
                        .bind(token.timestamp)
                        .bind(&token.project_id)
                        .bind(&token.subscription_tier)
                        .execute(&self.pool)
                        .await
                        .map_err(|e| format!("Failed to sync account to DB: {}", e))?;
                    }
                }
            }
        }
        Ok(())
    }

    /// 辅助方法：加载单个 JSON 文件
    async fn load_single_account_file(&self, path: &PathBuf) -> Result<Option<ProxyToken>, String> {
        let content = std::fs::read_to_string(path).map_err(|e| format!("读取文件失败: {}", e))?;
        let account: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| format!("解析 JSON 失败: {}", e))?;

        if account
            .get("disabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            return Ok(None);
        }

        let account_id = account["id"].as_str().ok_or("缺少 id 字段")?.to_string();
        let email = account["email"]
            .as_str()
            .ok_or("缺少 email 字段")?
            .to_string();
        let token_obj = account["token"].as_object().ok_or("缺少 token 字段")?;
        let access_token = token_obj["access_token"]
            .as_str()
            .ok_or("缺少 access_token")?
            .to_string();
        let refresh_token = token_obj["refresh_token"]
            .as_str()
            .ok_or("缺少 refresh_token")?
            .to_string();
        let expires_in = token_obj["expires_in"].as_i64().ok_or("缺少 expires_in")?;
        let timestamp = token_obj["expiry_timestamp"]
            .as_i64()
            .ok_or("缺少 expiry_timestamp")?;
        let project_id = token_obj
            .get("project_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let subscription_tier = account
            .get("quota")
            .and_then(|q| q.get("subscription_tier"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(Some(ProxyToken {
            account_id,
            access_token,
            refresh_token,
            expires_in,
            timestamp,
            email,
            account_path: Some(path.clone()),
            project_id,
            subscription_tier,
            disabled: false,
            disabled_reason: None,
        }))
    }

    /// 获取当前可用的 Token（支持粘性会话与智能调度）
    /// 返回: (access_token, project_id, email, account_id)
    pub async fn get_token(
        &self,
        quota_group: &str,
        force_rotate: bool,
        session_id: Option<&str>,
    ) -> Result<(String, String, String, String), String> {
        let mut tokens_snapshot: Vec<ProxyToken> =
            self.tokens.iter().map(|e| e.value().clone()).collect();
        let total = tokens_snapshot.len();
        if total == 0 {
            return Err("Token pool is empty".to_string());
        }

        // ===== 根据订阅等级排序 (优先级: ULTRA > PRO > FREE) =====
        tokens_snapshot.sort_by(|a, b| {
            let tier_priority = |tier: &Option<String>| match tier.as_deref() {
                Some("ULTRA") => 0,
                Some("PRO") => 1,
                Some("FREE") => 2,
                _ => 3,
            };
            tier_priority(&a.subscription_tier).cmp(&tier_priority(&b.subscription_tier))
        });

        let scheduling = self.sticky_config.read().await.clone();
        use crate::proxy::sticky_config::SchedulingMode;

        let mut attempted: HashSet<String> = HashSet::new();
        let mut last_error: Option<String> = None;

        for attempt in 0..total {
            let rotate = force_rotate || attempt > 0;
            let mut target_token: Option<ProxyToken> = None;

            // 模式 A: 粘性会话处理
            if !rotate
                && session_id.is_some()
                && scheduling.mode != SchedulingMode::PerformanceFirst
            {
                let sid = session_id.unwrap();
                if let Some(bound_id) = self.session_accounts.get(sid).map(|v| v.clone()) {
                    let reset_sec = self.rate_limit_tracker.get_remaining_wait(&bound_id);
                    if reset_sec > 0 {
                        if scheduling.mode == SchedulingMode::CacheFirst
                            && reset_sec <= scheduling.max_wait_seconds
                        {
                            tracing::warn!(
                                "Cache-first: Session {} bound to {} is limited. Waiting {}s...",
                                sid,
                                bound_id,
                                reset_sec
                            );
                            tokio::time::sleep(std::time::Duration::from_secs(reset_sec)).await;
                            if let Some(found) =
                                tokens_snapshot.iter().find(|t| t.account_id == bound_id)
                            {
                                target_token = Some(found.clone());
                            }
                        } else {
                            self.session_accounts.remove(sid);
                        }
                    } else if !attempted.contains(&bound_id) {
                        if let Some(found) =
                            tokens_snapshot.iter().find(|t| t.account_id == bound_id)
                        {
                            target_token = Some(found.clone());
                        }
                    }
                }
            }

            // 模式 B: 原子化 60s 全局锁定
            if target_token.is_none() && !rotate && quota_group != "image_gen" {
                let mut last_used = self.last_used_account.lock().await;
                if let Some((account_id, last_time)) = &*last_used {
                    if last_time.elapsed().as_secs() < 60 && !attempted.contains(account_id) {
                        if let Some(found) =
                            tokens_snapshot.iter().find(|t| &t.account_id == account_id)
                        {
                            target_token = Some(found.clone());
                        }
                    }
                }

                if target_token.is_none() {
                    let start_idx = self.current_index.fetch_add(1, Ordering::SeqCst) % total;
                    for offset in 0..total {
                        let idx = (start_idx + offset) % total;
                        let candidate = &tokens_snapshot[idx];
                        if attempted.contains(&candidate.account_id)
                            || self.is_rate_limited(&candidate.account_id)
                        {
                            continue;
                        }
                        target_token = Some(candidate.clone());
                        *last_used =
                            Some((candidate.account_id.clone(), std::time::Instant::now()));
                        if let Some(sid) = session_id {
                            if scheduling.mode != SchedulingMode::PerformanceFirst {
                                self.session_accounts
                                    .insert(sid.to_string(), candidate.account_id.clone());
                            }
                        }
                        break;
                    }
                }
            } else if target_token.is_none() {
                // 模式 C: 纯轮询模式
                let start_idx = self.current_index.fetch_add(1, Ordering::SeqCst) % total;
                for offset in 0..total {
                    let idx = (start_idx + offset) % total;
                    let candidate = &tokens_snapshot[idx];
                    if attempted.contains(&candidate.account_id)
                        || self.is_rate_limited(&candidate.account_id)
                    {
                        continue;
                    }
                    target_token = Some(candidate.clone());
                    break;
                }
            }

            let mut token = match target_token {
                Some(t) => t,
                None => {
                    let min_wait = tokens_snapshot
                        .iter()
                        .filter_map(|t| self.rate_limit_tracker.get_reset_seconds(&t.account_id))
                        .min()
                        .unwrap_or(60);
                    return Err(format!(
                        "All accounts are currently limited. Please wait {}s.",
                        min_wait
                    ));
                }
            };

            // 检查过期 (提前 300s 刷新)
            let now = chrono::Utc::now().timestamp();
            if now >= token.timestamp - 300 {
                match crate::core::services::oauth::refresh_access_token(&token.refresh_token).await
                {
                    Ok(token_response) => {
                        token.access_token = token_response.access_token.clone();
                        token.expires_in = token_response.expires_in;
                        token.timestamp = now + token_response.expires_in;
                        if let Some(mut entry) = self.tokens.get_mut(&token.account_id) {
                            entry.access_token = token.access_token.clone();
                            entry.expires_in = token.expires_in;
                            entry.timestamp = token.timestamp;
                        }
                        let _ = self
                            .save_refreshed_token(&token.account_id, &token_response)
                            .await;
                    }
                    Err(e) => {
                        if e.contains("invalid_grant") {
                            let _ = self
                                .disable_account(
                                    &token.account_id,
                                    &format!("invalid_grant: {}", e),
                                )
                                .await;
                            self.tokens.remove(&token.account_id);
                        }
                        last_error = Some(format!("Token refresh failed: {}", e));
                        attempted.insert(token.account_id.clone());
                        continue;
                    }
                }
            }

            // 检查 project_id
            let project_id = if let Some(pid) = &token.project_id {
                pid.clone()
            } else {
                match crate::proxy::project_resolver::fetch_project_id(&token.access_token).await {
                    Ok(pid) => {
                        if let Some(mut entry) = self.tokens.get_mut(&token.account_id) {
                            entry.project_id = Some(pid.clone());
                        }
                        let _ = self.save_project_id(&token.account_id, &pid).await;
                        pid
                    }
                    Err(e) => {
                        last_error = Some(format!("Failed to fetch project_id: {}", e));
                        attempted.insert(token.account_id.clone());
                        continue;
                    }
                }
            };

            return Ok((
                token.access_token,
                project_id,
                token.email,
                token.account_id,
            ));
        }

        Err(last_error.unwrap_or_else(|| "All accounts failed".to_string()))
    }

    /// 禁用账号 (更新数据库)
    pub async fn disable_account(&self, account_id: &str, reason: &str) -> Result<(), String> {
        let now = chrono::Utc::now().timestamp();
        let reason_truncated = truncate_reason(reason, 800);

        sqlx::query("UPDATE accounts SET disabled = 1, disabled_reason = ? WHERE id = ?")
            .bind(reason_truncated.clone())
            .bind(account_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to disable account in DB: {}", e))?;

        // 同时也尝试更新 JSON (如果存在的话，兼容旧版本)
        if let Some(entry) = self.tokens.get(account_id) {
            if let Some(path) = &entry.account_path {
                if let Ok(content) = std::fs::read_to_string(path) {
                    if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                        json["disabled"] = serde_json::Value::Bool(true);
                        json["disabled_at"] = serde_json::Value::Number(now.into());
                        json["disabled_reason"] = serde_json::Value::String(reason_truncated);
                        let _ = std::fs::write(path, serde_json::to_string_pretty(&json).unwrap());
                    }
                }
            }
        }

        self.tokens.remove(account_id);
        tracing::warn!(
            "Account disabled in DB: {} (reason: {})",
            account_id,
            reason
        );
        Ok(())
    }

    /// 保存 project_id (更新数据库)
    async fn save_project_id(&self, account_id: &str, project_id: &str) -> Result<(), String> {
        sqlx::query("UPDATE accounts SET project_id = ? WHERE id = ?")
            .bind(project_id)
            .bind(account_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to save project_id to DB: {}", e))?;
        Ok(())
    }

    /// 保存刷新后的 token (更新数据库)
    async fn save_refreshed_token(
        &self,
        account_id: &str,
        token_response: &crate::core::services::oauth::TokenResponse,
    ) -> Result<(), String> {
        let now = chrono::Utc::now().timestamp();
        let expiry = now + token_response.expires_in;

        sqlx::query(
            "UPDATE accounts SET access_token = ?, expires_in = ?, expiry_timestamp = ? WHERE id = ?"
        )
        .bind(&token_response.access_token)
        .bind(token_response.expires_in)
        .bind(expiry)
        .bind(account_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to save token to DB: {}", e))?;
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn mark_rate_limited(
        &self,
        account_id: &str,
        status: u16,
        retry_after: Option<&str>,
        error: &str,
    ) {
        self.rate_limit_tracker
            .parse_from_error(account_id, status, retry_after, error);
    }

    pub fn is_rate_limited(&self, account_id: &str) -> bool {
        self.rate_limit_tracker.is_rate_limited(account_id)
    }

    pub async fn get_sticky_config(&self) -> StickySessionConfig {
        self.sticky_config.read().await.clone()
    }

    pub async fn update_sticky_config(&self, new: StickySessionConfig) {
        *self.sticky_config.write().await = new;
    }

    pub fn clear_all_sessions(&self) {
        self.session_accounts.clear();
    }

    /// 根据 Email 获取指定账号的 Token 信息 (用于预热等)
    pub async fn get_token_by_email(
        &self,
        email: &str,
    ) -> Result<(String, String, String), String> {
        // 1. 在内存中查找
        for entry in self.tokens.iter() {
            let token = entry.value();
            if token.email == email {
                return Ok((
                    token.access_token.clone(),
                    token.project_id.clone().unwrap_or_default(),
                    token.account_id.clone(),
                ));
            }
        }

        // 2. 如果内存中没有，尝试从数据库加载 (可能是已禁用的或者刚添加的)
        let row = sqlx::query(
            "SELECT access_token, project_id, id FROM accounts WHERE email = ? LIMIT 1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("DB error: {}", e))?;

        if let Some(row) = row {
            use sqlx::Row;
            Ok((
                row.get("access_token"),
                row.get::<Option<String>, _>("project_id")
                    .unwrap_or_default(),
                row.get("id"),
            ))
        } else {
            Err(format!("Account not found for email: {}", email))
        }
    }
}

fn truncate_reason(reason: &str, max_len: usize) -> String {
    if reason.chars().count() <= max_len {
        return reason.to_string();
    }
    let mut s: String = reason.chars().take(max_len).collect();
    s.push('…');
    s
}
