//! 应用配置模型

use super::ProxyConfig;
use serde::{Deserialize, Serialize};

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub language: String,
    pub theme: String,
    pub auto_refresh: bool,
    pub refresh_interval: i32, // 分钟
    pub auto_sync: bool,
    pub sync_interval: i32, // 分钟
    pub default_export_path: Option<String>,
    #[serde(default)]
    pub antigravity_executable: Option<String>, // 手动指定的反重力程序路径
    #[serde(default)]
    pub proxy: ProxyConfig,
}

impl AppConfig {
    pub fn new() -> Self {
        Self {
            language: "zh-CN".to_string(),
            theme: "system".to_string(),
            auto_refresh: false,
            refresh_interval: 15,
            auto_sync: false,
            sync_interval: 5,
            default_export_path: None,
            antigravity_executable: None,
            proxy: ProxyConfig::default(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::new()
    }
}
