//! 代理日志存储
//! 使用 SQLite 数据库存储日志

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// 代理日志条目
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProxyLogEntry {
    pub id: i64,
    pub timestamp: i64,
    pub method: String,
    pub url: String,
    pub account_email: String,
    pub model: String,
    pub tokens_in: u32,
    pub tokens_out: u32,
    pub latency_ms: u32,
    pub status_code: u16,
    pub error: Option<String>,
    pub request_body: Option<String>,
    pub response_body: Option<String>,
}

/// 日志存储（基于数据库）
pub struct LogStore {
    pool: SqlitePool,
}

impl LogStore {
    /// 创建新的日志存储
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 记录一条日志（异步存入数据库）
    pub fn record(
        &self,
        method: String,
        url: String,
        account_email: String,
        model: String,
        tokens_in: u32,
        tokens_out: u32,
        latency_ms: u32,
        status_code: u16,
        error: Option<String>,
        request_body: Option<String>,
        response_body: Option<String>,
    ) {
        let pool = self.pool.clone();
        let timestamp = chrono::Utc::now().timestamp();

        // 异步写入日志，不阻塞当前请求
        tokio::spawn(async move {
            let result = sqlx::query(
                "INSERT INTO proxy_logs (
                    timestamp, method, url, account_email, model, 
                    tokens_in, tokens_out, latency_ms, status_code, 
                    error, request_body, response_body
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(timestamp)
            .bind(method)
            .bind(url)
            .bind(account_email)
            .bind(model)
            .bind(tokens_in)
            .bind(tokens_out)
            .bind(latency_ms)
            .bind(status_code)
            .bind(error)
            .bind(request_body)
            .bind(response_body)
            .execute(&pool)
            .await;

            if let Err(e) = result {
                tracing::error!("Failed to record proxy log to database: {}", e);
            }

            // 维护日志上限（可选，SQLite 可以存很多，暂时保留 5000 条）
            let _ = sqlx::query(
                "DELETE FROM proxy_logs WHERE id IN (
                    SELECT id FROM proxy_logs ORDER BY id DESC LIMIT -1 OFFSET 5000
                )",
            )
            .execute(&pool)
            .await;
        });
    }

    /// 获取日志（支持分页）
    pub async fn get_logs(&self, limit: usize, offset: usize) -> Vec<ProxyLogEntry> {
        let result = sqlx::query_as::<_, ProxyLogEntry>(
            "SELECT * FROM proxy_logs ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await;

        match result {
            Ok(logs) => logs,
            Err(e) => {
                tracing::error!("Failed to fetch proxy logs from database: {}", e);
                Vec::new()
            }
        }
    }

    /// 获取日志总数
    pub async fn len(&self) -> usize {
        let result = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM proxy_logs")
            .fetch_one(&self.pool)
            .await;

        match result {
            Ok(count) => count as usize,
            Err(e) => {
                tracing::error!("Failed to count proxy logs in database: {}", e);
                0
            }
        }
    }

    /// 清除所有日志
    pub async fn clear(&self) {
        let _ = sqlx::query("DELETE FROM proxy_logs")
            .execute(&self.pool)
            .await;
    }
}
