use anyhow::{bail, Result};

use crate::db;

pub fn run(name: &str) -> Result<()> {
    let conn = db::get_db()?;

    match db::get_note(&conn, name)? {
        Some((_id, content, _tags)) => {
            print!("{content}");
            Ok(())
        }
        None => bail!("Note '{}' not found", name),
    }
}
