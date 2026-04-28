// Snippets ("voice macros"). User says a trigger phrase, the cleaned transcript is
// matched against the trigger, and the expansion is injected instead. This bypasses
// LLM cleanup for the expansion text (so signatures, addresses, etc. paste verbatim).

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub id: i64,
    pub trigger: String,
    pub expansion: String,
    pub created_at: String,
    pub updated_at: String,
}

pub fn list(db: &Db) -> Result<Vec<Snippet>> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare(
            "SELECT id, trigger, expansion, created_at, updated_at \
             FROM snippets ORDER BY trigger ASC",
        )
        .context("prepare snippets list")?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Snippet {
                id: row.get(0)?,
                trigger: row.get(1)?,
                expansion: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn upsert(db: &Db, id: Option<i64>, trigger: &str, expansion: &str) -> Result<Snippet> {
    let trigger = trigger.trim();
    if trigger.is_empty() {
        anyhow::bail!("empty trigger");
    }
    if expansion.is_empty() {
        anyhow::bail!("empty expansion");
    }
    let now = Utc::now().to_rfc3339();
    let conn = db.lock();
    let new_id = match id {
        Some(existing_id) => {
            conn.execute(
                "UPDATE snippets SET trigger = ?, expansion = ?, updated_at = ? WHERE id = ?",
                params![trigger, expansion, now, existing_id],
            )?;
            existing_id
        }
        None => {
            conn.execute(
                "INSERT INTO snippets (trigger, expansion, created_at, updated_at) VALUES (?, ?, ?, ?)",
                params![trigger, expansion, now, now],
            )?;
            conn.last_insert_rowid()
        }
    };
    drop(conn);
    fetch(db, new_id)
}

pub fn fetch(db: &Db, id: i64) -> Result<Snippet> {
    let conn = db.lock();
    conn.query_row(
        "SELECT id, trigger, expansion, created_at, updated_at FROM snippets WHERE id = ?",
        params![id],
        |row| {
            Ok(Snippet {
                id: row.get(0)?,
                trigger: row.get(1)?,
                expansion: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        },
    )
    .context("fetch snippet")
}

pub fn delete(db: &Db, id: i64) -> Result<()> {
    let conn = db.lock();
    conn.execute("DELETE FROM snippets WHERE id = ?", params![id])?;
    Ok(())
}

/// Try to match a cleaned transcript against any snippet trigger.
/// Returns the expansion if matched, otherwise None.
pub fn match_trigger(db: &Db, transcript: &str) -> Result<Option<String>> {
    let normalized = transcript
        .trim()
        .trim_end_matches(|c: char| c.is_ascii_punctuation())
        .trim()
        .to_lowercase();
    if normalized.is_empty() {
        return Ok(None);
    }
    let conn = db.lock();
    let result: Option<String> = conn
        .query_row(
            "SELECT expansion FROM snippets WHERE LOWER(trigger) = ? LIMIT 1",
            params![normalized],
            |row| row.get(0),
        )
        .ok();
    Ok(result)
}
