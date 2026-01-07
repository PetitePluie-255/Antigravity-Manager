//! Web 服务器

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

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
    pub db_pool: sqlx::SqlitePool,
}

impl WebAppState {
    pub async fn new() -> Result<Self, String> {
        let storage = DefaultStorageConfig::new()?;
        let db_pool = crate::core::db::init_db(&storage.data_dir()).await?;
        let proxy_manager = ProxyServiceManager::new(&storage);
        Ok(Self {
            storage,
            emitter: NoopEmitter,
            proxy_manager,
            oauth_pending: RwLock::new(None),
            oauth_result: RwLock::new(OAuthResult::Pending),
            log_store: crate::proxy::LogStore::new(db_pool.clone()),
            db_pool,
        })
    }

    /// 从指定数据目录创建
    pub async fn with_data_dir(data_dir: std::path::PathBuf) -> Result<Self, String> {
        let storage = DefaultStorageConfig::with_path(data_dir)?;
        let db_pool = crate::core::db::init_db(&storage.data_dir()).await?;
        let proxy_manager = ProxyServiceManager::new(&storage);
        Ok(Self {
            storage,
            emitter: NoopEmitter,
            proxy_manager,
            oauth_pending: RwLock::new(None),
            oauth_result: RwLock::new(OAuthResult::Pending),
            log_store: crate::proxy::LogStore::new(db_pool.clone()),
            db_pool,
        })
    }
}

/// Web 服务器
pub struct WebServer {
    port: u16,
    state: Arc<WebAppState>,
    static_dir: Option<PathBuf>,
}

impl WebServer {
    /// 创建新的 Web 服务器
    pub async fn new(port: u16) -> Result<Self, String> {
        let state = Arc::new(WebAppState::new().await?);
        let static_dir = std::env::var("STATIC_DIR")
            .or_else(|_| std::env::var("STATIC_PATH"))
            .ok()
            .map(PathBuf::from);
        Ok(Self {
            port,
            state,
            static_dir,
        })
    }

    /// 从指定数据目录创建
    pub async fn with_data_dir(port: u16, data_dir: std::path::PathBuf) -> Result<Self, String> {
        let state = Arc::new(WebAppState::with_data_dir(data_dir).await?);
        let static_dir = std::env::var("STATIC_DIR")
            .or_else(|_| std::env::var("STATIC_PATH"))
            .ok()
            .map(PathBuf::from);
        Ok(Self {
            port,
            state,
            static_dir,
        })
    }

    /// 启动服务器
    pub async fn run(self) -> Result<(), String> {
        use axum::Router;
        use tower_http::services::ServeFile;

        // 构建 API 路由
        let api_routes = routes::build_routes(self.state.clone());

        // 构建完整路由
        let app = if let Some(static_path) = &self.static_dir {
            if static_path.exists() {
                println!("📦 静态文件目录: {:?}", static_path);
                let index_path = static_path.join("index.html");
                // API 路由优先，静态文件作为 fallback
                // 对于 SPA，未匹配的路径返回 index.html
                Router::new().merge(api_routes).fallback_service(
                    ServeDir::new(static_path)
                        .append_index_html_on_directories(true)
                        .fallback(ServeFile::new(index_path)),
                )
            } else {
                println!("⚠️  静态文件目录不存在: {:?}", static_path);
                api_routes
            }
        } else {
            api_routes
        };

        // 添加 CORS 支持
        let app = app.layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

        // 绑定地址 - 在 Docker 中需要绑定 0.0.0.0
        let bind_addr = std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string());
        let addr: SocketAddr = format!("{}:{}", bind_addr, self.port)
            .parse()
            .map_err(|e| format!("无效的地址: {}", e))?;

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
