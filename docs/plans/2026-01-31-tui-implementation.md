# Scrap TUI Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the fzf-based find with an interactive TUI using ratatui, featuring a note list, markdown preview, search, and command mode.

**Architecture:** New `src/tui/` module with app state, UI rendering, event handling, and markdown conversion. The TUI calls existing `db.rs` functions. Terminal is suspended/restored when launching the editor. Main entry point routes `scrap` (no args) and `scrap find` to the TUI.

**Tech Stack:** ratatui + crossterm for TUI, pulldown-cmark for markdown rendering, existing rusqlite DB layer.

---

### Task 1: Add Dependencies

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add ratatui, crossterm, pulldown-cmark to Cargo.toml**

Add to `[dependencies]`:
```toml
ratatui = { version = "0.29", features = ["crossterm"] }
crossterm = "0.28"
pulldown-cmark = "0.12"
```

**Step 2: Run `cargo check` to verify deps resolve**

Run: `cargo check`
Expected: Compiles with no errors.

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "feat: add ratatui, crossterm, pulldown-cmark deps for TUI"
```

---

### Task 2: Add `list_notes` to db.rs

**Files:**
- Modify: `src/db.rs`

**Step 1: Add a `NoteEntry` struct and `list_notes` function**

Add to `db.rs`:
```rust
#[derive(Clone)]
pub struct NoteEntry {
    pub id: i64,
    pub title: String,
    pub note: String,
    pub tags: Vec<String>,
    pub updated_at: String,
}

pub fn list_notes(conn: &Connection) -> Result<Vec<NoteEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, note, tags, updated_at FROM notes ORDER BY updated_at DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        let tags_str: String = row.get(3)?;
        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
        Ok(NoteEntry {
            id: row.get(0)?,
            title: row.get(1)?,
            note: row.get(2)?,
            tags,
            updated_at: row.get(4)?,
        })
    })?;
    let mut notes = Vec::new();
    for row in rows {
        notes.push(row?);
    }
    Ok(notes)
}
```

**Step 2: Run `cargo check`**

Run: `cargo check`
Expected: Compiles with no errors.

**Step 3: Commit**

```bash
git add src/db.rs
git commit -m "feat: add list_notes and NoteEntry to db module"
```

---

### Task 3: Create TUI module scaffold with app state

**Files:**
- Create: `src/tui/mod.rs`
- Create: `src/tui/ui.rs`
- Create: `src/tui/events.rs`
- Create: `src/tui/markdown.rs`
- Modify: `src/main.rs`

**Step 1: Create `src/tui/mod.rs` with app state and main loop**

```rust
pub mod events;
pub mod markdown;
pub mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;

use crate::db::{self, NoteEntry};

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Search,
    Command,
    AddNoteName,
    AddNoteTags,
    EditTagsAdd,
    EditTagsRemove,
}

pub struct App {
    pub notes: Vec<NoteEntry>,
    pub filtered_notes: Vec<usize>,
    pub selected: usize,
    pub mode: Mode,
    pub search_query: String,
    pub input_buffer: String,
    pub tags_buffer: String,
    pub status_message: Option<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new(notes: Vec<NoteEntry>) -> Self {
        let filtered_notes: Vec<usize> = (0..notes.len()).collect();
        Self {
            notes,
            filtered_notes,
            selected: 0,
            mode: Mode::Normal,
            search_query: String::new(),
            input_buffer: String::new(),
            tags_buffer: String::new(),
            status_message: None,
            should_quit: false,
        }
    }

    pub fn selected_note(&self) -> Option<&NoteEntry> {
        self.filtered_notes
            .get(self.selected)
            .and_then(|&idx| self.notes.get(idx))
    }

    pub fn refresh_notes(&mut self) -> Result<()> {
        let conn = db::get_db()?;
        self.notes = db::list_notes(&conn)?;
        self.apply_filter();
        if self.selected >= self.filtered_notes.len() && !self.filtered_notes.is_empty() {
            self.selected = self.filtered_notes.len() - 1;
        }
        Ok(())
    }

    pub fn apply_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        if query.is_empty() {
            self.filtered_notes = (0..self.notes.len()).collect();
        } else {
            self.filtered_notes = self
                .notes
                .iter()
                .enumerate()
                .filter(|(_, n)| {
                    n.title.to_lowercase().contains(&query)
                        || n.tags.iter().any(|t| t.to_lowercase().contains(&query))
                })
                .map(|(i, _)| i)
                .collect();
        }
    }

    pub fn move_selection(&mut self, delta: i32) {
        if self.filtered_notes.is_empty() {
            return;
        }
        let len = self.filtered_notes.len() as i32;
        let new = (self.selected as i32 + delta).rem_euclid(len);
        self.selected = new as usize;
    }
}

pub fn run() -> Result<()> {
    let conn = db::get_db()?;
    let notes = db::list_notes(&conn)?;
    drop(conn);

    let mut app = App::new(notes);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            events::handle_key(app, key, terminal)?;
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
```

**Step 2: Create `src/tui/events.rs` with key handling**

```rust
use anyhow::Result;
use crossterm::{
    event::{KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;

use super::{App, Mode};
use crate::{db, utils};

pub fn handle_key(
    app: &mut App,
    key: KeyEvent,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    match app.mode {
        Mode::Normal => handle_normal(app, key, terminal),
        Mode::Search => handle_search(app, key),
        Mode::Command => handle_command(app, key, terminal),
        Mode::AddNoteeName => handle_add_note_name(app, key),
        Mode::AddNoteTags => handle_add_note_tags(app, key, terminal),
        Mode::EditTagsAdd | Mode::EditTagsRemove => handle_edit_tags(app, key),
    }
}

fn handle_normal(
    app: &mut App,
    key: KeyEvent,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.move_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_selection(-1),
        KeyCode::Char('/') => {
            app.mode = Mode::Search;
            app.search_query.clear();
            app.status_message = None;
        }
        KeyCode::Char(':') => {
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
            app.mode = Mode::Normal;
            app.search_query.clear();
            app.apply_filter();
            app.selected = 0;
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
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
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
            app.mode = Mode::AddNoteName;
            app.input_buffer.clear();
            app.tags_buffer.clear();
        }
        KeyCode::Char('t') => {
            if app.selected_note().is_some() {
                app.mode = Mode::EditTagsAdd;
                app.input_buffer.clear();
            }
        }
        KeyCode::Char('s') => {
            app.mode = Mode::Normal;
            app.status_message = Some("Summarize: coming soon".to_string());
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
            if let Err(e) = utils::validate_name(&app.input_buffer) {
                app.status_message = Some(e.to_string());
            } else {
                let conn = db::get_db()?;
                if db::get_note(&conn, &app.input_buffer)?.is_some() {
                    app.status_message = Some(format!("Note '{}' already exists.", app.input_buffer));
                } else {
                    app.mode = Mode::AddNoteTags;
                    app.tags_buffer.clear();
                }
            }
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
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            let name = app.input_buffer.clone();
            let tags: Vec<String> = app
                .tags_buffer
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            if let Err(e) = utils::validate_tags(&tags) {
                app.status_message = Some(e.to_string());
                return Ok(());
            }
            // Suspend TUI, launch editor, restore TUI
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;

            let result = utils::get_user_input(&name);

            enable_raw_mode()?;
            execute!(terminal.backend_mut(), EnterAlternateScreen)?;
            terminal.hide_cursor()?;
            terminal.clear()?;

            match result {
                Ok(contents) => {
                    let conn = db::get_db()?;
                    db::insert_note(&conn, &name, &contents, &tags)?;
                    app.refresh_notes()?;
                    app.status_message = Some(format!("Note '{}' created.", name));
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
            app.mode = if app.mode == Mode::EditTagsAdd {
                Mode::EditTagsRemove
            } else {
                Mode::EditTagsAdd
            };
        }
        KeyCode::Enter => {
            let tags: Vec<String> = app
                .input_buffer
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            if tags.is_empty() {
                app.status_message = Some("No tags provided.".to_string());
                app.mode = Mode::Normal;
                return Ok(());
            }
            if let Err(e) = utils::validate_tags(&tags) {
                app.status_message = Some(e.to_string());
                return Ok(());
            }
            if let Some(note) = app.selected_note() {
                let title = note.title.clone();
                let conn = db::get_db()?;
                if let Some((id, mut existing)) = db::get_tags_and_id(&conn, &title)? {
                    if app.mode == Mode::EditTagsAdd {
                        for tag in &tags {
                            if !existing.contains(tag) {
                                existing.push(tag.clone());
                            }
                        }
                        app.status_message = Some(format!("Tags added to '{}'.", title));
                    } else {
                        existing.retain(|t| !tags.contains(t));
                        app.status_message = Some(format!("Tags removed from '{}'.", title));
                    }
                    db::update_tags(&conn, id, &existing)?;
                    app.refresh_notes()?;
                }
            }
            app.input_buffer.clear();
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
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    let note = match app.selected_note() {
        Some(n) => n.clone(),
        None => return Ok(()),
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    let result = utils::get_user_input_with_contents(&note.title, &note.note);

    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.hide_cursor()?;
    terminal.clear()?;

    match result {
        Ok(new_contents) => {
            if new_contents != note.note {
                let conn = db::get_db()?;
                db::update_note(&conn, note.id, &new_contents)?;
                app.refresh_notes()?;
                app.status_message = Some(format!("Note '{}' updated.", note.title));
            } else {
                app.status_message = Some("No changes made.".to_string());
            }
        }
        Err(e) => {
            app.status_message = Some(format!("Error: {}", e));
        }
    }
    Ok(())
}
```

**Step 3: Create `src/tui/markdown.rs`**

```rust
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn render_markdown(input: &str) -> Vec<Line<'static>> {
    let options = Options::ENABLE_STRIKETHROUGH;
    let parser = Parser::new_ext(input, options);

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default()];
    let mut in_code_block = false;

    for event in parser {
        match event {
            Event::Start(tag) => {
                let style = match &tag {
                    Tag::Heading { level, .. } => {
                        let color = match *level {
                            pulldown_cmark::HeadingLevel::H1 => Color::Cyan,
                            pulldown_cmark::HeadingLevel::H2 => Color::Green,
                            _ => Color::Yellow,
                        };
                        Style::default().fg(color).add_modifier(Modifier::BOLD)
                    }
                    Tag::Strong => Style::default().add_modifier(Modifier::BOLD),
                    Tag::Emphasis => Style::default().add_modifier(Modifier::ITALIC),
                    Tag::CodeBlock(_) => {
                        in_code_block = true;
                        Style::default().fg(Color::Gray)
                    }
                    Tag::List(_) => Style::default(),
                    Tag::Item => {
                        current_spans.push(Span::styled("  • ", Style::default().fg(Color::Cyan)));
                        Style::default()
                    }
                    _ => Style::default(),
                };
                style_stack.push(style);
            }
            Event::End(tag_end) => {
                style_stack.pop();
                match tag_end {
                    TagEnd::Heading(_) | TagEnd::Paragraph | TagEnd::Item => {
                        lines.push(Line::from(std::mem::take(&mut current_spans)));
                    }
                    TagEnd::CodeBlock => {
                        in_code_block = false;
                        lines.push(Line::from(std::mem::take(&mut current_spans)));
                    }
                    TagEnd::List(_) => {
                        lines.push(Line::from(""));
                    }
                    _ => {}
                }
            }
            Event::Text(text) => {
                let style = style_stack.last().copied().unwrap_or_default();
                if in_code_block {
                    for line_text in text.split('\n') {
                        if !current_spans.is_empty() {
                            lines.push(Line::from(std::mem::take(&mut current_spans)));
                        }
                        current_spans.push(Span::styled(
                            format!("  {}", line_text),
                            Style::default().fg(Color::Gray),
                        ));
                    }
                } else {
                    current_spans.push(Span::styled(text.to_string(), style));
                }
            }
            Event::Code(code) => {
                current_spans.push(Span::styled(
                    format!("`{}`", code),
                    Style::default().fg(Color::Magenta),
                ));
            }
            Event::SoftBreak | Event::HardBreak => {
                lines.push(Line::from(std::mem::take(&mut current_spans)));
            }
            _ => {}
        }
    }

    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "(empty note)",
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines
}
```

**Step 4: Create `src/tui/ui.rs`**

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::{markdown, App, Mode};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(f.area());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[0]);

    draw_note_list(f, app, main_chunks[0]);
    draw_preview(f, app, main_chunks[1]);
    draw_status_bar(f, app, chunks[1]);

    match app.mode {
        Mode::AddNoteName => draw_input_modal(f, "New Note - Name", &app.input_buffer),
        Mode::AddNoteTags => draw_input_modal(f, "New Note - Tags (space-separated)", &app.tags_buffer),
        Mode::EditTagsAdd => draw_input_modal(f, "Add Tags (space-separated, Tab to switch to remove)", &app.input_buffer),
        Mode::EditTagsRemove => draw_input_modal(f, "Remove Tags (space-separated, Tab to switch to add)", &app.input_buffer),
        _ => {}
    }
}

fn draw_note_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .filtered_notes
        .iter()
        .enumerate()
        .map(|(i, &idx)| {
            let note = &app.notes[idx];
            let tags = if note.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", note.tags.join(", "))
            };
            let style = if i == app.selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(&note.title, style),
                Span::styled(tags, if i == app.selected {
                    style
                } else {
                    Style::default().fg(Color::DarkGray)
                }),
            ]))
        })
        .collect();

    let title = if app.mode == Mode::Search {
        format!("Notes [/{}]", app.search_query)
    } else {
        format!("Notes ({})", app.filtered_notes.len())
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title),
    );

    f.render_widget(list, area);
}

fn draw_preview(f: &mut Frame, app: &App, area: Rect) {
    let (title, content) = match app.selected_note() {
        Some(note) => (
            note.title.clone(),
            markdown::render_markdown(&note.note),
        ),
        None => (
            "Preview".to_string(),
            vec![Line::from(Span::styled(
                "No note selected",
                Style::default().fg(Color::DarkGray),
            ))],
        ),
    };

    let preview = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false });

    f.render_widget(preview, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let (mode_text, help_text) = match app.mode {
        Mode::Normal => (
            Span::styled(" NORMAL ", Style::default().fg(Color::Black).bg(Color::Cyan)),
            " q quit │ / search │ : commands │ j/k navigate ",
        ),
        Mode::Search => (
            Span::styled(" SEARCH ", Style::default().fg(Color::Black).bg(Color::Yellow)),
            " Type to filter │ Enter accept │ Esc cancel ",
        ),
        Mode::Command => (
            Span::styled(" COMMAND ", Style::default().fg(Color::Black).bg(Color::Magenta)),
            " o open │ a add │ t edit tags │ s summarize │ Esc cancel ",
        ),
        Mode::AddNoteName | Mode::AddNoteTags => (
            Span::styled(" ADD NOTE ", Style::default().fg(Color::Black).bg(Color::Green)),
            " Enter confirm │ Esc cancel ",
        ),
        Mode::EditTagsAdd | Mode::EditTagsRemove => (
            Span::styled(" EDIT TAGS ", Style::default().fg(Color::Black).bg(Color::Green)),
            " Enter confirm │ Tab switch add/remove │ Esc cancel ",
        ),
    };

    let status = if let Some(msg) = &app.status_message {
        Line::from(vec![
            mode_text,
            Span::raw(" "),
            Span::styled(msg.as_str(), Style::default().fg(Color::Yellow)),
        ])
    } else {
        Line::from(vec![
            mode_text,
            Span::styled(help_text, Style::default().fg(Color::DarkGray)),
        ])
    };

    let bar = Paragraph::new(status);
    f.render_widget(bar, area);
}

fn draw_input_modal(f: &mut Frame, title: &str, input: &str) {
    let area = centered_rect(50, 5, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(Color::Cyan));

    let text = Paragraph::new(format!("> {}", input))
        .block(block);

    f.render_widget(text, area);
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
```

**Step 5: Update `src/main.rs` to wire in the TUI**

Change `main.rs` to add `mod tui;` and route no-args and `Find` to the TUI:

```rust
mod commands;
mod db;
mod tui;
mod utils;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "scrap", about = "A CLI note-taking app")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new note
    Add {
        /// Name of the note
        name: String,
        /// Tags for the note
        tags: Vec<String>,
    },
    /// Delete a note
    Delete {
        /// Name of the note to delete
        name: String,
    },
    /// Find and browse notes (launches TUI)
    Find {
        /// Optional search query
        query: Option<String>,
    },
    /// Open and edit an existing note
    Open {
        /// Name of the note to open
        name: String,
    },
    /// Add or remove tags from a note
    EditTag {
        /// Add tags
        #[arg(long)]
        add: bool,
        /// Delete tags
        #[arg(long)]
        delete: bool,
        /// Name of the note
        name: String,
        /// Tags to add or remove
        tags: Vec<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None | Some(Commands::Find { query: None }) => tui::run(),
        Some(Commands::Find { query: Some(q) }) => {
            // Launch TUI with initial search pre-filled
            // For now just launch TUI - search prefill is a future enhancement
            let _ = q;
            tui::run()
        }
        Some(Commands::Add { name, tags }) => commands::add::run(&name, &tags),
        Some(Commands::Delete { name }) => commands::delete::run(&name),
        Some(Commands::Open { name }) => commands::open::run(&name),
        Some(Commands::EditTag {
            add,
            delete,
            name,
            tags,
        }) => commands::edit_tag::run(&name, &tags, add, delete),
    }
}
```

**Step 6: Run `cargo build`**

Run: `cargo build`
Expected: Compiles with zero errors and zero warnings.

**Step 7: Commit**

```bash
git add src/tui/ src/main.rs
git commit -m "feat: add ratatui TUI with note list, preview, search, and command mode"
```

---

### Task 4: Manual Verification

**Step 1: Run the TUI**

Run: `cargo run`
Verify: TUI launches, notes listed on left, preview on right.

**Step 2: Test navigation**

Press j/k and arrow keys — selection moves, preview updates.

**Step 3: Test search**

Press `/`, type part of a note title — list filters. Esc clears.

**Step 4: Test open**

Press `:` then `o` — editor opens with note content. Save and quit — TUI returns.

**Step 5: Test add**

Press `:` then `a` — modal for name appears. Enter name, Enter, tags modal, Enter — editor opens. Save and quit — note appears in list.

**Step 6: Test edit tags**

Press `:` then `t` — tag input modal. Type tags, Enter — tags update. Tab switches between add/remove mode.

**Step 7: Test quit**

Press `q` — app exits cleanly to terminal.
