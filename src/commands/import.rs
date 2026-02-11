use anyhow::{Context, Result};
use rusqlite::params;
use serde::Deserialize;
use std::fs;

use crate::db;

#[derive(Deserialize)]
struct ImportNote {
    title: String,
    note: String,
    tags: Vec<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize)]
struct ImportData {
    #[allow(dead_code)]
    version: u32,
    #[allow(dead_code)]
    exported_at: String,
    notes: Vec<ImportNote>,
}

pub fn run(path: &str, overwrite: bool) -> Result<()> {
    let conn = db::get_db()?;

    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path))?;

    let data: ImportData = serde_json::from_str(&contents)
        .with_context(|| "Failed to parse export file. Is it a valid scrap export?")?;

    if overwrite {
        conn.execute("DELETE FROM notes", [])?;
        println!("Cleared existing notes.");
    }

    let mut imported = 0;
    let mut skipped = 0;

    for note in data.notes {
        let tags_json = serde_json::to_string(&note.tags)?;

        if overwrite {
            conn.execute(
                "INSERT INTO notes (title, note, tags, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![note.title, note.note, tags_json, note.created_at, note.updated_at],
            )?;
            imported += 1;
        } else {
            // Check if note with this title already exists
            let exists: bool = conn.query_row(
                "SELECT 1 FROM notes WHERE title = ?1",
                params![note.title],
                |_| Ok(true),
            ).unwrap_or(false);

            if exists {
                skipped += 1;
            } else {
                conn.execute(
                    "INSERT INTO notes (title, note, tags, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![note.title, note.note, tags_json, note.created_at, note.updated_at],
                )?;
                imported += 1;
            }
        }
    }

    if overwrite {
        println!("Imported {} notes from {}", imported, path);
    } else {
        println!("Imported {} notes, skipped {} duplicates from {}", imported, skipped, path);
    }

    Ok(())
}
