use anyhow::{bail, Result};

use crate::db;
use crate::utils;

pub fn run(name: &str, tags: &[String]) -> Result<()> {
    utils::validate_name(name)?;
    utils::validate_tags(tags)?;

    let conn = db::get_db()?;

    if db::get_note(&conn, name)?.is_some() {
        bail!("Note '{}' already exists. Use 'open' to edit it.", name);
    }

    let contents = utils::get_user_input(name)?;
    db::insert_note(&conn, name, &contents, tags)?;
    println!("Note '{}' created.", name);
    Ok(())
}
