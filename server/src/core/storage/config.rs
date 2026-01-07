//! 配置存储服务
//! 使用 SQLite 数据库持久化配置

use crate::core::models::AppConfig;
use crate::core::traits::StorageConfig;
use sqlx::SqlitePool;

/// 配置存储服务
pub struct ConfigStorage;

impl ConfigStorage {
    /// 加载应用配置
    pub async fn load<S: StorageConfig>(
        pool: &SqlitePool,
        storage: &S,
    ) -> Result<AppConfig, String> {
        // 1. 尝试从数据库加载
        let row = sqlx::query("SELECT value FROM configs WHERE key = 'app_config'")
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("Failed to fetch config from DB: {}", e))?;

        if let Some(row) = row {
            use sqlx::Row;
            let value: String = row.get("value");
            let config: AppConfig = serde_json::from_str(&value)
                .map_err(|e| format!("Failed to parse config from DB: {}", e))?;
            return Ok(config);
        }

        // 2. 如果数据库没有，尝试从旧的 JSON 文件加载 (迁移逻辑)
        let config_path = storage.config_path();
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| format!("读取配置文件失败: {}", e))?;
            let config: AppConfig =
                serde_json::from_str(&content).map_err(|e| format!("解析配置文件失败: {}", e))?;

            // 迁移到数据库
            Self::save(pool, &config).await?;
            tracing::info!("Migrated config.json to database.");
            return Ok(config);
        }

        // 3. 都没有，则返回默认并保存到数据库
        let default_config = AppConfig::default();
        Self::save(pool, &default_config).await?;
        Ok(default_config)
    }

    /// 保存应用配置到数据库
    pub async fn save(pool: &SqlitePool, config: &AppConfig) -> Result<(), String> {
        let content =
            serde_json::to_string_pretty(config).map_err(|e| format!("序列化配置失败: {}", e))?;

        sqlx::query(
            "INSERT INTO configs (key, value) VALUES ('app_config', ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(content)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to save config to DB: {}", e))?;

        Ok(())
    }
}
