// SQLite-backed storage for dictionary, snippets, and history.
//
// Connection is wrapped in Arc<Mutex<>> so it can be shared as Tauri-managed state
// across sync/async command handlers and pipeline tasks. Operations are quick at
// our scale (hundreds-to-thousands of rows) so a single shared connection is fine.

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use parking_lot::Mutex;
use rusqlite::Connection;

pub type Db = Arc<Mutex<Connection>>;

const MIGRATIONS: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS dictionary (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word TEXT NOT NULL UNIQUE COLLATE NOCASE,
    is_starred INTEGER NOT NULL DEFAULT 0,
    auto_learned INTEGER NOT NULL DEFAULT 0,
    usage_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_dictionary_starred ON dictionary(is_starred DESC, updated_at DESC);

CREATE TABLE IF NOT EXISTS snippets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    trigger TEXT NOT NULL UNIQUE COLLATE NOCASE,
    expansion TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS history (
    id TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL,
    raw_transcript TEXT NOT NULL,
    cleaned_transcript TEXT NOT NULL,
    source_app TEXT,
    duration_ms INTEGER,
    word_count INTEGER NOT NULL DEFAULT 0,
    error TEXT
);

CREATE INDEX IF NOT EXISTS idx_history_timestamp ON history(timestamp DESC);
"#;

pub fn open(path: &Path) -> Result<Db> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("create db dir")?;
    }
    let conn = Connection::open(path).context("sqlite open")?;
    conn.execute_batch(MIGRATIONS).context("apply migrations")?;
    Ok(Arc::new(Mutex::new(conn)))
}
