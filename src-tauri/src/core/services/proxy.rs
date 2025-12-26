//! 代理服务管理器
//! 负责启动、停止 API 代理服务器

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::models::ProxyConfig;
use crate::core::traits::StorageConfig;
use crate::proxy::{AxumServer, TokenManager};

/// 代理服务状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyStatus {
    pub running: bool,
    pub port: u16,
    pub base_url: String,
    pub active_accounts: usize,
}

impl Default for ProxyStatus {
    fn default() -> Self {
        Self {
            running: false,
            port: 0,
            base_url: String::new(),
            active_accounts: 0,
        }
    }
}

/// 代理服务实例
struct ProxyServiceInstance {
    pub config: ProxyConfig,
    pub token_manager: Arc<TokenManager>,
    pub server: AxumServer,
    pub server_handle: tokio::task::JoinHandle<()>,
}

/// 代理服务管理器
pub struct ProxyServiceManager {
    instance: RwLock<Option<ProxyServiceInstance>>,
    data_dir: PathBuf,
}

impl ProxyServiceManager {
    pub fn new<S: StorageConfig>(storage: &S) -> Self {
        Self {
            instance: RwLock::new(None),
            data_dir: storage.data_dir(),
        }
    }

    /// 启动代理服务
    pub async fn start(&self, config: ProxyConfig) -> Result<ProxyStatus, String> {
        let mut instance_lock = self.instance.write().await;

        if instance_lock.is_some() {
            return Err("服务已在运行中".to_string());
        }

        // 初始化 Token 管理器 (使用完整版)
        let token_manager = Arc::new(TokenManager::new(self.data_dir.clone()));
        let active_accounts = token_manager.load_accounts().await?;

        if active_accounts == 0 {
            return Err("没有可用账号，请先添加账号".to_string());
        }

        // 转换 UpstreamProxyConfig 类型
        let upstream = crate::proxy::config::UpstreamProxyConfig {
            enabled: config.upstream_proxy.enabled,
            url: config.upstream_proxy.url.clone(),
        };

        // 使用完整的 AxumServer 启动代理
        let (server, handle) = AxumServer::start(
            config.port,
            token_manager.clone(),
            config.anthropic_mapping.clone(),
            config.openai_mapping.clone(),
            config.custom_mapping.clone(),
            config.request_timeout,
            upstream,
        )
        .await?;

        let active = token_manager.len();

        // 创建服务实例
        let instance = ProxyServiceInstance {
            config: config.clone(),
            token_manager,
            server,
            server_handle: handle,
        };

        *instance_lock = Some(instance);

        Ok(ProxyStatus {
            running: true,
            port: config.port,
            base_url: format!("http://127.0.0.1:{}", config.port),
            active_accounts: active,
        })
    }

    /// 停止代理服务
    pub async fn stop(&self) -> Result<(), String> {
        let mut instance_lock = self.instance.write().await;

        if instance_lock.is_none() {
            return Err("服务未运行".to_string());
        }

        if let Some(instance) = instance_lock.take() {
            // 停止服务器
            instance.server.stop();
            // 等待服务器任务完成
            let _ = instance.server_handle.await;
        }

        Ok(())
    }

    /// 获取代理状态
    pub async fn status(&self) -> ProxyStatus {
        let instance_lock = self.instance.read().await;

        match instance_lock.as_ref() {
            Some(instance) => ProxyStatus {
                running: true,
                port: instance.config.port,
                base_url: format!("http://127.0.0.1:{}", instance.config.port),
                active_accounts: instance.token_manager.len(),
            },
            None => ProxyStatus::default(),
        }
    }
}
