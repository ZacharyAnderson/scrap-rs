use anyhow::Result;

use crate::db;
use crate::utils;

pub fn run(name: &str) -> Result<()> {
    utils::validate_name(name)?;

    let conn = db::get_db()?;
    if db::delete_note(&conn, name)? {
        println!("Note '{}' deleted.", name);
    } else {
        println!("Note '{}' not found.", name);
    }
    Ok(())
}
