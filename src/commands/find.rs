use anyhow::Result;
use std::process::Command;

use crate::utils;

pub fn run(query: Option<&str>) -> Result<()> {
    let script = utils::find_explorer_script()?;

    let mut cmd = Command::new("bash");
    cmd.arg(&script);
    if let Some(q) = query {
        cmd.arg(q);
    }

    let status = cmd.status()?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}
