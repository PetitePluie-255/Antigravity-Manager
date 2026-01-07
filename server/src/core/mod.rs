//! 核心模块
//! 包含不依赖 Tauri 的业务逻辑

pub mod db;
pub mod models;
pub mod services;
pub mod storage;
pub mod traits;

// 重导出常用类型
pub use traits::{
    AppContext, DefaultStorageConfig, EventEmitter, NoopEmitter, StorageConfig, WebAppContext,
};
