mod events;
mod markdown;
mod ui;

use std::time::Instant;

use anyhow::Result;
use crossterm::{
    event::{self, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use rusqlite::Connection;

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
    TagBrowse,
    VisualLine,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    NoteList,
    TagPanel,
    Preview,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PreviewTab {
    Note,
    Summary,
}

#[derive(Clone)]
pub struct TagEntry {
    pub name: String,
    pub count: usize,
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
    pub conn: Connection,
    pub focus: Focus,
    pub all_tags: Vec<TagEntry>,
    pub visible_tags: Vec<TagEntry>,
    pub selected_tag: usize,
    pub active_tag_filters: Vec<String>,
    pub showing_summary: bool,
    pub summary_content: Option<String>,
    pub summary_stale: bool,
    pub summary_force_regen: bool,
    pub preview_tab: PreviewTab,
    pub preview_scroll: u16,
    pub preview_content_height: u16,
    pub status_expires: Option<Instant>,
    pub pending_g: bool,
    pub tag_suggestions: Vec<String>,
    pub selected_suggestion: usize,
    pub preview_cursor: usize,
    pub visual_anchor: Option<usize>,
    pub yank_register: Option<String>,
}

impl App {
    pub fn new(conn: Connection, notes: Vec<NoteEntry>) -> Self {
        let filtered_notes: Vec<usize> = (0..notes.len()).collect();
        let all_tags = compute_tags(&notes);
        let visible_tags = all_tags.clone();
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
            conn,
            focus: Focus::NoteList,
            all_tags,
            visible_tags,
            selected_tag: 0,
            active_tag_filters: Vec::new(),
            showing_summary: false,
            summary_content: None,
            summary_stale: false,
            summary_force_regen: false,
            preview_tab: PreviewTab::Note,
            preview_scroll: 0,
            preview_content_height: 0,
            status_expires: None,
            pending_g: false,
            tag_suggestions: Vec::new(),
            selected_suggestion: 0,
            preview_cursor: 0,
            visual_anchor: None,
            yank_register: None,
        }
    }

    pub fn selected_note(&self) -> Option<&NoteEntry> {
        self.filtered_notes
            .get(self.selected)
            .and_then(|&idx| self.notes.get(idx))
    }

    pub fn refresh_notes(&mut self) -> Result<()> {
        self.notes = db::list_notes(&self.conn)?;
        self.all_tags = compute_tags(&self.notes);
        self.apply_filter();
        if self.selected_tag >= self.all_tags.len() && !self.all_tags.is_empty() {
            self.selected_tag = self.all_tags.len() - 1;
        }
        Ok(())
    }

    pub fn apply_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        self.filtered_notes = self
            .notes
            .iter()
            .enumerate()
            .filter(|(_, note)| {
                // Tag filter — note must match at least one selected tag
                if !self.active_tag_filters.is_empty() {
                    if !note.tags.iter().any(|t| self.active_tag_filters.contains(t)) {
                        return false;
                    }
                }
                // Search query
                if query.is_empty() {
                    true
                } else {
                    note.title.to_lowercase().contains(&query)
                        || note.note.to_lowercase().contains(&query)
                        || note.tags.iter().any(|t| t.to_lowercase().contains(&query))
                }
            })
            .map(|(i, _)| i)
            .collect();
        if self.selected >= self.filtered_notes.len() {
            self.selected = 0;
        }
        // Recompute visible tags from filtered notes
        let filtered_notes_ref: Vec<&NoteEntry> = self
            .filtered_notes
            .iter()
            .map(|&i| &self.notes[i])
            .collect();
        self.visible_tags = compute_tags_from_refs(&filtered_notes_ref);
        if self.selected_tag >= self.visible_tags.len() && !self.visible_tags.is_empty() {
            self.selected_tag = self.visible_tags.len() - 1;
        } else if self.visible_tags.is_empty() {
            self.selected_tag = 0;
        }
    }

    pub fn move_selection(&mut self, delta: i32) {
        let len = self.filtered_notes.len();
        if len == 0 {
            return;
        }
        self.selected = ((self.selected as i32 + delta).rem_euclid(len as i32)) as usize;
    }

    pub fn move_tag_selection(&mut self, delta: i32) {
        let len = self.visible_tags.len();
        if len == 0 {
            return;
        }
        self.selected_tag = ((self.selected_tag as i32 + delta).rem_euclid(len as i32)) as usize;
    }

    /// Update tag suggestions based on current input
    pub fn update_tag_suggestions(&mut self, input: &str) {
        // Get the current word being typed (after last space)
        let current_word = input.split_whitespace().last().unwrap_or("");

        if current_word.is_empty() {
            self.tag_suggestions.clear();
            self.selected_suggestion = 0;
            return;
        }

        let current_lower = current_word.to_lowercase();

        // Get tags already entered in the input
        let entered_tags: Vec<&str> = input.split_whitespace().collect();
        let entered_set: std::collections::HashSet<&str> =
            entered_tags.iter().take(entered_tags.len().saturating_sub(1)).copied().collect();

        // Filter all_tags to find matches not already entered
        self.tag_suggestions = self
            .all_tags
            .iter()
            .filter(|t| {
                t.name.to_lowercase().starts_with(&current_lower)
                    && !entered_set.contains(t.name.as_str())
            })
            .take(5)
            .map(|t| t.name.clone())
            .collect();

        // Reset selection if out of bounds
        if self.selected_suggestion >= self.tag_suggestions.len() {
            self.selected_suggestion = 0;
        }
    }

    /// Accept the currently selected tag suggestion
    pub fn accept_tag_suggestion(&mut self, buffer: &mut String) {
        if let Some(suggestion) = self.tag_suggestions.get(self.selected_suggestion) {
            // Remove the partial word and replace with suggestion
            let words: Vec<&str> = buffer.split_whitespace().collect();
            let prefix: String = if words.len() > 1 {
                words[..words.len() - 1].join(" ") + " "
            } else {
                String::new()
            };
            *buffer = format!("{}{} ", prefix, suggestion);
            self.tag_suggestions.clear();
            self.selected_suggestion = 0;
        }
    }

    pub fn move_suggestion_selection(&mut self, delta: i32) {
        let len = self.tag_suggestions.len();
        if len == 0 {
            return;
        }
        self.selected_suggestion = ((self.selected_suggestion as i32 + delta).rem_euclid(len as i32)) as usize;
    }

    /// Get the raw markdown content currently displayed in the preview.
    /// Returns note content or summary content depending on the active preview tab.
    pub fn preview_raw_content(&self) -> Option<String> {
        match self.preview_tab {
            PreviewTab::Summary if self.summary_content.is_some() => {
                self.summary_content.clone()
            }
            _ => self.selected_note().map(|n| n.note.clone()),
        }
    }

    /// Get the lines of raw content for the preview.
    pub fn preview_raw_lines(&self) -> Vec<String> {
        self.preview_raw_content()
            .map(|c| c.lines().map(|l| l.to_string()).collect())
            .unwrap_or_default()
    }
}

fn compute_tags(notes: &[NoteEntry]) -> Vec<TagEntry> {
    let mut map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for note in notes {
        for tag in &note.tags {
            *map.entry(tag.clone()).or_insert(0) += 1;
        }
    }
    let mut tags: Vec<TagEntry> = map
        .into_iter()
        .map(|(name, count)| TagEntry { name, count })
        .collect();
    tags.sort_by(|a, b| b.count.cmp(&a.count).then(a.name.cmp(&b.name)));
    tags
}

fn compute_tags_from_refs(notes: &[&NoteEntry]) -> Vec<TagEntry> {
    let mut map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for note in notes {
        for tag in &note.tags {
            *map.entry(tag.clone()).or_insert(0) += 1;
        }
    }
    let mut tags: Vec<TagEntry> = map
        .into_iter()
        .map(|(name, count)| TagEntry { name, count })
        .collect();
    tags.sort_by(|a, b| b.count.cmp(&a.count).then(a.name.cmp(&b.name)));
    tags
}

pub fn run() -> Result<()> {
    let conn = db::get_db()?;
    let notes = db::list_notes(&conn)?;
    let mut app = App::new(conn, notes);

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let result = run_loop(&mut app, &mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_loop(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> Result<()> {
    loop {
        // Clear expired status messages
        if let Some(expires) = app.status_expires {
            if Instant::now() >= expires {
                app.status_message = None;
                app.status_expires = None;
            }
        }

        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(std::time::Duration::from_millis(250))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    events::handle_key(app, key, terminal)?;
                }
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
