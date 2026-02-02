use anyhow::{Context, Result};
use std::io::Read;

use crate::db;

pub fn run(name: &str, tags: &[String]) -> Result<()> {
    let mut content = String::new();
    std::io::stdin()
        .read_to_string(&mut content)
        .context("Failed to read from stdin")?;

    let conn = db::get_db()?;

    match db::get_note(&conn, name)? {
        Some((id, _existing, _tags)) => {
            db::update_note(&conn, id, &content)?;
            if !tags.is_empty() {
                db::update_tags(&conn, id, tags)?;
            }
            db::mark_summary_stale(&conn, id)?;
        }
        None => {
            db::insert_note(&conn, name, &content, tags)?;
        }
    }

    Ok(())
}
