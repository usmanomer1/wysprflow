// Dictionary CRUD. Words are fed into the Anthropic cleanup prompt so Haiku spells
// names, jargon, and project-specific terms correctly.

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictEntry {
    pub id: i64,
    pub word: String,
    pub is_starred: bool,
    pub auto_learned: bool,
    pub usage_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

pub fn list(db: &Db) -> Result<Vec<DictEntry>> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare(
            "SELECT id, word, is_starred, auto_learned, usage_count, created_at, updated_at \
             FROM dictionary ORDER BY is_starred DESC, updated_at DESC",
        )
        .context("prepare dictionary list")?;
    let rows = stmt
        .query_map([], |row| {
            Ok(DictEntry {
                id: row.get(0)?,
                word: row.get(1)?,
                is_starred: row.get::<_, i64>(2)? != 0,
                auto_learned: row.get::<_, i64>(3)? != 0,
                usage_count: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .context("collect dictionary rows")?;
    Ok(rows)
}

pub fn list_words(db: &Db) -> Result<Vec<String>> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare("SELECT word FROM dictionary ORDER BY is_starred DESC, usage_count DESC")
        .context("prepare dictionary words")?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()
        .context("collect dictionary words")?;
    Ok(rows)
}

pub fn add(db: &Db, word: &str) -> Result<DictEntry> {
    let trimmed = word.trim();
    if trimmed.is_empty() {
        anyhow::bail!("empty word");
    }
    let now = Utc::now().to_rfc3339();
    let conn = db.lock();
    conn.execute(
        "INSERT INTO dictionary (word, created_at, updated_at) VALUES (?, ?, ?) \
         ON CONFLICT(word) DO UPDATE SET updated_at = excluded.updated_at",
        params![trimmed, now, now],
    )
    .context("insert dictionary word")?;
    let id: i64 = conn
        .query_row(
            "SELECT id FROM dictionary WHERE word = ? COLLATE NOCASE",
            params![trimmed],
            |row| row.get(0),
        )
        .context("lookup inserted word")?;
    drop(conn);
    fetch(db, id)
}

pub fn fetch(db: &Db, id: i64) -> Result<DictEntry> {
    let conn = db.lock();
    conn.query_row(
        "SELECT id, word, is_starred, auto_learned, usage_count, created_at, updated_at \
         FROM dictionary WHERE id = ?",
        params![id],
        |row| {
            Ok(DictEntry {
                id: row.get(0)?,
                word: row.get(1)?,
                is_starred: row.get::<_, i64>(2)? != 0,
                auto_learned: row.get::<_, i64>(3)? != 0,
                usage_count: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        },
    )
    .context("fetch dictionary row")
}

pub fn delete(db: &Db, id: i64) -> Result<()> {
    let conn = db.lock();
    conn.execute("DELETE FROM dictionary WHERE id = ?", params![id])
        .context("delete dictionary row")?;
    Ok(())
}

pub fn toggle_star(db: &Db, id: i64) -> Result<DictEntry> {
    let now = Utc::now().to_rfc3339();
    let conn = db.lock();
    conn.execute(
        "UPDATE dictionary SET is_starred = 1 - is_starred, updated_at = ? WHERE id = ?",
        params![now, id],
    )
    .context("toggle star")?;
    drop(conn);
    fetch(db, id)
}

#[allow(dead_code)]
pub fn record_usage(db: &Db, word: &str) -> Result<()> {
    let conn = db.lock();
    conn.execute(
        "UPDATE dictionary SET usage_count = usage_count + 1, updated_at = ? \
         WHERE word = ? COLLATE NOCASE",
        params![Utc::now().to_rfc3339(), word],
    )
    .context("record usage")?;
    Ok(())
}
