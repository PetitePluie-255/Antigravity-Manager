//! 核心 trait 定义
//! 用于解耦业务逻辑与 Tauri/Web 运行时

use serde::Serialize;
use std::path::PathBuf;

/// 事件发射器 trait
/// Tauri 模式下使用 AppHandle.emit()
/// Web 模式下可选择 WebSocket 推送或忽略
pub trait EventEmitter: Send + Sync {
    fn emit<T: Serialize + Clone>(&self, event: &str, payload: T);
}

/// 空事件发射器 (Web 模式使用)
pub struct NoopEmitter;

impl EventEmitter for NoopEmitter {
    fn emit<T: Serialize + Clone>(&self, _event: &str, _payload: T) {
        // Web 模式下不发射事件
    }
}

/// 存储配置 trait
/// 抽象数据目录和文件系统操作
pub trait StorageConfig: Send + Sync {
    /// 获取数据目录路径
    fn data_dir(&self) -> PathBuf;
    
    /// 获取账户目录路径
    fn accounts_dir(&self) -> PathBuf {
        self.data_dir().join("accounts")
    }
    
    /// 获取配置文件路径
    fn config_path(&self) -> PathBuf {
        self.data_dir().join("config.json")
    }
    
    /// 获取账户索引文件路径
    fn accounts_index_path(&self) -> PathBuf {
        self.data_dir().join("accounts.json")
    }
}

/// 默认存储配置 (使用 ~/.antigravity_tools/)
pub struct DefaultStorageConfig {
    data_dir: PathBuf,
}

impl DefaultStorageConfig {
    pub fn new() -> Result<Self, String> {
        let home = dirs::home_dir()
            .ok_or_else(|| "无法获取用户主目录".to_string())?;
        let data_dir = home.join(".antigravity_tools");
        
        // 确保目录存在
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| format!("创建数据目录失败: {}", e))?;
        std::fs::create_dir_all(data_dir.join("accounts"))
            .map_err(|e| format!("创建账户目录失败: {}", e))?;
        
        Ok(Self { data_dir })
    }
    
    /// 从指定路径创建
    pub fn with_path(data_dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| format!("创建数据目录失败: {}", e))?;
        std::fs::create_dir_all(data_dir.join("accounts"))
            .map_err(|e| format!("创建账户目录失败: {}", e))?;
        
        Ok(Self { data_dir })
    }
}

impl Default for DefaultStorageConfig {
    fn default() -> Self {
        Self::new().expect("无法初始化默认存储配置")
    }
}

impl StorageConfig for DefaultStorageConfig {
    fn data_dir(&self) -> PathBuf {
        self.data_dir.clone()
    }
}

/// 应用上下文
/// 包含运行时所需的所有依赖
pub struct AppContext<E: EventEmitter, S: StorageConfig> {
    pub emitter: E,
    pub storage: S,
}

impl<E: EventEmitter, S: StorageConfig> AppContext<E, S> {
    pub fn new(emitter: E, storage: S) -> Self {
        Self { emitter, storage }
    }
}

/// Web 模式的默认上下文
pub type WebAppContext = AppContext<NoopEmitter, DefaultStorageConfig>;

impl WebAppContext {
    pub fn new_web() -> Result<Self, String> {
        Ok(Self {
            emitter: NoopEmitter,
            storage: DefaultStorageConfig::new()?,
        })
    }
}
