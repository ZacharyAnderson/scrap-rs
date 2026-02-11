use anyhow::{Context, Result};
use serde::Serialize;
use std::fs::File;
use std::io::Write;

use crate::db;

#[derive(Serialize)]
struct ExportNote {
    title: String,
    note: String,
    tags: Vec<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize)]
struct ExportData {
    version: u32,
    exported_at: String,
    notes: Vec<ExportNote>,
}

pub fn run(path: &str) -> Result<()> {
    let conn = db::get_db()?;

    let mut stmt = conn.prepare(
        "SELECT title, note, tags, created_at, updated_at FROM notes ORDER BY id"
    )?;

    let rows = stmt.query_map([], |row| {
        let tags_json: String = row.get(2)?;
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        Ok(ExportNote {
            title: row.get(0)?,
            note: row.get(1)?,
            tags,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        })
    })?;

    let mut notes = Vec::new();
    for row in rows {
        notes.push(row?);
    }

    let count = notes.len();

    let export = ExportData {
        version: 1,
        exported_at: current_timestamp(),
        notes,
    };

    let json = serde_json::to_string_pretty(&export)?;

    let mut file = File::create(path)
        .with_context(|| format!("Failed to create file: {}", path))?;

    file.write_all(json.as_bytes())?;

    println!("Exported {} notes to {}", count, path);
    Ok(())
}

fn current_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}
