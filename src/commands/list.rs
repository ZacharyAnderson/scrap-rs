use anyhow::Result;

use crate::db;

pub fn run(tag: Option<&str>) -> Result<()> {
    let conn = db::get_db()?;
    let notes = db::list_notes(&conn)?;

    for note in notes {
        if let Some(filter) = tag {
            if !note.tags.iter().any(|t| t == filter) {
                continue;
            }
        }
        println!("{}", note.title);
    }

    Ok(())
}
