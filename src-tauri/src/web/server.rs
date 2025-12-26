//! Web 服务器

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

use super::routes;
use crate::core::models::Account;
use crate::core::services::ProxyServiceManager;
use crate::core::traits::{DefaultStorageConfig, NoopEmitter, StorageConfig};

/// 待处理的 OAuth 授权
#[derive(Debug, Clone)]
pub struct PendingOAuth {
    pub redirect_uri: String,
    pub created_at: i64,
}

/// OAuth 授权结果
#[derive(Debug, Clone)]
pub enum OAuthResult {
    Pending,
    Success(Account),
    Error(String),
}

/// Web 应用状态
pub struct WebAppState {
    pub storage: DefaultStorageConfig,
    pub emitter: NoopEmitter,
    pub proxy_manager: ProxyServiceManager,
    pub oauth_pending: RwLock<Option<PendingOAuth>>,
    pub oauth_result: RwLock<OAuthResult>,
    pub log_store: crate::proxy::LogStore,
}

impl WebAppState {
    pub fn new() -> Result<Self, String> {
        let storage = DefaultStorageConfig::new()?;
        let proxy_manager = ProxyServiceManager::new(&storage);
        Ok(Self {
            storage,
            emitter: NoopEmitter,
            proxy_manager,
            oauth_pending: RwLock::new(None),
            oauth_result: RwLock::new(OAuthResult::Pending),
            log_store: crate::proxy::LogStore::default(),
        })
    }

    /// 从指定数据目录创建
    pub fn with_data_dir(data_dir: std::path::PathBuf) -> Result<Self, String> {
        let storage = DefaultStorageConfig::with_path(data_dir)?;
        let proxy_manager = ProxyServiceManager::new(&storage);
        Ok(Self {
            storage,
            emitter: NoopEmitter,
            proxy_manager,
            oauth_pending: RwLock::new(None),
            oauth_result: RwLock::new(OAuthResult::Pending),
            log_store: crate::proxy::LogStore::default(),
        })
    }
}

/// Web 服务器
pub struct WebServer {
    port: u16,
    state: Arc<WebAppState>,
}

impl WebServer {
    /// 创建新的 Web 服务器
    pub fn new(port: u16) -> Result<Self, String> {
        let state = Arc::new(WebAppState::new()?);
        Ok(Self { port, state })
    }

    /// 从指定数据目录创建
    pub fn with_data_dir(port: u16, data_dir: std::path::PathBuf) -> Result<Self, String> {
        let state = Arc::new(WebAppState::with_data_dir(data_dir)?);
        Ok(Self { port, state })
    }

    /// 启动服务器
    pub async fn run(self) -> Result<(), String> {
        // 构建路由
        let app = routes::build_routes(self.state.clone())
            // 添加 CORS 支持
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
            );

        // 绑定地址
        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));

        println!("🚀 Web 服务器启动在 http://{}", addr);
        println!("📁 数据目录: {:?}", self.state.storage.data_dir());

        // 启动服务器
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| format!("绑定端口失败: {}", e))?;

        axum::serve(listener, app)
            .await
            .map_err(|e| format!("服务器错误: {}", e))?;

        Ok(())
    }
}
