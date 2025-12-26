//! 数据库导入服务
//! 支持从 PostgreSQL 和 SQLite 导入账号数据

use serde::Deserialize;

/// 导入配置
#[derive(Debug, Deserialize)]
pub struct ImportConfig {
    /// 数据库表名 (默认 "accounts")
    #[serde(default = "default_table")]
    pub table: String,
    /// email 列名 (默认 "email")
    #[serde(default = "default_email_column")]
    pub email_column: String,
    /// refresh_token 列名 (默认 "refresh_token")
    #[serde(default = "default_token_column")]
    pub token_column: String,
}

fn default_table() -> String {
    "accounts".to_string()
}
fn default_email_column() -> String {
    "email".to_string()
}
fn default_token_column() -> String {
    "refresh_token".to_string()
}

impl Default for ImportConfig {
    fn default() -> Self {
        Self {
            table: default_table(),
            email_column: default_email_column(),
            token_column: default_token_column(),
        }
    }
}

/// 从数据库导入的账号数据
#[derive(Debug)]
pub struct ImportedAccount {
    pub email: Option<String>,
    pub refresh_token: String,
}

/// 数据库导入器
pub struct DatabaseImporter;

impl DatabaseImporter {
    /// 从 PostgreSQL 导入账号
    #[cfg(feature = "web-server")]
    pub async fn import_from_postgres(
        url: &str,
        config: &ImportConfig,
    ) -> Result<Vec<ImportedAccount>, String> {
        use sqlx::postgres::PgPoolOptions;
        use sqlx::Row;

        // 创建连接池
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(url)
            .await
            .map_err(|e| format!("连接 PostgreSQL 失败: {}", e))?;

        // 构建查询
        let query = format!(
            "SELECT {email}, {token} FROM {table}",
            email = config.email_column,
            token = config.token_column,
            table = config.table
        );

        // 执行查询
        let rows = sqlx::query(&query)
            .fetch_all(&pool)
            .await
            .map_err(|e| format!("查询失败: {}", e))?;

        // 解析结果
        let accounts: Vec<ImportedAccount> = rows
            .iter()
            .filter_map(|row| {
                let refresh_token: String = row.try_get(&config.token_column as &str).ok()?;
                let email: Option<String> = row.try_get(&config.email_column as &str).ok();
                Some(ImportedAccount {
                    email,
                    refresh_token,
                })
            })
            .collect();

        Ok(accounts)
    }

    /// 从 SQLite 导入账号
    #[cfg(feature = "web-server")]
    pub async fn import_from_sqlite(
        path: &str,
        config: &ImportConfig,
    ) -> Result<Vec<ImportedAccount>, String> {
        use sqlx::sqlite::SqlitePoolOptions;
        use sqlx::Row;

        // 构建连接 URL
        let url = if path.starts_with("sqlite:") {
            path.to_string()
        } else {
            format!("sqlite:{}", path)
        };

        // 创建连接池
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .map_err(|e| format!("连接 SQLite 失败: {}", e))?;

        // 构建查询
        let query = format!(
            "SELECT {email}, {token} FROM {table}",
            email = config.email_column,
            token = config.token_column,
            table = config.table
        );

        // 执行查询
        let rows = sqlx::query(&query)
            .fetch_all(&pool)
            .await
            .map_err(|e| format!("查询失败: {}", e))?;

        // 解析结果
        let accounts: Vec<ImportedAccount> = rows
            .iter()
            .filter_map(|row| {
                let refresh_token: String = row.try_get(&config.token_column as &str).ok()?;
                let email: Option<String> = row.try_get(&config.email_column as &str).ok();
                Some(ImportedAccount {
                    email,
                    refresh_token,
                })
            })
            .collect();

        Ok(accounts)
    }

    /// 根据 URL 自动选择数据库类型导入
    #[cfg(feature = "web-server")]
    pub async fn import_from_url(
        url: &str,
        config: &ImportConfig,
    ) -> Result<Vec<ImportedAccount>, String> {
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            Self::import_from_postgres(url, config).await
        } else if url.starts_with("sqlite:") || url.ends_with(".db") || url.ends_with(".sqlite") {
            Self::import_from_sqlite(url, config).await
        } else {
            Err("不支持的数据库类型，请使用 postgres:// 或 sqlite: 开头的 URL".to_string())
        }
    }
}
