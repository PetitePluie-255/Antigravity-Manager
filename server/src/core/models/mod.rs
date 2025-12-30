//! 核心数据模型
//! 不依赖 Tauri 的数据结构定义

mod account;
mod token;
mod quota;
mod config;
mod proxy_config;

pub use account::{Account, AccountIndex, AccountSummary};
pub use token::TokenData;
pub use quota::{QuotaData, ModelQuota};
pub use config::AppConfig;
pub use proxy_config::{ProxyConfig, UpstreamProxyConfig};
