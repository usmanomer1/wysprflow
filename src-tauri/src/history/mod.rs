// Dictation history. The pipeline calls `record()` at the end of each session
// (success or error). The Settings UI Run Log tab reads `list()`.

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp: String,
    pub raw_transcript: String,
    pub cleaned_transcript: String,
    pub source_app: Option<String>,
    pub duration_ms: Option<i64>,
    pub word_count: i64,
    pub error: Option<String>,
}

pub struct NewEntry {
    pub raw: String,
    pub cleaned: String,
    pub source_app: Option<String>,
    pub duration_ms: Option<i64>,
    pub error: Option<String>,
}

pub fn record(db: &Db, entry: NewEntry) -> Result<HistoryEntry> {
    if entry.raw.trim().is_empty() && entry.cleaned.trim().is_empty() && entry.error.is_none() {
        // Don't bother recording empty no-op sessions.
        anyhow::bail!("empty history entry");
    }
    let id = Uuid::new_v4().to_string();
    let timestamp = Utc::now().to_rfc3339();
    let word_count = entry.cleaned.split_whitespace().count() as i64;
    let conn = db.lock();
    conn.execute(
        "INSERT INTO history (id, timestamp, raw_transcript, cleaned_transcript, source_app, duration_ms, word_count, error) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            id,
            timestamp,
            entry.raw,
            entry.cleaned,
            entry.source_app,
            entry.duration_ms,
            word_count,
            entry.error,
        ],
    )?;
    drop(conn);
    fetch(db, &id)
}

pub fn fetch(db: &Db, id: &str) -> Result<HistoryEntry> {
    let conn = db.lock();
    conn.query_row(
        "SELECT id, timestamp, raw_transcript, cleaned_transcript, source_app, duration_ms, word_count, error \
         FROM history WHERE id = ?",
        params![id],
        row_to_entry,
    )
    .context("fetch history entry")
}

pub fn list(db: &Db, limit: i64) -> Result<Vec<HistoryEntry>> {
    let conn = db.lock();
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, raw_transcript, cleaned_transcript, source_app, duration_ms, word_count, error \
         FROM history ORDER BY timestamp DESC LIMIT ?",
    )?;
    let rows = stmt
        .query_map(params![limit], row_to_entry)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn search(db: &Db, query: &str, limit: i64) -> Result<Vec<HistoryEntry>> {
    let q = format!("%{}%", query.trim());
    let conn = db.lock();
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, raw_transcript, cleaned_transcript, source_app, duration_ms, word_count, error \
         FROM history WHERE cleaned_transcript LIKE ? OR raw_transcript LIKE ? \
         ORDER BY timestamp DESC LIMIT ?",
    )?;
    let rows = stmt
        .query_map(params![&q, &q, limit], row_to_entry)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn delete(db: &Db, id: &str) -> Result<()> {
    let conn = db.lock();
    conn.execute("DELETE FROM history WHERE id = ?", params![id])?;
    Ok(())
}

pub fn clear_all(db: &Db) -> Result<()> {
    let conn = db.lock();
    conn.execute("DELETE FROM history", [])?;
    Ok(())
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryEntry> {
    Ok(HistoryEntry {
        id: row.get(0)?,
        timestamp: row.get(1)?,
        raw_transcript: row.get(2)?,
        cleaned_transcript: row.get(3)?,
        source_app: row.get(4)?,
        duration_ms: row.get(5)?,
        word_count: row.get(6)?,
        error: row.get(7)?,
    })
}
