//! 配置存储服务

use std::fs;
use crate::core::models::AppConfig;
use crate::core::traits::StorageConfig;

/// 配置存储服务
pub struct ConfigStorage;

impl ConfigStorage {
    /// 加载应用配置
    pub fn load<S: StorageConfig>(storage: &S) -> Result<AppConfig, String> {
        let config_path = storage.config_path();
        
        if !config_path.exists() {
            // 如果配置文件不存在，返回默认配置
            let default_config = AppConfig::default();
            // 保存默认配置
            Self::save(storage, &default_config)?;
            return Ok(default_config);
        }
        
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("读取配置文件失败: {}", e))?;
        
        let config: AppConfig = serde_json::from_str(&content)
            .map_err(|e| format!("解析配置文件失败: {}", e))?;
        
        Ok(config)
    }
    
    /// 保存应用配置
    pub fn save<S: StorageConfig>(storage: &S, config: &AppConfig) -> Result<(), String> {
        let config_path = storage.config_path();
        
        // 确保目录存在
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("创建配置目录失败: {}", e))?;
        }
        
        let content = serde_json::to_string_pretty(config)
            .map_err(|e| format!("序列化配置失败: {}", e))?;
        
        // 原子写入：先写入临时文件，再重命名
        let temp_path = config_path.with_extension("json.tmp");
        fs::write(&temp_path, &content)
            .map_err(|e| format!("写入临时配置文件失败: {}", e))?;
        
        fs::rename(&temp_path, &config_path)
            .map_err(|e| format!("重命名配置文件失败: {}", e))?;
        
        Ok(())
    }
}
