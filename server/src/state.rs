use crate::core::models::Account;
use crate::core::traits::{DefaultStorageConfig, EventEmitter, StorageConfig};
use crate::proxy::TokenManager;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64};
use std::sync::Arc;
use tokio::sync::RwLock;

// Mock Emitter for Web Server
pub struct NoopEmitter;
impl EventEmitter for NoopEmitter {
    fn emit<T: Serialize + Clone>(&self, _event: &str, _payload: T) {}
}

/// OAuth 结果状态
#[derive(Clone, Debug)]
pub enum OAuthResult {
    Pending,
    Success(Account),
    Error(String),
}

/// 待处理的 OAuth 请求
#[derive(Clone, Debug)]
pub struct PendingOAuth {
    pub redirect_uri: String,
    pub created_at: i64,
}

/// Web 应用状态
pub struct AppState {
    pub storage: DefaultStorageConfig,
    pub emitter: NoopEmitter,
    // pub proxy_manager: ProxyServiceManager, // Removed
    pub oauth_pending: RwLock<Option<PendingOAuth>>,
    pub oauth_result: RwLock<OAuthResult>,
    pub log_store: Arc<crate::proxy::LogStore>,

    // Proxy Fields
    pub token_manager: Arc<TokenManager>,
    pub anthropic_mapping: Arc<RwLock<std::collections::HashMap<String, String>>>,
    pub openai_mapping: Arc<RwLock<std::collections::HashMap<String, String>>>,
    pub custom_mapping: Arc<RwLock<std::collections::HashMap<String, String>>>,
    pub request_timeout: Arc<AtomicU64>,
    pub thought_signature_map: Arc<tokio::sync::Mutex<std::collections::HashMap<String, String>>>,
    pub upstream_proxy: Arc<RwLock<crate::proxy::config::UpstreamProxyConfig>>,
    pub upstream: Arc<crate::proxy::upstream::client::UpstreamClient>,
    pub proxy_enabled: Arc<AtomicBool>,
    pub proxy_port: Arc<AtomicU16>, // Persisted config port, for status display

    // z.ai integration fields
    pub zai: Arc<RwLock<crate::proxy::ZaiConfig>>,
    pub provider_rr: Arc<std::sync::atomic::AtomicUsize>, // Round-robin counter for provider selection
    pub zai_vision_mcp: crate::proxy::zai_vision_mcp::ZaiVisionMcpState,
    pub monitor: Arc<crate::proxy::monitor::ProxyMonitor>,
    pub db_pool: sqlx::SqlitePool,
}

impl AppState {
    pub async fn new() -> Result<Self, String> {
        let storage = DefaultStorageConfig::new()?;
        // let proxy_manager = ProxyServiceManager::new(&storage);

        let db_pool = crate::core::db::init_db(&storage.data_dir()).await?;

        let token_manager = Arc::new(TokenManager::new(storage.data_dir(), db_pool.clone()));
        // Initialize other proxy fields with defaults
        let upstream_proxy = crate::proxy::config::UpstreamProxyConfig::default();

        Ok(Self {
            storage,
            emitter: NoopEmitter,
            // proxy_manager,
            oauth_pending: RwLock::new(None),
            oauth_result: RwLock::new(OAuthResult::Pending),
            log_store: Arc::new(crate::proxy::LogStore::new(db_pool.clone())),

            token_manager,
            anthropic_mapping: Arc::new(RwLock::new(std::collections::HashMap::new())),
            openai_mapping: Arc::new(RwLock::new(std::collections::HashMap::new())),
            custom_mapping: Arc::new(RwLock::new(std::collections::HashMap::new())),
            request_timeout: Arc::new(AtomicU64::new(120)),
            thought_signature_map: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            upstream_proxy: Arc::new(RwLock::new(upstream_proxy.clone())),
            upstream: Arc::new(crate::proxy::upstream::client::UpstreamClient::new(Some(
                upstream_proxy,
            ))),
            proxy_enabled: Arc::new(AtomicBool::new(true)), // Always enabled in integrated mode
            proxy_port: Arc::new(AtomicU16::new(0)),
            zai: Arc::new(RwLock::new(crate::proxy::ZaiConfig::default())),
            provider_rr: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            zai_vision_mcp: crate::proxy::zai_vision_mcp::ZaiVisionMcpState::new(),
            monitor: Arc::new(crate::proxy::monitor::ProxyMonitor::default()),
            db_pool,
        })
    }

    pub async fn with_data_dir(data_dir: std::path::PathBuf) -> Result<Self, String> {
        let storage = DefaultStorageConfig::with_path(data_dir)?;

        let db_pool = crate::core::db::init_db(&storage.data_dir()).await?;

        let token_manager = Arc::new(TokenManager::new(storage.data_dir(), db_pool.clone()));

        // Initialize other proxy fields with defaults
        let upstream_proxy = crate::proxy::config::UpstreamProxyConfig::default();

        Ok(Self {
            storage,
            emitter: NoopEmitter,
            // proxy_manager,
            oauth_pending: RwLock::new(None),
            oauth_result: RwLock::new(OAuthResult::Pending),
            log_store: Arc::new(crate::proxy::LogStore::new(db_pool.clone())),

            token_manager,
            anthropic_mapping: Arc::new(RwLock::new(std::collections::HashMap::new())),
            openai_mapping: Arc::new(RwLock::new(std::collections::HashMap::new())),
            custom_mapping: Arc::new(RwLock::new(std::collections::HashMap::new())),
            request_timeout: Arc::new(AtomicU64::new(120)),
            thought_signature_map: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            upstream_proxy: Arc::new(RwLock::new(upstream_proxy.clone())),
            upstream: Arc::new(crate::proxy::upstream::client::UpstreamClient::new(Some(
                upstream_proxy,
            ))),
            proxy_enabled: Arc::new(AtomicBool::new(true)), // Always enabled in integrated mode
            proxy_port: Arc::new(AtomicU16::new(0)),
            zai: Arc::new(RwLock::new(crate::proxy::ZaiConfig::default())),
            provider_rr: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            zai_vision_mcp: crate::proxy::zai_vision_mcp::ZaiVisionMcpState::new(),
            monitor: Arc::new(crate::proxy::monitor::ProxyMonitor::default()),
            db_pool,
        })
    }
}
