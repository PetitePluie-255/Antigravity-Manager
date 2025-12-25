use serde::{Deserialize, Serialize};
// use std::path::PathBuf;

/// 反代服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub anthropic_mapping: std::collections::HashMap<String, String>,

    /// OpenAI 模型映射表 (key: OpenAI模型组, value: Gemini模型名)
    #[serde(default)]
    pub openai_mapping: std::collections::HashMap<String, String>,

    /// 自定义精确模型映射表 (key: 原始模型名, value: 目标模型名)
    #[serde(default)]
    pub custom_mapping: std::collections::HashMap<String, String>,

    /// API 请求超时时间(秒)
    #[serde(default = "default_request_timeout")]
    pub request_timeout: u64,

    /// 上游代理配置
    #[serde(default)]
    pub upstream_proxy: UpstreamProxyConfig,
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
            anthropic_mapping: std::collections::HashMap::new(),
            openai_mapping: std::collections::HashMap::new(),
            custom_mapping: std::collections::HashMap::new(),
            request_timeout: default_request_timeout(),
            upstream_proxy: UpstreamProxyConfig::default(),
        }
    }
}

fn default_request_timeout() -> u64 {
    120  // 默认 120 秒,原来 60 秒太短
}
