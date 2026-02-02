use anyhow::Result;
use crossterm::{
    event::{KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use std::time::{Duration, Instant};

use super::{App, Focus, Mode, PreviewTab};
use crate::db;
use crate::llm;
use crate::utils;

pub fn handle_key(
    app: &mut App,
    key: KeyEvent,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> Result<()> {
    // Preview focus is handled regardless of mode
    if app.focus == Focus::Preview && app.mode == Mode::Normal {
        return handle_preview(app, key);
    }

    match &app.mode {
        Mode::Normal => handle_normal(app, key),
        Mode::TagBrowse => handle_tag_browse(app, key),
        Mode::Search => handle_search(app, key),
        Mode::Command => handle_command(app, key, terminal),
        Mode::AddNoteName => handle_add_note_name(app, key),
        Mode::AddNoteTags => handle_add_note_tags(app, key, terminal),
        Mode::EditTagsAdd | Mode::EditTagsRemove => handle_edit_tags(app, key),
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => {
            app.move_selection(1);
            clear_summary(app);
            app.preview_scroll = 0;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.move_selection(-1);
            clear_summary(app);
            app.preview_scroll = 0;
        }
        KeyCode::Char('/') => {
            app.mode = Mode::Search;
            app.search_query.clear();
            app.apply_filter();
            app.selected = 0;
            app.status_message = None;
        }
        KeyCode::Char(':') => {
            app.mode = Mode::Command;
            app.status_message = None;
        }
        KeyCode::Esc => {
            clear_summary(app);
        }
        KeyCode::Tab => {
            app.focus = Focus::TagPanel;
            app.mode = Mode::TagBrowse;
            app.status_message = None;
            clear_summary(app);
        }
        _ => {}
    }
    Ok(())
}

fn clear_summary(app: &mut App) {
    app.showing_summary = false;
    app.summary_content = None;
    app.summary_stale = false;
    app.summary_force_regen = false;
}

fn handle_preview(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => {
            app.preview_scroll = app.preview_scroll.saturating_add(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.preview_scroll = app.preview_scroll.saturating_sub(1);
        }
        KeyCode::Tab => {
            match app.preview_tab {
                PreviewTab::Note => {
                    // Try to load summary from DB if not already in memory
                    if app.summary_content.is_none() {
                        if let Some(note) = app.selected_note() {
                            if let Ok(Some((summary, stale))) = db::get_summary(&app.conn, note.id) {
                                app.summary_content = Some(summary);
                                app.showing_summary = true;
                                app.summary_stale = stale;
                            }
                        }
                    }
                    if app.summary_content.is_some() {
                        app.preview_tab = PreviewTab::Summary;
                        app.preview_scroll = 0;
                    } else {
                        app.status_message = Some("No summary available. Use :s to generate.".to_string());
                        app.status_expires = Some(Instant::now() + Duration::from_secs(3));
                        app.focus = Focus::NoteList;
                        app.preview_scroll = 0;
                    }
                }
                PreviewTab::Summary => {
                    app.focus = Focus::NoteList;
                    app.preview_tab = PreviewTab::Note;
                    app.preview_scroll = 0;
                }
            }
        }
        KeyCode::Esc => {
            app.focus = Focus::NoteList;
            app.preview_scroll = 0;
        }
        KeyCode::Char(':') => {
            app.focus = Focus::NoteList;
            app.mode = Mode::Command;
            app.status_message = None;
        }
        _ => {}
    }
    Ok(())
}

fn handle_tag_browse(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.move_tag_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_tag_selection(-1),
        KeyCode::Enter => {
            if let Some(tag_name) = app.visible_tags.get(app.selected_tag).map(|t| t.name.clone()) {
                if let Some(pos) = app.active_tag_filters.iter().position(|t| t == &tag_name) {
                    app.active_tag_filters.remove(pos);
                } else {
                    app.active_tag_filters.push(tag_name);
                }
                app.apply_filter();
                app.selected = 0;
                if app.active_tag_filters.is_empty() {
                    app.status_message = None;
                } else {
                    app.status_message = Some(format!("Filtered by: {}", app.active_tag_filters.join(", ")));
                }
            }
        }
        KeyCode::Esc => {
            app.active_tag_filters.clear();
            app.apply_filter();
            app.selected = 0;
            app.focus = Focus::NoteList;
            app.mode = Mode::Normal;
            app.status_message = None;
        }
        KeyCode::Tab => {
            app.focus = Focus::Preview;
            app.mode = Mode::Normal;
            app.preview_scroll = 0;
        }
        KeyCode::Char(':') => {
            app.focus = Focus::NoteList;
            app.mode = Mode::Command;
            app.status_message = None;
        }
        _ => {}
    }
    Ok(())
}

fn handle_search(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.search_query.clear();
            app.apply_filter();
            app.selected = 0;
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            app.mode = Mode::Normal;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.apply_filter();
            app.selected = 0;
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.apply_filter();
            app.selected = 0;
        }
        _ => {}
    }
    Ok(())
}

fn handle_command(
    app: &mut App,
    key: KeyEvent,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Char('o') => {
            app.mode = Mode::Normal;
            open_selected_note(app, terminal)?;
        }
        KeyCode::Char('a') => {
            app.input_buffer.clear();
            app.tags_buffer.clear();
            app.mode = Mode::AddNoteName;
        }
        KeyCode::Char('t') => {
            app.input_buffer.clear();
            app.mode = Mode::EditTagsAdd;
        }
        KeyCode::Char('s') => {
            app.mode = Mode::Normal;
            summarize_selected_note(app)?;
        }
        _ => {}
    }
    Ok(())
}

fn handle_add_note_name(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            let name = app.input_buffer.trim().to_string();
            if let Err(e) = utils::validate_name(&name) {
                app.status_message = Some(format!("Invalid name: {}", e));
                return Ok(());
            }
            if let Some(_) = db::get_note(&app.conn, &name)? {
                app.status_message = Some(format!("Note '{}' already exists", name));
                return Ok(());
            }
            app.tags_buffer.clear();
            app.mode = Mode::AddNoteTags;
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
    Ok(())
}

fn handle_add_note_tags(
    app: &mut App,
    key: KeyEvent,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            let tags: Vec<String> = app
                .tags_buffer
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            if !tags.is_empty() {
                if let Err(e) = utils::validate_tags(&tags) {
                    app.status_message = Some(format!("Invalid tags: {}", e));
                    return Ok(());
                }
            }
            let name = app.input_buffer.clone();

            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;

            let contents = utils::get_user_input(&name);

            enable_raw_mode()?;
            execute!(terminal.backend_mut(), EnterAlternateScreen)?;
            terminal.hide_cursor()?;
            terminal.clear()?;

            match contents {
                Ok(contents) => {
                    db::insert_note(&app.conn, &name, &contents, &tags)?;
                    app.refresh_notes()?;
                    app.status_message = Some(format!("Note '{}' created", name));
                }
                Err(e) => {
                    app.status_message = Some(format!("Error: {}", e));
                }
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Backspace => {
            app.tags_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.tags_buffer.push(c);
        }
        _ => {}
    }
    Ok(())
}

fn handle_edit_tags(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Tab => {
            app.mode = match app.mode {
                Mode::EditTagsAdd => Mode::EditTagsRemove,
                _ => Mode::EditTagsAdd,
            };
        }
        KeyCode::Enter => {
            let selected_note = match app.selected_note() {
                Some(n) => n.title.clone(),
                None => {
                    app.status_message = Some("No note selected".to_string());
                    app.mode = Mode::Normal;
                    return Ok(());
                }
            };
            let new_tags: Vec<String> = app
                .input_buffer
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            if new_tags.is_empty() {
                app.status_message = Some("No tags provided".to_string());
                app.mode = Mode::Normal;
                return Ok(());
            }
            if let Err(e) = utils::validate_tags(&new_tags) {
                app.status_message = Some(format!("Invalid tags: {}", e));
                return Ok(());
            }

            let (id, mut existing_tags) = match db::get_tags_and_id(&app.conn, &selected_note)? {
                Some(v) => v,
                None => {
                    app.status_message = Some("Note not found".to_string());
                    app.mode = Mode::Normal;
                    return Ok(());
                }
            };

            let is_add = app.mode == Mode::EditTagsAdd;
            if is_add {
                for tag in &new_tags {
                    if !existing_tags.contains(tag) {
                        existing_tags.push(tag.clone());
                    }
                }
            } else {
                existing_tags.retain(|t| !new_tags.contains(t));
            }

            db::update_tags(&app.conn, id, &existing_tags)?;
            app.refresh_notes()?;
            let action = if is_add { "added to" } else { "removed from" };
            app.status_message = Some(format!("Tags {} '{}'", action, selected_note));
            app.mode = Mode::Normal;
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
    Ok(())
}

fn open_selected_note(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> Result<()> {
    let (title, id, old_contents) = match app.selected_note() {
        Some(n) => (n.title.clone(), n.id, n.note.clone()),
        None => {
            app.status_message = Some("No note selected".to_string());
            return Ok(());
        }
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    let result = utils::get_user_input_with_contents(&title, &old_contents);

    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.hide_cursor()?;
    terminal.clear()?;

    match result {
        Ok(new_contents) => {
            if new_contents != old_contents {
                db::update_note(&app.conn, id, &new_contents)?;
                db::mark_summary_stale(&app.conn, id)?;
                app.refresh_notes()?;
                app.status_message = Some(format!("Note '{}' updated", title));
                // Clear displayed summary since content changed
                if app.showing_summary {
                    app.showing_summary = false;
                    app.summary_content = None;
                    app.summary_stale = false;
                    app.summary_force_regen = false;
                }
            }
        }
        Err(e) => {
            app.status_message = Some(format!("Error: {}", e));
        }
    }
    Ok(())
}

fn summarize_selected_note(app: &mut App) -> Result<()> {
    let note = match app.selected_note() {
        Some(n) => n.clone(),
        None => {
            app.status_message = Some("No note selected".to_string());
            return Ok(());
        }
    };

    // If already showing a stale summary, second :s forces regen
    if app.showing_summary && app.summary_stale && !app.summary_force_regen {
        app.summary_force_regen = true;
    }

    // Check cache
    if !app.summary_force_regen {
        if let Some((cached, stale)) = db::get_summary(&app.conn, note.id)? {
            app.showing_summary = true;
            app.summary_stale = stale;
            app.summary_force_regen = false;
            if stale {
                app.summary_content = Some(cached);
                app.status_message = Some("Summary may be outdated. Press :s again to regenerate.".to_string());
            } else {
                app.summary_content = Some(cached);
                app.status_message = None;
            }
            return Ok(());
        }
    }

    // Generate new summary
    app.status_message = Some("Generating summary...".to_string());

    match llm::summarize_note(&note.title, &note.note) {
        Ok(summary) => {
            db::set_summary(&app.conn, note.id, &summary)?;
            app.showing_summary = true;
            app.summary_content = Some(summary);
            app.summary_stale = false;
            app.summary_force_regen = false;
            app.status_message = None;
        }
        Err(e) => {
            app.status_message = Some(format!("Summary error: {}", e));
        }
    }
    Ok(())
}
