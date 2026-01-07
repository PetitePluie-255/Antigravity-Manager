//! 代理日志存储
//! 使用内存环形缓冲区存储日志

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

/// 代理日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyLogEntry {
    pub id: u64,
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

/// 日志存储（环形缓冲区）
pub struct LogStore {
    logs: RwLock<VecDeque<ProxyLogEntry>>,
    max_size: usize,
    next_id: AtomicU64,
}

impl LogStore {
    /// 创建新的日志存储
    pub fn new(max_size: usize) -> Self {
        Self {
            logs: RwLock::new(VecDeque::with_capacity(max_size)),
            max_size,
            next_id: AtomicU64::new(1),
        }
    }

    /// 记录一条日志
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
        let entry = ProxyLogEntry {
            id: self.next_id.fetch_add(1, Ordering::SeqCst),
            timestamp: chrono::Utc::now().timestamp(),
            method,
            url,
            account_email,
            model,
            tokens_in,
            tokens_out,
            latency_ms,
            status_code,
            error,
            request_body,
            response_body,
        };

        let mut logs = self.logs.write().unwrap();

        // 如果超过最大容量，移除最旧的
        if logs.len() >= self.max_size {
            logs.pop_front();
        }

        logs.push_back(entry);
    }

    /// 获取日志（支持分页）
    pub fn get_logs(&self, limit: usize, offset: usize) -> Vec<ProxyLogEntry> {
        let logs = self.logs.read().unwrap();

        // 从后往前取（最新的在前）
        logs.iter()
            .rev()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect()
    }

    /// 获取日志总数
    pub fn len(&self) -> usize {
        self.logs.read().unwrap().len()
    }

    /// 清除所有日志
    pub fn clear(&self) {
        let mut logs = self.logs.write().unwrap();
        logs.clear();
    }
}

impl Default for LogStore {
    fn default() -> Self {
        Self::new(1000) // 默认保留 1000 条
    }
}
