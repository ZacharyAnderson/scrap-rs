use anyhow::{Context, Result};
use rusqlite::{params, Connection};

#[derive(Clone)]
pub struct NoteEntry {
    pub id: i64,
    pub title: String,
    pub note: String,
    pub tags: Vec<String>,
    #[allow(dead_code)]
    pub updated_at: String,
}

fn db_path() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let dir = home.join(".scrap");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("scrap.db"))
}

pub fn get_db() -> Result<Connection> {
    let path = db_path()?;
    let conn = Connection::open(&path)?;
    conn.execute_batch(
        "DROP TRIGGER IF EXISTS update_last_modified;
        CREATE TABLE IF NOT EXISTS notes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            note TEXT NOT NULL,
            tags JSON,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TRIGGER IF NOT EXISTS update_notes_updated_at
            AFTER UPDATE ON notes
            WHEN old.updated_at <> CURRENT_TIMESTAMP
        BEGIN
            UPDATE notes SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
        END;",
    )?;
    // Migration: add summary columns if they don't exist
    let _ = conn.execute_batch("ALTER TABLE notes ADD COLUMN summary TEXT;");
    let _ = conn.execute_batch("ALTER TABLE notes ADD COLUMN summary_stale INTEGER NOT NULL DEFAULT 0;");
    Ok(conn)
}

pub fn insert_note(conn: &Connection, name: &str, contents: &str, tags: &[String]) -> Result<()> {
    let tags_json = serde_json::to_string(tags)?;
    conn.execute(
        "INSERT INTO notes (title, note, tags) VALUES (?1, ?2, ?3)",
        params![name, contents, tags_json],
    )?;
    Ok(())
}

pub fn get_note(conn: &Connection, name: &str) -> Result<Option<(i64, String, String)>> {
    let mut stmt = conn.prepare("SELECT id, note, tags FROM notes WHERE title = ?1")?;
    let mut rows = stmt.query(params![name])?;
    match rows.next()? {
        Some(row) => Ok(Some((row.get(0)?, row.get(1)?, row.get(2)?))),
        None => Ok(None),
    }
}

pub fn get_tags_and_id(conn: &Connection, name: &str) -> Result<Option<(i64, Vec<String>)>> {
    let mut stmt = conn.prepare("SELECT id, tags FROM notes WHERE title = ?1")?;
    let mut rows = stmt.query(params![name])?;
    match rows.next()? {
        Some(row) => {
            let id: i64 = row.get(0)?;
            let tags_str: String = row.get(1)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str)?;
            Ok(Some((id, tags)))
        }
        None => Ok(None),
    }
}

pub fn update_note(conn: &Connection, id: i64, contents: &str) -> Result<()> {
    conn.execute(
        "UPDATE notes SET note = ?1 WHERE id = ?2",
        params![contents, id],
    )?;
    Ok(())
}

pub fn update_tags(conn: &Connection, id: i64, tags: &[String]) -> Result<()> {
    let tags_json = serde_json::to_string(tags)?;
    conn.execute(
        "UPDATE notes SET tags = ?1 WHERE id = ?2",
        params![tags_json, id],
    )?;
    Ok(())
}

pub fn delete_note(conn: &Connection, name: &str) -> Result<bool> {
    let count = conn.execute("DELETE FROM notes WHERE title = ?1", params![name])?;
    Ok(count > 0)
}

pub fn get_summary(conn: &Connection, id: i64) -> Result<Option<(String, bool)>> {
    let mut stmt = conn.prepare("SELECT summary, summary_stale FROM notes WHERE id = ?1")?;
    let mut rows = stmt.query(params![id])?;
    match rows.next()? {
        Some(row) => {
            let summary: Option<String> = row.get(0)?;
            match summary {
                Some(s) if !s.is_empty() => {
                    let stale: i64 = row.get(1)?;
                    Ok(Some((s, stale != 0)))
                }
                _ => Ok(None),
            }
        }
        None => Ok(None),
    }
}

pub fn set_summary(conn: &Connection, id: i64, summary: &str) -> Result<()> {
    conn.execute(
        "UPDATE notes SET summary = ?1, summary_stale = 0 WHERE id = ?2",
        params![summary, id],
    )?;
    Ok(())
}

pub fn mark_summary_stale(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE notes SET summary_stale = 1 WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

pub fn list_notes(conn: &Connection) -> Result<Vec<NoteEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, note, tags, updated_at FROM notes ORDER BY updated_at DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        let tags_str: String = row.get(3)?;
        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
        Ok(NoteEntry {
            id: row.get(0)?,
            title: row.get(1)?,
            note: row.get(2)?,
            tags,
            updated_at: row.get(4)?,
        })
    })?;
    let mut notes = Vec::new();
    for row in rows {
        notes.push(row?);
    }
    Ok(notes)
}
