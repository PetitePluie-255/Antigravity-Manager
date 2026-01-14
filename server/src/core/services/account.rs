//! 账户服务
//! 账户 CRUD 操作，使用 SQLite 数据库持久化

use crate::core::models::{Account, DeviceProfile, DeviceProfileVersion, QuotaData, TokenData};
use crate::core::traits::EventEmitter;
use sqlx::{Row, SqlitePool};

/// 账户服务
pub struct AccountService;

impl AccountService {
    /// 加载单个账户
    pub async fn load_account(pool: &SqlitePool, account_id: &str) -> Result<Account, String> {
        let row = sqlx::query("SELECT * FROM accounts WHERE id = ?")
            .bind(account_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("加载账户失败: {}", e))?
            .ok_or_else(|| format!("账户不存在: {}", account_id))?;

        Ok(Self::map_row_to_account(&row))
    }

    /// 列出所有账户
    pub async fn list_accounts(pool: &SqlitePool) -> Result<Vec<Account>, String> {
        let rows = sqlx::query("SELECT * FROM accounts ORDER BY created_at DESC")
            .fetch_all(pool)
            .await
            .map_err(|e| format!("加载账户列表失败: {}", e))?;

        Ok(rows.iter().map(Self::map_row_to_account).collect())
    }

    /// 添加账户
    pub async fn add_account<E: EventEmitter>(
        pool: &SqlitePool,
        emitter: &E,
        email: String,
        name: Option<String>,
        token: TokenData,
    ) -> Result<Account, String> {
        // 检查是否已存在相同邮箱的账户
        let existing = sqlx::query("SELECT id FROM accounts WHERE email = ?")
            .bind(&email)
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("查询账户失败: {}", e))?;

        if existing.is_some() {
            return Err(format!("账户已存在: {}", email));
        }

        let id = uuid::Uuid::new_v4().to_string();
        let mut account = Account::new(id.clone(), email.clone(), token.clone());

        // 生成初始设备指纹: 优先使用全局原始指纹，否则生成并保存为全局原始
        let profile = if let Some(global) = crate::core::device::load_global_original() {
            global
        } else {
            let new_profile = crate::core::device::generate_profile();
            let _ = crate::core::device::save_global_original(&new_profile);
            new_profile
        };

        account.device_profile = Some(profile.clone());
        account.device_history.push(DeviceProfileVersion {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: account.created_at,
            label: "初始指纹".to_string(),
            profile,
            is_current: true,
        });

        let quota_json = serde_json::to_string(&account.quota).unwrap_or_default();
        let device_profile_json =
            serde_json::to_string(&account.device_profile).unwrap_or_default();
        let device_history_json =
            serde_json::to_string(&account.device_history).unwrap_or_default();

        sqlx::query(
            "INSERT INTO accounts (id, email, name, access_token, refresh_token, expires_in, expiry_timestamp, created_at, last_used, quota, device_profile, device_history) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(&email)
        .bind(&name)
        .bind(&token.access_token)
        .bind(&token.refresh_token)
        .bind(token.expires_in)
        .bind(token.expiry_timestamp)
        .bind(account.created_at)
        .bind(account.last_used)
        .bind(quota_json)
        .bind(device_profile_json)
        .bind(device_history_json)
        .execute(pool)
        .await
        .map_err(|e| format!("添加账户到数据库失败: {}", e))?;

        // 维护 is_current: 如果是第一个账户，设为当前
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

        if count == 1 {
            sqlx::query("UPDATE accounts SET is_current = 1 WHERE id = ?")
                .bind(&id)
                .execute(pool)
                .await
                .ok();
        }

        let mut final_account = account;
        final_account.name = name;
        emitter.emit("account-added", &final_account);

        Ok(final_account)
    }

    /// 添加或更新账户 (upsert)
    pub async fn upsert_account<E: EventEmitter>(
        pool: &SqlitePool,
        emitter: &E,
        email: String,
        name: Option<String>,
        token: TokenData,
    ) -> Result<Account, String> {
        let existing = sqlx::query("SELECT id FROM accounts WHERE email = ?")
            .bind(&email)
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("查询账户失败: {}", e))?;

        if let Some(row) = existing {
            let id: String = row.get("id");
            let mut account = Self::load_account(pool, &id).await?;
            account.token = token.clone();
            account.name = name.clone();
            account.update_last_used();

            sqlx::query(
                "UPDATE accounts SET access_token = ?, refresh_token = ?, expires_in = ?, expiry_timestamp = ?, name = ?, last_used = ? WHERE id = ?"
            )
            .bind(&token.access_token)
            .bind(&token.refresh_token)
            .bind(token.expires_in)
            .bind(token.expiry_timestamp)
            .bind(&name)
            .bind(account.last_used)
            .bind(&id)
            .execute(pool)
            .await
            .map_err(|e| format!("更新账户失败: {}", e))?;

            emitter.emit("account-updated", &account);
            return Ok(account);
        }

        Self::add_account(pool, emitter, email, name, token).await
    }

    /// 删除账户
    pub async fn delete_account<E: EventEmitter>(
        pool: &SqlitePool,
        emitter: &E,
        account_id: &str,
    ) -> Result<(), String> {
        let _ = Self::load_account(pool, account_id).await?;

        sqlx::query("DELETE FROM accounts WHERE id = ?")
            .bind(account_id)
            .execute(pool)
            .await
            .map_err(|e| format!("删除账户失败: {}", e))?;

        // 如果删除了当前账户，尝试选另一个
        let current_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts WHERE is_current = 1)")
                .fetch_one(pool)
                .await
                .unwrap_or(false);

        if !current_exists {
            sqlx::query(
                "UPDATE accounts SET is_current = 1 WHERE id IN (SELECT id FROM accounts LIMIT 1)",
            )
            .execute(pool)
            .await
            .ok();
        }

        emitter.emit("account-deleted", account_id);
        Ok(())
    }

    /// 批量删除账户
    pub async fn delete_accounts<E: EventEmitter>(
        pool: &SqlitePool,
        emitter: &E,
        account_ids: &[String],
    ) -> Result<(), String> {
        for id in account_ids {
            Self::delete_account(pool, emitter, id).await.ok();
        }
        emitter.emit("accounts-deleted", account_ids);
        Ok(())
    }

    /// 切换当前账户
    pub async fn switch_account<E: EventEmitter>(
        pool: &SqlitePool,
        emitter: &E,
        account_id: &str,
    ) -> Result<(), String> {
        // 先检查是否存在
        let _ = Self::load_account(pool, account_id).await?;

        let mut tx = pool.begin().await.map_err(|e| format!("{}", e))?;

        sqlx::query("UPDATE accounts SET is_current = 0")
            .execute(&mut *tx)
            .await
            .ok();
        sqlx::query("UPDATE accounts SET is_current = 1 WHERE id = ?")
            .bind(account_id)
            .execute(&mut *tx)
            .await
            .ok();
        sqlx::query("UPDATE accounts SET last_used = ? WHERE id = ?")
            .bind(chrono::Utc::now().timestamp())
            .bind(account_id)
            .execute(&mut *tx)
            .await
            .ok();

        tx.commit().await.map_err(|e| format!("{}", e))?;

        emitter.emit("account-switched", account_id);
        Ok(())
    }

    /// 获取当前账户
    pub async fn get_current_account(pool: &SqlitePool) -> Result<Option<Account>, String> {
        let row = sqlx::query("SELECT * FROM accounts WHERE is_current = 1 LIMIT 1")
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("加载当前账户失败: {}", e))?;

        if let Some(row) = row {
            Ok(Some(Self::map_row_to_account(&row)))
        } else {
            // 如果没有标记为 current 的，尝试取第一个
            let first = sqlx::query("SELECT * FROM accounts LIMIT 1")
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("加载首个账户失败: {}", e))?;

            Ok(first.map(|r| Self::map_row_to_account(&r)))
        }
    }

    /// 更新账户配额
    pub async fn update_account_quota(
        pool: &SqlitePool,
        account_id: &str,
        quota: QuotaData,
    ) -> Result<(), String> {
        let quota_json = serde_json::to_string(&quota).unwrap_or_default();
        sqlx::query("UPDATE accounts SET quota = ? WHERE id = ?")
            .bind(quota_json)
            .bind(account_id)
            .execute(pool)
            .await
            .map_err(|e| format!("更新配额失败: {}", e))?;
        Ok(())
    }

    /// 更新账户设备指纹
    pub async fn update_account_device(
        pool: &SqlitePool,
        account_id: &str,
        device_profile: Option<DeviceProfile>,
        device_history: Vec<DeviceProfileVersion>,
    ) -> Result<(), String> {
        let device_profile_json = serde_json::to_string(&device_profile).unwrap_or_default();
        let device_history_json = serde_json::to_string(&device_history).unwrap_or_default();
        sqlx::query("UPDATE accounts SET device_profile = ?, device_history = ? WHERE id = ?")
            .bind(device_profile_json)
            .bind(device_history_json)
            .bind(account_id)
            .execute(pool)
            .await
            .map_err(|e| format!("更新设备指纹失败: {}", e))?;
        Ok(())
    }

    /// 导出所有账户的 refresh_token
    pub async fn export_accounts(pool: &SqlitePool) -> Result<Vec<(String, String)>, String> {
        let accounts = Self::list_accounts(pool).await?;
        Ok(accounts
            .into_iter()
            .map(|a| (a.email, a.token.refresh_token))
            .collect())
    }

    /// 辅助方法：将 SQL 行映射到 Account 结构体
    fn map_row_to_account(row: &sqlx::sqlite::SqliteRow) -> Account {
        let quota_raw: Option<String> = row.get("quota");
        let quota: Option<QuotaData> = quota_raw.and_then(|s| serde_json::from_str(&s).ok());

        let device_profile_raw: Option<String> = row.get("device_profile");
        let device_profile: Option<DeviceProfile> =
            device_profile_raw.and_then(|s| serde_json::from_str(&s).ok());

        let device_history_raw: Option<String> = row.get("device_history");
        let device_history: Vec<DeviceProfileVersion> = device_history_raw
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        Account {
            id: row.get("id"),
            email: row.get("email"),
            name: row.get("name"),
            token: TokenData {
                access_token: row.get("access_token"),
                refresh_token: row.get("refresh_token"),
                expires_in: row.get("expires_in"),
                expiry_timestamp: row.get("expiry_timestamp"),
                token_type: "Bearer".to_string(),
                email: Some(row.get("email")),
                project_id: row.get("project_id"),
                session_id: None,
            },
            device_profile,
            device_history,
            quota,
            disabled: row.get("disabled"),
            disabled_reason: row.get("disabled_reason"),
            disabled_at: row.get("disabled_at"),
            proxy_disabled: row.get("proxy_disabled"),
            proxy_disabled_reason: row.get("proxy_disabled_reason"),
            proxy_disabled_at: row.get("proxy_disabled_at"),
            created_at: row.get("created_at"),
            last_used: row.get("last_used"),
        }
    }
}
