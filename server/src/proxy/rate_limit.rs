use dashmap::DashMap;
use regex::Regex;
use std::time::{Duration, SystemTime};

/// 限流原因类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RateLimitReason {
    /// 配额耗尽 (QUOTA_EXHAUSTED)
    QuotaExhausted,
    /// 模型并发/容量耗尽 (MODEL_CAPACITY_EXHAUSTED)
    ModelCapacityExhausted,
    /// 速率限制 (RATE_LIMIT_EXCEEDED)
    RateLimitExceeded,
    /// 服务器错误 (5xx)
    ServerError,
    /// 未知原因
    Unknown,
}

/// 限流信息
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    /// 限流重置时间
    pub reset_time: SystemTime,
    /// 重试间隔(秒)
    #[allow(dead_code)]
    pub retry_after_sec: u64,
    /// 检测时间
    #[allow(dead_code)]
    pub detected_at: SystemTime,
    /// 限流原因
    pub reason: RateLimitReason,
    /// 关联模型 (可选)
    pub model: Option<String>,
}

/// 限流跟踪器
pub struct RateLimitTracker {
    limits: DashMap<String, RateLimitInfo>,
}

impl RateLimitTracker {
    pub fn new() -> Self {
        Self {
            limits: DashMap::new(),
        }
    }

    /// 获取账号剩余的等待时间(秒)
    /// # Arguments
    /// * `account_id` - 账号 ID
    /// * `model` - 可选模型名
    pub fn get_remaining_wait(&self, account_id: &str, model: Option<&str>) -> u64 {
        let mut max_wait = 0;

        // 1. 检查账号级别锁定
        if let Some(info) = self.limits.get(account_id) {
            let now = SystemTime::now();
            if info.reset_time > now {
                max_wait = info
                    .reset_time
                    .duration_since(now)
                    .unwrap_or_default()
                    .as_secs();
            }
        }

        // 2. 检查特定模型锁定
        if let Some(m) = model {
            let key = format!("{}:{}", account_id, m);
            if let Some(info) = self.limits.get(&key) {
                let now = SystemTime::now();
                if info.reset_time > now {
                    let wait = info
                        .reset_time
                        .duration_since(now)
                        .unwrap_or_default()
                        .as_secs();
                    max_wait = max_wait.max(wait);
                }
            }
        }

        max_wait
    }

    /// 手动设置锁定到指定时间
    pub fn set_lockout_until(
        &self,
        account_id: &str,
        reset_time: SystemTime,
        reason: RateLimitReason,
        model: Option<String>,
    ) {
        let key = if let Some(m) = &model {
            format!("{}:{}", account_id, m)
        } else {
            account_id.to_string()
        };

        let now = SystemTime::now();
        let retry_after_sec = reset_time.duration_since(now).unwrap_or_default().as_secs();

        let info = RateLimitInfo {
            reset_time,
            retry_after_sec,
            detected_at: now,
            reason,
            model: model.clone(),
        };

        self.limits.insert(key, info);

        if let Some(m) = model {
            tracing::warn!(
                "账号 {} 的模型 {} 被手动锁定至 {:?}",
                account_id,
                m,
                reset_time
            );
        } else {
            tracing::warn!("账号 {} 被手动锁定至 {:?}", account_id, reset_time);
        }
    }

    pub fn set_lockout_until_iso(
        &self,
        account_id: &str,
        reset_time_str: &str,
        reason: RateLimitReason,
        model: Option<String>,
    ) -> bool {
        // 尝试解析 ISO 8601 格式
        match chrono::DateTime::parse_from_rfc3339(reset_time_str) {
            Ok(dt) => {
                let reset_time =
                    SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(dt.timestamp() as u64);
                self.set_lockout_until(account_id, reset_time, reason, model);
                true
            }
            Err(e) => {
                tracing::warn!(
                    "无法解析配额刷新时间 '{}': {},将使用默认退避策略",
                    reset_time_str,
                    e
                );
                false
            }
        }
    }

    /// 从错误响应解析限流信息
    ///
    /// # Arguments
    /// * `account_id` - 账号 ID
    /// * `status` - HTTP 状态码
    /// * `retry_after_header` - Retry-After header 值
    /// * `body` - 错误响应 body
    pub fn parse_from_error(
        &self,
        account_id: &str,
        status: u16,
        retry_after_header: Option<&str>,
        body: &str,
        model: Option<String>,
    ) -> Option<RateLimitInfo> {
        // 支持 429 (限流) 以及 500/503/529 (后端故障软避让)
        if status != 429 && status != 500 && status != 503 && status != 529 {
            return None;
        }

        // 1. 解析限流原因类型
        let reason = if status == 429 {
            self.parse_rate_limit_reason(body)
        } else {
            RateLimitReason::ServerError
        };

        let mut retry_after_sec = None;

        // 2. 从 Retry-After header 提取
        if let Some(retry_after) = retry_after_header {
            if let Ok(seconds) = retry_after.parse::<u64>() {
                retry_after_sec = Some(seconds);
            }
        }

        // 3. 从错误消息提取 (优先尝试 JSON 解析，再试正则)
        if retry_after_sec.is_none() {
            retry_after_sec = self.parse_retry_time_from_body(body);
        }

        // 4. 处理默认值与软避让逻辑（根据限流类型设置不同默认值）
        let retry_sec = match retry_after_sec {
            Some(s) => {
                // 引入 PR #28 的安全缓冲区：最小 2 秒，防止极高频无效重试
                if s < 2 {
                    2
                } else {
                    s
                }
            }
            None => {
                match reason {
                    RateLimitReason::QuotaExhausted => {
                        // 配额耗尽：使用较长的默认值（1小时），避免频繁重试
                        tracing::warn!(
                            "检测到配额耗尽 (QUOTA_EXHAUSTED)，使用默认值 3600秒 (1小时)"
                        );
                        3600
                    }
                    RateLimitReason::RateLimitExceeded => {
                        // 速率限制：使用较短的默认值（30秒），可以较快恢复
                        tracing::debug!("检测到速率限制 (RATE_LIMIT_EXCEEDED)，使用默认值 30秒");
                        30
                    }
                    RateLimitReason::ServerError => {
                        // 服务器错误：执行"软避让"，默认锁定 20 秒
                        tracing::warn!("检测到 5xx 错误 ({}), 执行 20s 软避让...", status);
                        20
                    }
                    RateLimitReason::ModelCapacityExhausted => {
                        // 模型容量耗尽：默认锁定 2 分钟，因为它通常比总量配额恢复得快
                        tracing::debug!(
                            "检测到模型容量限流 (MODEL_CAPACITY_EXHAUSTED)，使用默认值 120秒"
                        );
                        120
                    }
                    RateLimitReason::Unknown => {
                        // 未知原因：使用中等默认值（60秒）
                        tracing::debug!("无法解析 429 限流原因, 使用默认值 60秒");
                        60
                    }
                }
            }
        };

        let info = RateLimitInfo {
            reset_time: SystemTime::now() + Duration::from_secs(retry_sec),
            retry_after_sec: retry_sec,
            detected_at: SystemTime::now(),
            reason,
            model: model.clone(),
        };

        // 存储
        let key = if let Some(m) = &model {
            format!("{}:{}", account_id, m)
        } else {
            account_id.to_string()
        };
        self.limits.insert(key, info.clone());

        if let Some(m) = model {
            tracing::warn!(
                "账号 {} (模型 {}) [{}] 限流类型: {:?}, 重置延时: {}秒",
                account_id,
                m,
                status,
                reason,
                retry_sec
            );
        } else {
            tracing::warn!(
                "账号 {} [{}] 限流类型: {:?}, 重置延时: {}秒",
                account_id,
                status,
                reason,
                retry_sec
            );
        }

        Some(info)
    }

    /// 解析限流原因类型
    fn parse_rate_limit_reason(&self, body: &str) -> RateLimitReason {
        // 尝试从 JSON 中提取 reason 字段
        let trimmed = body.trim();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if let Some(reason_str) = json
                    .get("error")
                    .and_then(|e| e.get("details"))
                    .and_then(|d| d.as_array())
                    .and_then(|a| a.get(0))
                    .and_then(|o| o.get("reason"))
                    .and_then(|v| v.as_str())
                {
                    return match reason_str {
                        "QUOTA_EXHAUSTED" => RateLimitReason::QuotaExhausted,
                        "RATE_LIMIT_EXCEEDED" => RateLimitReason::RateLimitExceeded,
                        _ => RateLimitReason::Unknown,
                    };
                }
            }
        }

        // 如果无法从 JSON 解析，尝试从消息文本判断
        if body.contains("exhausted") || body.contains("quota") {
            // 进一步区分模型并发和总量配额
            if body.contains("model_capacity")
                || body.contains("Tokens per minute")
                || body.contains("Requests per minute")
            {
                RateLimitReason::ModelCapacityExhausted
            } else {
                RateLimitReason::QuotaExhausted
            }
        } else if body.contains("rate limit") || body.contains("too many requests") {
            RateLimitReason::RateLimitExceeded
        } else {
            RateLimitReason::Unknown
        }
    }

    /// 通用时间解析函数：支持 "2h1m1s" 等所有格式组合
    fn parse_duration_string(&self, s: &str) -> Option<u64> {
        tracing::debug!("[时间解析] 尝试解析: '{}'", s);

        // 使用正则表达式提取小时、分钟、秒、毫秒
        // 支持格式："2h1m1s", "1h30m", "5m", "30s", "500ms" 等
        let re = Regex::new(r"(?:(\d+)h)?(?:(\d+)m)?(?:(\d+(?:\.\d+)?)s)?(?:(\d+)ms)?").ok()?;
        let caps = match re.captures(s) {
            Some(c) => c,
            None => {
                tracing::warn!("[时间解析] 正则未匹配: '{}'", s);
                return None;
            }
        };

        let hours = caps
            .get(1)
            .and_then(|m| m.as_str().parse::<u64>().ok())
            .unwrap_or(0);
        let minutes = caps
            .get(2)
            .and_then(|m| m.as_str().parse::<u64>().ok())
            .unwrap_or(0);
        let seconds = caps
            .get(3)
            .and_then(|m| m.as_str().parse::<f64>().ok())
            .unwrap_or(0.0);
        let milliseconds = caps
            .get(4)
            .and_then(|m| m.as_str().parse::<u64>().ok())
            .unwrap_or(0);

        tracing::debug!(
            "[时间解析] 提取结果: {}h {}m {:.3}s {}ms",
            hours,
            minutes,
            seconds,
            milliseconds
        );

        // 计算总秒数
        let total_seconds =
            hours * 3600 + minutes * 60 + seconds.ceil() as u64 + (milliseconds + 999) / 1000;

        // 如果总秒数为 0，说明解析失败
        if total_seconds == 0 {
            tracing::warn!("[时间解析] 失败: '{}' (总秒数为0)", s);
            None
        } else {
            tracing::info!(
                "[时间解析] ✓ 成功: '{}' => {}秒 ({}h {}m {:.1}s)",
                s,
                total_seconds,
                hours,
                minutes,
                seconds
            );
            Some(total_seconds)
        }
    }

    /// 从错误消息 body 中解析重置时间
    fn parse_retry_time_from_body(&self, body: &str) -> Option<u64> {
        // A. 优先尝试 JSON 精准解析 (借鉴 PR #28)
        let trimmed = body.trim();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
                // 1. Google 常见的 quotaResetDelay 格式 (支持所有格式："2h1m1s", "1h30m", "42s", "500ms" 等)
                // 路径: error.details[0].metadata.quotaResetDelay
                if let Some(delay_str) = json
                    .get("error")
                    .and_then(|e| e.get("details"))
                    .and_then(|d| d.as_array())
                    .and_then(|a| a.get(0))
                    .and_then(|o| o.get("metadata")) // 添加 metadata 层级
                    .and_then(|m| m.get("quotaResetDelay"))
                    .and_then(|v| v.as_str())
                {
                    tracing::debug!("[JSON解析] 找到 quotaResetDelay: '{}'", delay_str);

                    // 使用通用时间解析函数
                    if let Some(seconds) = self.parse_duration_string(delay_str) {
                        return Some(seconds);
                    }
                }

                // 2. OpenAI 常见的 retry_after 字段 (数字)
                if let Some(retry) = json
                    .get("error")
                    .and_then(|e| e.get("retry_after"))
                    .and_then(|v| v.as_u64())
                {
                    return Some(retry);
                }
            }
        }

        // B. 正则匹配模式 (兜底)
        // 模式 1: "Try again in 2m 30s"
        if let Ok(re) = Regex::new(r"(?i)try again in (\d+)m\s*(\d+)s") {
            if let Some(caps) = re.captures(body) {
                if let (Ok(m), Ok(s)) = (caps[1].parse::<u64>(), caps[2].parse::<u64>()) {
                    return Some(m * 60 + s);
                }
            }
        }

        // 模式 2: "Try again in 30s" 或 "backoff for 42s"
        if let Ok(re) = Regex::new(r"(?i)(?:try again in|backoff for|wait)\s*(\d+)s") {
            if let Some(caps) = re.captures(body) {
                if let Ok(s) = caps[1].parse::<u64>() {
                    return Some(s);
                }
            }
        }

        // 模式 3: "quota will reset in X seconds"
        if let Ok(re) = Regex::new(r"(?i)quota will reset in (\d+) second") {
            if let Some(caps) = re.captures(body) {
                if let Ok(s) = caps[1].parse::<u64>() {
                    return Some(s);
                }
            }
        }

        // 模式 4: OpenAI 风格的 "Retry after (\d+) seconds"
        if let Ok(re) = Regex::new(r"(?i)retry after (\d+) second") {
            if let Some(caps) = re.captures(body) {
                if let Ok(s) = caps[1].parse::<u64>() {
                    return Some(s);
                }
            }
        }

        // 模式 5: 括号形式 "(wait (\d+)s)"
        if let Ok(re) = Regex::new(r"\(wait (\d+)s\)") {
            if let Some(caps) = re.captures(body) {
                if let Ok(s) = caps[1].parse::<u64>() {
                    return Some(s);
                }
            }
        }

        None
    }

    /// 获取账号的限流信息
    pub fn get(&self, account_id: &str) -> Option<RateLimitInfo> {
        self.limits.get(account_id).map(|r| r.clone())
    }

    /// 检查账号是否仍在限流中
    pub fn is_rate_limited(&self, account_id: &str, model: Option<&str>) -> bool {
        // 1. 检查账号级别锁定
        if let Some(info) = self.limits.get(account_id) {
            if info.reset_time > SystemTime::now() {
                return true;
            }
        }

        // 2. 检查模型级别锁定
        if let Some(m) = model {
            let key = format!("{}:{}", account_id, m);
            if let Some(info) = self.limits.get(&key) {
                if info.reset_time > SystemTime::now() {
                    return true;
                }
            }
        }

        false
    }

    /// 获取距离限流重置还有多少秒
    pub fn get_reset_seconds(&self, account_id: &str, model: Option<&str>) -> Option<u64> {
        let wait = self.get_remaining_wait(account_id, model);
        if wait > 0 {
            Some(wait)
        } else {
            None
        }
    }

    /// 清除过期的限流记录
    #[allow(dead_code)]
    pub fn cleanup_expired(&self) -> usize {
        let now = SystemTime::now();
        let mut count = 0;

        self.limits.retain(|_k, v| {
            if v.reset_time <= now {
                count += 1;
                false
            } else {
                true
            }
        });

        if count > 0 {
            tracing::debug!("清除了 {} 个过期的限流记录", count);
        }

        count
    }

    /// 清除指定账号的限流记录
    #[allow(dead_code)]
    pub fn clear(&self, account_id: &str) -> bool {
        self.limits.remove(account_id).is_some()
    }

    /// 清除所有限流记录
    #[allow(dead_code)]
    pub fn clear_all(&self) {
        let count = self.limits.len();
        self.limits.clear();
        tracing::debug!("清除了所有 {} 条限流记录", count);
    }
}

impl Default for RateLimitTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_retry_time_minutes_seconds() {
        let tracker = RateLimitTracker::new();
        let body = "Rate limit exceeded. Try again in 2m 30s";
        let time = tracker.parse_retry_time_from_body(body);
        assert_eq!(time, Some(150));
    }

    #[test]
    fn test_parse_google_json_delay() {
        let tracker = RateLimitTracker::new();
        let body = r#"{
            "error": {
                "details": [
                    { "metadata": { "quotaResetDelay": "42s" } }
                ]
            }
        }"#;
        let time = tracker.parse_retry_time_from_body(body);
        assert_eq!(time, Some(42));
    }

    #[test]
    fn test_parse_retry_after_ignore_case() {
        let tracker = RateLimitTracker::new();
        let body = "Quota limit hit. Retry After 99 Seconds";
        let time = tracker.parse_retry_time_from_body(body);
        assert_eq!(time, Some(99));
    }

    #[test]
    fn test_get_remaining_wait() {
        let tracker = RateLimitTracker::new();
        tracker.parse_from_error("acc1", 429, Some("30"), "", None);
        let wait = tracker.get_remaining_wait("acc1", None);
        assert!(wait > 25 && wait <= 30);
    }

    #[test]
    fn test_safety_buffer() {
        let tracker = RateLimitTracker::new();
        // 如果 API 返回 1s，我们强制设为 2s
        tracker.parse_from_error("acc1", 429, Some("1"), "", None);
        let wait = tracker.get_remaining_wait("acc1", None);
        // 由于时间精度，允许 1-2 秒范围
        assert!(wait >= 1 && wait <= 2);
    }
}
