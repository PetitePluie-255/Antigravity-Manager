import Database from "better-sqlite3";
import path from "path";
import fs from "fs";
import os from "os";

// Data directory
const DATA_DIR =
  process.env.DATA_DIR || path.join(os.homedir(), ".antigravity_tools");
const DB_PATH = path.join(DATA_DIR, "accounts.db");

// Ensure data directory exists
if (!fs.existsSync(DATA_DIR)) {
  fs.mkdirSync(DATA_DIR, { recursive: true });
}

const db: Database.Database = new Database(DB_PATH);

// Initialize database schema
db.exec(`
    CREATE TABLE IF NOT EXISTS accounts (
        id TEXT PRIMARY KEY,
        email TEXT NOT NULL,
        name TEXT,
        refresh_token TEXT NOT NULL,
        access_token TEXT,
        token_type TEXT DEFAULT 'Bearer',
        expires_in INTEGER DEFAULT 3600,
        expiry_timestamp INTEGER DEFAULT 0,
        quota_data TEXT,
        created_at INTEGER NOT NULL,
        last_used INTEGER NOT NULL,
        is_forbidden INTEGER DEFAULT 0
    );

    CREATE TABLE IF NOT EXISTS config (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS current_account (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        account_id TEXT
    );
`);

console.log(`📦 Database initialized at: ${DB_PATH}`);

export { db, DATA_DIR, DB_PATH };
