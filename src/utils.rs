use anyhow::{bail, Context, Result};
use std::process::Command;

pub fn validate_name(name: &str) -> Result<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        bail!("Note name cannot be empty.");
    }
    if trimmed.contains(' ') {
        bail!("Note name cannot contain spaces. Use hyphens or underscores instead.");
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        bail!("Note name cannot contain path separators.");
    }
    if trimmed.len() > 100 {
        bail!("Note name cannot exceed 100 characters.");
    }
    Ok(())
}

pub fn validate_tags(tags: &[String]) -> Result<()> {
    for tag in tags {
        let trimmed = tag.trim();
        if trimmed.is_empty() {
            bail!("Tag cannot be empty.");
        }
        if trimmed.contains(' ') {
            bail!("Tag '{}' cannot contain spaces.", tag);
        }
        if trimmed.len() > 50 {
            bail!("Tag '{}' cannot exceed 50 characters.", tag);
        }
    }
    Ok(())
}

pub fn get_editor() -> Result<String> {
    if let Ok(editor) = std::env::var("EDITOR") {
        return Ok(editor);
    }
    let candidates = ["nvim", "vim", "vi", "nano", "emacs"];
    for name in candidates {
        let check = Command::new("which").arg(name).output();
        if let Ok(output) = check {
            if output.status.success() {
                return Ok(name.to_string());
            }
        }
    }
    bail!("No editor found. Set the $EDITOR environment variable.")
}

pub fn get_user_input(name: &str) -> Result<String> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let temp_dir = home.join(".scrap/temp");
    std::fs::create_dir_all(&temp_dir)?;
    let temp_file = temp_dir.join(format!("{}.md", name));

    if !temp_file.exists() {
        std::fs::write(&temp_file, "")?;
    }

    let editor = get_editor()?;
    let status = Command::new(&editor)
        .arg(&temp_file)
        .status()
        .with_context(|| format!("Failed to open editor: {}", editor))?;

    if !status.success() {
        bail!("Editor exited with non-zero status");
    }

    let contents = std::fs::read_to_string(&temp_file)?;
    std::fs::remove_file(&temp_file)?;
    Ok(contents)
}

pub fn get_user_input_with_contents(name: &str, existing: &str) -> Result<String> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let temp_dir = home.join(".scrap/temp");
    std::fs::create_dir_all(&temp_dir)?;
    let temp_file = temp_dir.join(format!("{}.md", name));

    std::fs::write(&temp_file, existing)?;

    let editor = get_editor()?;
    let status = Command::new(&editor)
        .arg(&temp_file)
        .status()
        .with_context(|| format!("Failed to open editor: {}", editor))?;

    if !status.success() {
        bail!("Editor exited with non-zero status");
    }

    let contents = std::fs::read_to_string(&temp_file)?;
    std::fs::remove_file(&temp_file)?;
    Ok(contents)
}
