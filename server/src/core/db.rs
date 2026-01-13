use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;

pub async fn init_db(data_dir: &Path) -> Result<SqlitePool, String> {
    let db_path = data_dir.join("antigravity.db");
    let db_url = format!("sqlite:{}", db_path.to_string_lossy());

    let options = SqliteConnectOptions::from_str(&db_url)
        .map_err(|e| e.to_string())?
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(|e| format!("Failed to connect to database: {}", e))?;

    // Run migrations
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS proxy_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            method TEXT NOT NULL,
            url TEXT NOT NULL,
            account_email TEXT NOT NULL,
            model TEXT NOT NULL,
            tokens_in INTEGER NOT NULL,
            tokens_out INTEGER NOT NULL,
            latency_ms INTEGER NOT NULL,
            status_code INTEGER NOT NULL,
            error TEXT,
            request_body TEXT,
            response_body TEXT
        );",
    )
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to create proxy_logs table: {}", e))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS accounts (
            id TEXT PRIMARY KEY,
            email TEXT UNIQUE NOT NULL,
            access_token TEXT NOT NULL,
            refresh_token TEXT NOT NULL,
            expires_in INTEGER NOT NULL,
            expiry_timestamp INTEGER NOT NULL,
            project_id TEXT,
            disabled BOOLEAN DEFAULT FALSE,
            disabled_reason TEXT,
            subscription_tier TEXT
        );",
    )
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to create accounts table: {}", e))?;

    // Add new columns if they don't exist
    let new_columns = [
        ("name", "TEXT"),
        ("created_at", "INTEGER"),
        ("last_used", "INTEGER"),
        ("is_current", "BOOLEAN DEFAULT FALSE"),
        ("quota", "TEXT"),
        ("device_profile", "TEXT"),
        ("device_history", "TEXT"),
        ("disabled", "BOOLEAN DEFAULT FALSE"),
        ("disabled_reason", "TEXT"),
        ("disabled_at", "INTEGER"),
        ("proxy_disabled", "BOOLEAN DEFAULT FALSE"),
        ("proxy_disabled_reason", "TEXT"),
        ("proxy_disabled_at", "INTEGER"),
    ];

    for (name, col_type) in new_columns {
        let _ = sqlx::query(&format!(
            "ALTER TABLE accounts ADD COLUMN {} {}",
            name, col_type
        ))
        .execute(&pool)
        .await;
    }

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS configs (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to create configs table: {}", e))?;

    Ok(pool)
}
