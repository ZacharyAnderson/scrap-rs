use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use super::{markdown, App, Focus, Mode, PreviewTab};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(f.area());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[0]);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(main_chunks[0]);

    draw_note_list(f, app, left_chunks[0]);
    draw_tag_panel(f, app, left_chunks[1]);
    draw_preview(f, app, main_chunks[1]);
    draw_status_bar(f, app, chunks[1]);

    match app.mode {
        Mode::Search => draw_search_popup(f, app),
        Mode::AddNoteName | Mode::AddNoteTags | Mode::EditTagsAdd | Mode::EditTagsRemove => {
            draw_input_modal(f, app);
        }
        _ => {}
    }
}

fn draw_note_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .filtered_notes
        .iter()
        .map(|&idx| {
            let note = &app.notes[idx];
            ListItem::new(note.title.clone())
        })
        .collect();

    let title = if app.active_tag_filters.is_empty() {
        format!("Notes ({})", app.filtered_notes.len())
    } else {
        format!("Notes [{}] ({})", app.active_tag_filters.join(", "), app.filtered_notes.len())
    };

    let border_style = if app.focus == Focus::NoteList && (app.mode == Mode::Normal || app.mode == Mode::TagBrowse) {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border_style),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    if !app.filtered_notes.is_empty() {
        state.select(Some(app.selected));
    }

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_tag_panel(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .visible_tags
        .iter()
        .map(|tag| {
            let active = app.active_tag_filters.contains(&tag.name);
            let text = format!("{} ({})", tag.name, tag.count);
            if active {
                ListItem::new(text).style(
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ListItem::new(text)
            }
        })
        .collect();

    let border_style = if app.focus == Focus::TagPanel {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Tags")
                .border_style(border_style),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    if !app.visible_tags.is_empty() && app.focus == Focus::TagPanel {
        state.select(Some(app.selected_tag));
    }

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_preview(f: &mut Frame, app: &App, area: Rect) {
    let note_title = app
        .selected_note()
        .map(|n| n.title.clone())
        .unwrap_or_default();

    let tab_label = match app.preview_tab {
        PreviewTab::Note => "Note",
        PreviewTab::Summary => "Summary",
    };

    let is_focused = app.focus == Focus::Preview;

    // Decide what content to show based on preview_tab
    let (title, lines, border_style) = match app.preview_tab {
        PreviewTab::Summary if app.summary_content.is_some() || app.showing_summary => {
            let title = if app.summary_stale {
                format!("{} [{}] (outdated)", note_title, tab_label)
            } else {
                format!("{} [{}]", note_title, tab_label)
            };
            let lines = match &app.summary_content {
                Some(content) => markdown::render_markdown(content),
                None => vec![Line::from("Generating summary...")],
            };
            let border = if is_focused {
                Style::default().fg(Color::Cyan)
            } else if app.summary_stale {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Green)
            };
            (title, lines, border)
        }
        _ => {
            // Note tab (or Summary tab with no summary available — fall back to Note)
            let (title, lines) = match app.selected_note() {
                Some(note) => {
                    let rendered = markdown::render_markdown(&note.note);
                    (format!("{} [{}]", note.title, tab_label), rendered)
                }
                None => ("Preview".to_string(), vec![Line::from("No note selected")]),
            };
            let border = if is_focused {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };
            (title, lines, border)
        }
    };

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border_style),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.preview_scroll, 0));

    f.render_widget(paragraph, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let (mode_text, mode_color) = match app.mode {
        Mode::Normal if app.focus == Focus::Preview => (" PREVIEW ", Color::Cyan),
        Mode::Normal => (" NORMAL ", Color::Cyan),
        Mode::TagBrowse => (" TAGS ", Color::Yellow),
        Mode::Search => (" SEARCH ", Color::Yellow),
        Mode::Command => (" COMMAND ", Color::Magenta),
        Mode::AddNoteName | Mode::AddNoteTags => (" ADD NOTE ", Color::Green),
        Mode::EditTagsAdd => (" EDIT TAGS [+] ", Color::Green),
        Mode::EditTagsRemove => (" EDIT TAGS [-] ", Color::Green),
    };

    let key_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Color::DarkGray);
    let sep = Span::styled("  ", desc_style);

    let help_spans: Vec<Span> = match &app.status_message {
        Some(msg) => vec![Span::raw(" "), Span::styled(msg.clone(), Style::default().fg(Color::Yellow))],
        None => {
            let bindings: &[(&str, &str)] = match app.mode {
                Mode::Normal if app.focus == Focus::Preview => &[("j/k", "scroll"), ("Tab", "toggle view"), ("Esc", "back"), (":", "command")],
                Mode::Normal => &[("q", "quit"), ("/", "search"), (":", "command"), ("Tab", "tags")],
                Mode::TagBrowse => &[("Enter", "filter"), ("Esc", "clear & back"), ("Tab", "notes"), (":", "command")],
                Mode::Search => &[("Enter", "confirm"), ("Esc", "cancel")],
                Mode::Command => &[("o", "open"), ("a", "add"), ("t", "tags"), ("s", "summarize"), ("Esc", "cancel")],
                Mode::AddNoteName => &[("Enter", "next"), ("Esc", "cancel")],
                Mode::AddNoteTags => &[("Enter", "open editor"), ("Esc", "cancel")],
                Mode::EditTagsAdd | Mode::EditTagsRemove => &[("Enter", "apply"), ("Tab", "toggle add/remove"), ("Esc", "cancel")],
            };
            let mut spans = vec![Span::raw(" ")];
            for (i, (key, desc)) in bindings.iter().enumerate() {
                if i > 0 {
                    spans.push(sep.clone());
                }
                spans.push(Span::styled(*key, key_style));
                spans.push(Span::styled(format!(" {}", desc), desc_style));
            }
            spans
        }
    };

    let mut bar_spans = vec![
        Span::styled(
            mode_text,
            Style::default()
                .fg(Color::Black)
                .bg(mode_color)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    bar_spans.extend(help_spans);
    let bar = Line::from(bar_spans);

    let paragraph = Paragraph::new(bar);
    f.render_widget(paragraph, area);
}

fn draw_input_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 5, f.area());

    let (title, input) = match app.mode {
        Mode::AddNoteName => ("Add Note - Name", &app.input_buffer),
        Mode::AddNoteTags => ("Add Note - Tags (space-separated)", &app.tags_buffer),
        Mode::EditTagsAdd => ("Edit Tags [Add] (space-separated)", &app.input_buffer),
        Mode::EditTagsRemove => ("Edit Tags [Remove] (space-separated)", &app.input_buffer),
        _ => return,
    };

    f.render_widget(Clear, area);

    let block = Block::default().borders(Borders::ALL).title(title);
    let paragraph = Paragraph::new(format!("> {}", input)).block(block);
    f.render_widget(paragraph, area);
}

fn draw_search_popup(f: &mut Frame, app: &App) {
    let area = f.area();
    let width = (area.width / 2).max(30).min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(3)) / 2;
    let popup = Rect::new(x, y, width, 3);

    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Search ")
        .border_style(Style::default().fg(Color::Yellow));
    let input = Paragraph::new(format!("/{}", app.search_query)).block(block);
    f.render_widget(input, popup);
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
