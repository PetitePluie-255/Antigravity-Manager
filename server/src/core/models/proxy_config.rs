//! 代理服务配置

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 反代服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProxyConfig {
    /// 是否启用反代服务
    pub enabled: bool,

    /// 监听端口
    pub port: u16,

    /// API 密钥
    pub api_key: String,

    /// 是否自动启动
    pub auto_start: bool,

    /// Anthropic 模型映射表 (key: Claude模型名, value: Gemini模型名)
    #[serde(default)]
    pub anthropic_mapping: HashMap<String, String>,

    /// OpenAI 模型映射表 (key: OpenAI模型组, value: Gemini模型名)
    #[serde(default)]
    pub openai_mapping: HashMap<String, String>,

    /// 自定义精确模型映射表 (key: 原始模型名, value: 目标模型名)
    #[serde(default)]
    pub custom_mapping: HashMap<String, String>,

    /// API 请求超时时间(秒)
    #[serde(default = "default_request_timeout")]
    pub request_timeout: u64,

    /// 上游代理配置
    #[serde(default)]
    pub upstream_proxy: UpstreamProxyConfig,

    /// 是否允许局域网访问
    #[serde(default)]
    pub allow_lan_access: bool,

    /// z.ai 配置
    #[serde(default)]
    pub zai: ZaiConfig,

    /// 调度模式配置
    #[serde(default)]
    pub scheduling: SchedulingConfig,

    /// 安全模式
    #[serde(default)]
    pub auth_mode: String,
}

/// z.ai 服务配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZaiConfig {
    /// 是否启用 z.ai
    #[serde(default)]
    pub enabled: bool,
    /// API Key
    #[serde(default)]
    pub api_key: String,
    /// 调度模式: off, exclusive, pooled, fallback
    #[serde(default)]
    pub dispatch_mode: String,
    /// MCP 功能配置
    #[serde(default)]
    pub mcp: ZaiMcpConfig,
}

/// z.ai MCP 配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZaiMcpConfig {
    /// 是否启用 MCP
    #[serde(default)]
    pub enabled: bool,
    /// 网页搜索
    #[serde(default)]
    pub web_search_enabled: bool,
    /// 网页阅读
    #[serde(default)]
    pub web_reader_enabled: bool,
    /// 视觉功能
    #[serde(default)]
    pub vision_enabled: bool,
}

/// 调度配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SchedulingConfig {
    /// 调度模式: cache_first, balance, performance_first
    #[serde(default = "default_scheduling_mode")]
    pub mode: String,
    /// 最大等待时间 (秒)
    #[serde(default = "default_max_wait_seconds")]
    pub max_wait_seconds: u64,
}

fn default_scheduling_mode() -> String {
    "balance".to_string()
}

fn default_max_wait_seconds() -> u64 {
    60
}

/// 上游代理配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpstreamProxyConfig {
    /// 是否启用
    pub enabled: bool,
    /// 代理地址 (http://, https://, socks5://)
    pub url: String,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 8045,
            api_key: format!("sk-{}", uuid::Uuid::new_v4().simple()),
            auto_start: false,
            anthropic_mapping: HashMap::new(),
            openai_mapping: HashMap::new(),
            custom_mapping: HashMap::new(),
            request_timeout: default_request_timeout(),
            upstream_proxy: UpstreamProxyConfig::default(),
            allow_lan_access: false,
            zai: ZaiConfig::default(),
            scheduling: SchedulingConfig::default(),
            auth_mode: "auto".to_string(),
        }
    }
}

fn default_request_timeout() -> u64 {
    120 // 默认 120 秒
}
