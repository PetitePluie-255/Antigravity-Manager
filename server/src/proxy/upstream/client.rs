// 上游客户端实现
// 基于高性能通讯接口封装

use reqwest::{header, Client, Response};
use serde_json::Value;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

// 生产环境端点
const V1_INTERNAL_BASE_URL: &str = "https://cloudcode-pa.googleapis.com/v1internal";

pub struct UpstreamClient {
    http_client: RwLock<Client>,
}

impl UpstreamClient {
    pub fn new(proxy_config: Option<crate::proxy::config::UpstreamProxyConfig>) -> Self {
        let client = Self::build_client(proxy_config);
        Self {
            http_client: RwLock::new(client),
        }
    }

    fn build_client(proxy_config: Option<crate::proxy::config::UpstreamProxyConfig>) -> Client {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(600))
            .user_agent("antigravity/1.11.9 windows/amd64");

        if let Some(config) = proxy_config {
            if config.enabled && !config.url.is_empty() {
                if let Ok(proxy) = reqwest::Proxy::all(&config.url) {
                    builder = builder.proxy(proxy);
                    tracing::info!("UpstreamClient enabled proxy: {}", config.url);
                }
            }
        }

        builder.build().expect("Failed to create HTTP client")
    }

    pub async fn update_config(&self, proxy_config: crate::proxy::config::UpstreamProxyConfig) {
        let new_client = Self::build_client(Some(proxy_config));
        *self.http_client.write().await = new_client;
        tracing::info!("UpstreamClient configuration updated");
    }

    /// 构建 v1internal URL
    ///
    /// 构建 API 请求地址
    fn build_url(method: &str, query_string: Option<&str>) -> String {
        if let Some(qs) = query_string {
            format!("{}:{}?{}", V1_INTERNAL_BASE_URL, method, qs)
        } else {
            format!("{}:{}", V1_INTERNAL_BASE_URL, method)
        }
    }

    /// 调用 v1internal API（基础方法）
    ///
    /// 发起基础网络请求
    pub async fn call_v1_internal(
        &self,
        method: &str,
        access_token: &str,
        body: Value,
        query_string: Option<&str>,
    ) -> Result<Response, String> {
        let url = Self::build_url(method, query_string);

        // 构建 Headers
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", access_token))
                .map_err(|e| e.to_string())?,
        );
        // 设置自定义 User-Agent
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("antigravity/1.11.9 windows/amd64"),
        );

        // 记录请求详情以便调试 404
        let client = self.http_client.read().await;
        let response = client
            .post(&url)
            .headers(headers) // Apply all headers at once
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        Ok(response)
    }

    /// 调用 v1internal API（带 429 重试,支持闭包）
    ///
    /// 带容错和重试的核心请求逻辑
    ///
    /// # Arguments
    /// * `method` - API method (e.g., "generateContent")
    /// * `query_string` - Optional query string (e.g., "?alt=sse")
    /// * `get_credentials` - 闭包，获取凭证（支持账号轮换）
    /// * `build_body` - 闭包，接收 project_id 构建请求体
    /// * `max_attempts` - 最大重试次数
    ///
    /// # Returns
    /// HTTP Response
    pub async fn call_v1_internal_with_retry<F, B>(
        &self,
        method: &str,
        query_string: Option<&str>,
        mut get_credentials: F,
        build_body: B,
        max_attempts: usize,
    ) -> Result<Response, String>
    where
        F: FnMut() -> Result<(String, String), String>, // () -> (access_token, project_id)
        B: Fn(&str) -> Result<Value, String>,           // project_id -> body
    {
        let mut last_error = String::new();

        for attempt in 0..max_attempts {
            // 1. 获取凭证（可能轮换账号）
            let (access_token, project_id) = match get_credentials() {
                Ok(creds) => creds,
                Err(e) => {
                    last_error = format!("Failed to get credentials: {}", e);
                    continue;
                }
            };

            // 2. 调用闭包构建请求体
            let body = match build_body(&project_id) {
                Ok(b) => b,
                Err(e) => {
                    return Err(format!("Failed to build request body: {}", e));
                }
            };

            // 3. 发送请求
            let response = match self
                .call_v1_internal(method, &access_token, body, query_string)
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    last_error = format!("Network error: {}", e);
                    tracing::warn!(
                        "Network error on attempt {}/{}: {}",
                        attempt + 1,
                        max_attempts,
                        e
                    );
                    continue;
                }
            };

            let status = response.status();

            // 4. 成功响应
            if status.is_success() {
                return Ok(response);
            }

            // 5. 处理 429 重试逻辑
            if status.as_u16() == 429 {
                // 读取错误详情
                let error_text = response.text().await.unwrap_or_else(|_| String::from("{}"));

                last_error = error_text.clone();

                // 解析 retry delay
                if let Some(delay_ms) = self.parse_retry_delay(&error_text) {
                    tracing::info!(
                        "429 error, attempt {}/{}, delay: {}ms",
                        attempt + 1,
                        max_attempts,
                        delay_ms
                    );

                    // 短延迟（<= 5000ms）: 等待后重试当前账号
                    // 短延迟重试处理
                    if delay_ms <= 5000 {
                        let actual_delay = delay_ms + 200; // 加 200ms buffer
                        tracing::info!("Short delay, waiting {}ms on same account", actual_delay);
                        sleep(Duration::from_millis(actual_delay)).await;
                        // 不轮换账号，继续循环会重新调用 get_credentials
                        continue;
                    } else {
                        // 长延迟: 立即轮换账号
                        tracing::info!("Long delay, rotating to next account");
                        continue; // get_credentials 会自动轮换
                    }
                } else {
                    // 没有 retry delay，默认轮换
                    tracing::warn!("429 without retry delay, rotating account");
                    continue;
                }
            }

            // 6. 其他 HTTP 错误
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| format!("HTTP {}", status));

            last_error = format!("HTTP {}: {}", status, error_text);

            // 对于 404/403/401 等，也可以尝试轮换账号
            // 错误重连与轮换策略
            if status.as_u16() == 404 || status.as_u16() == 403 || status.as_u16() == 401 {
                tracing::warn!(
                    "HTTP {} on attempt {}/{}, rotating account",
                    status,
                    attempt + 1,
                    max_attempts
                );
                continue;
            }

            // 其他错误直接返回
            return Err(last_error);
        }

        Err(format!(
            "Upstream call exhausted without a response after {} attempts. Last error: {}",
            max_attempts, last_error
        ))
    }

    /// 解析重试延迟
    ///
    /// 从错误详情中解析重试等待时间
    fn parse_retry_delay(&self, error_text: &str) -> Option<u64> {
        // 尝试解析 JSON
        if let Ok(json) = serde_json::from_str::<Value>(error_text) {
            // 检查 error.retryInfo.retryDelay
            if let Some(retry_info) = json.get("error").and_then(|e| e.get("retryInfo")) {
                if let Some(delay_str) = retry_info.get("retryDelay").and_then(|d| d.as_str()) {
                    return self.parse_duration_ms(delay_str);
                }
            }

            // 检查 error.quotaResetDelay
            if let Some(delay_str) = json
                .get("error")
                .and_then(|e| e.get("quotaResetDelay"))
                .and_then(|d| d.as_str())
            {
                return self.parse_duration_ms(delay_str);
            }
        }

        None
    }

    /// 解析 Duration 字符串为毫秒
    ///
    /// 解析时间间隔字符串
    /// 支持格式: "1.5s", "200ms", "1h16m0.667s"
    fn parse_duration_ms(&self, duration_str: &str) -> Option<u64> {
        // 简化版本，支持主要格式
        // 完整实现需要 regex，这里先做简单的
        if duration_str.ends_with("ms") {
            duration_str.trim_end_matches("ms").parse::<u64>().ok()
        } else if duration_str.ends_with('s') {
            duration_str
                .trim_end_matches('s')
                .parse::<f64>()
                .ok()
                .map(|x| (x * 1000.0) as u64)
        } else {
            None
        }
    }

    /// 获取可用模型列表
    ///
    /// 获取远端模型列表
    pub async fn fetch_available_models(&self, access_token: &str) -> Result<Value, String> {
        let url = Self::build_url("fetchAvailableModels", None);

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", access_token))
                .map_err(|e| e.to_string())?,
        );
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("antigravity/1.11.9 windows/amd64"),
        );

        let client = self.http_client.read().await;
        let response = client
            .post(&url)
            .headers(headers)
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Upstream error: {}", response.status()));
        }

        let json: Value = response
            .json()
            .await
            .map_err(|e| format!("Parse json failed: {}", e))?;
        Ok(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url() {
        let url1 = UpstreamClient::build_url("generateContent", None);
        assert_eq!(
            url1,
            "https://cloudcode-pa.googleapis.com/v1internal:generateContent"
        );

        let url2 = UpstreamClient::build_url("streamGenerateContent", Some("alt=sse"));
        assert_eq!(
            url2,
            "https://cloudcode-pa.googleapis.com/v1internal:streamGenerateContent?alt=sse"
        );
    }

    #[test]
    fn test_parse_duration() {
        let client = UpstreamClient::new(None);

        assert_eq!(client.parse_duration_ms("1500ms"), Some(1500));
        assert_eq!(client.parse_duration_ms("1.5s"), Some(1500));
        assert_eq!(client.parse_duration_ms("2s"), Some(2000));
    }
}
