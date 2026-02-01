use anyhow::Result;

use crate::db;
use crate::utils;

pub fn run(name: &str) -> Result<()> {
    utils::validate_name(name)?;

    let conn = db::get_db()?;

    let (id, contents, _tags) = db::get_note(&conn, name)?
        .ok_or_else(|| anyhow::anyhow!("Note '{}' not found.", name))?;

    let new_contents = utils::get_user_input_with_contents(name, &contents)?;

    if new_contents == contents {
        println!("No changes made.");
        return Ok(());
    }

    db::update_note(&conn, id, &new_contents)?;
    println!("Note '{}' updated.", name);
    Ok(())
}
