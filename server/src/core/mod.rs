//! 核心模块
//! 包含不依赖 Tauri 的业务逻辑

pub mod traits;
pub mod models;
pub mod services;
pub mod storage;

// 重导出常用类型
pub use traits::{EventEmitter, StorageConfig, AppContext, NoopEmitter, DefaultStorageConfig, WebAppContext};
