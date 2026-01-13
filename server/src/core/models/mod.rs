//! 核心数据模型
//! 不依赖 Tauri 的数据结构定义

mod account;
mod config;
mod proxy_config;
mod quota;
mod token;

pub use account::{Account, AccountIndex, AccountSummary, DeviceProfile, DeviceProfileVersion};
pub use config::AppConfig;
pub use proxy_config::{ProxyConfig, UpstreamProxyConfig};
pub use quota::{ModelQuota, QuotaData};
pub use token::TokenData;
