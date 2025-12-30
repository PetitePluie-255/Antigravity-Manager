//! 核心服务层
//! 业务逻辑实现，不依赖 Tauri

pub mod account;
#[cfg(feature = "web-server")]
pub mod database;
pub mod oauth;
// pub mod proxy; // Removed
pub mod quota;

pub use account::AccountService;
#[cfg(feature = "web-server")]
pub use database::{DatabaseImporter, ImportConfig, ImportedAccount};
pub use oauth::refresh_access_token;
// pub use proxy::ProxyServiceManager; // Removed
// pub use proxy::ProxyStatus; // Removed
pub use quota::QuotaService;
