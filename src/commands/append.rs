use anyhow::{bail, Context, Result};
use std::io::Read;

use crate::db;

pub fn run(name: &str) -> Result<()> {
    let conn = db::get_db()?;

    let (id, existing, _tags) = match db::get_note(&conn, name)? {
        Some(row) => row,
        None => bail!("Note '{}' not found", name),
    };

    let mut new_content = String::new();
    std::io::stdin()
        .read_to_string(&mut new_content)
        .context("Failed to read from stdin")?;

    let combined = format!("{existing}\n{new_content}");
    db::update_note(&conn, id, &combined)?;
    db::mark_summary_stale(&conn, id)?;

    Ok(())
}
