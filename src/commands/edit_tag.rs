use anyhow::{bail, Result};

use crate::db;
use crate::utils;

pub fn run(name: &str, tags: &[String], add: bool, delete: bool) -> Result<()> {
    if !add && !delete {
        bail!("Must specify --add or --delete");
    }
    if add && delete {
        bail!("Cannot specify both --add and --delete");
    }
    if tags.is_empty() {
        bail!("Must provide at least one tag.");
    }

    utils::validate_name(name)?;
    utils::validate_tags(tags)?;

    let conn = db::get_db()?;
    let (id, mut existing_tags) = db::get_tags_and_id(&conn, name)?
        .ok_or_else(|| anyhow::anyhow!("Note '{}' not found.", name))?;

    if add {
        for tag in tags {
            if !existing_tags.contains(tag) {
                existing_tags.push(tag.clone());
            }
        }
        println!("Tags added to '{}'.", name);
    } else {
        existing_tags.retain(|t| !tags.contains(t));
        println!("Tags removed from '{}'.", name);
    }

    db::update_tags(&conn, id, &existing_tags)?;
    Ok(())
}
