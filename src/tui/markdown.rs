use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::prelude::*;

pub fn render_markdown(input: &str) -> Vec<Line<'static>> {
    if input.trim().is_empty() {
        return vec![Line::from(Span::styled(
            "(empty note)",
            Style::default().fg(Color::DarkGray),
        ))];
    }

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    let parser = Parser::new_ext(input, options);

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default()];
    let mut in_code_block = false;
    let mut pending_list_marker = false;
    let mut link_url: Option<String> = None;
    let mut in_table = false;
    let mut table_row_cells: Vec<String> = Vec::new();
    let mut current_cell = String::new();
    let mut table_col_count = 0;
    const COL_WIDTH: usize = 14;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    let style = match level {
                        pulldown_cmark::HeadingLevel::H1 => {
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                        }
                        pulldown_cmark::HeadingLevel::H2 => {
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                        }
                        _ => Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    };
                    style_stack.push(style);
                }
                Tag::Strong => {
                    let base = *style_stack.last().unwrap_or(&Style::default());
                    style_stack.push(base.add_modifier(Modifier::BOLD));
                }
                Tag::Emphasis => {
                    let base = *style_stack.last().unwrap_or(&Style::default());
                    style_stack.push(base.add_modifier(Modifier::ITALIC));
                }
                Tag::CodeBlock(_) => {
                    in_code_block = true;
                }
                Tag::Item => {
                    pending_list_marker = true;
                }
                Tag::Link { dest_url, .. } => {
                    link_url = Some(dest_url.to_string());
                    let base = *style_stack.last().unwrap_or(&Style::default());
                    style_stack.push(base.fg(Color::Blue).add_modifier(Modifier::UNDERLINED));
                }
                Tag::Strikethrough => {
                    let base = *style_stack.last().unwrap_or(&Style::default());
                    style_stack.push(base.add_modifier(Modifier::CROSSED_OUT));
                }
                Tag::Table(_) => {
                    in_table = true;
                }
                Tag::TableHead => {}
                Tag::TableRow => {
                    table_row_cells.clear();
                }
                Tag::TableCell => {
                    current_cell.clear();
                }
                Tag::Paragraph => {}
                Tag::List(_) => {}
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Heading(_) => {
                    style_stack.pop();
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                    lines.push(Line::from(""));
                }
                TagEnd::Paragraph => {
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                    lines.push(Line::from(""));
                }
                TagEnd::Item => {
                    pending_list_marker = false;
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                }
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                    lines.push(Line::from(""));
                }
                TagEnd::Strong | TagEnd::Emphasis | TagEnd::Strikethrough => {
                    style_stack.pop();
                }
                TagEnd::Link => {
                    style_stack.pop();
                    if let Some(url) = link_url.take() {
                        current_spans.push(Span::styled(
                            format!(" ({})", url),
                            Style::default().fg(Color::DarkGray),
                        ));
                    }
                }
                TagEnd::TableHead => {
                    // Render header row with top border
                    if !table_row_cells.is_empty() {
                        table_col_count = table_row_cells.len();
                        let border_style = Style::default().fg(Color::DarkGray);

                        // Top border: ┌──────┬──────┐
                        let mut top = String::from("┌");
                        for i in 0..table_col_count {
                            top.push_str(&"─".repeat(COL_WIDTH));
                            if i < table_col_count - 1 {
                                top.push('┬');
                            }
                        }
                        top.push('┐');
                        lines.push(Line::from(Span::styled(top, border_style)));

                        // Header row: │ Name │ Age │
                        let mut row_spans: Vec<Span<'static>> = Vec::new();
                        row_spans.push(Span::styled("│", border_style));
                        for cell in table_row_cells.iter() {
                            let padded = format!("{:^width$}", cell, width = COL_WIDTH);
                            row_spans.push(Span::styled(
                                padded,
                                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                            ));
                            row_spans.push(Span::styled("│", border_style));
                        }
                        lines.push(Line::from(row_spans));

                        // Middle border: ├──────┼──────┤
                        let mut mid = String::from("├");
                        for i in 0..table_col_count {
                            mid.push_str(&"─".repeat(COL_WIDTH));
                            if i < table_col_count - 1 {
                                mid.push('┼');
                            }
                        }
                        mid.push('┤');
                        lines.push(Line::from(Span::styled(mid, border_style)));

                        table_row_cells.clear();
                    }
                }
                TagEnd::TableRow => {
                    // Render body row: │ Alice │ 30 │
                    if !table_row_cells.is_empty() {
                        let border_style = Style::default().fg(Color::DarkGray);
                        let mut row_spans: Vec<Span<'static>> = Vec::new();
                        row_spans.push(Span::styled("│", border_style));
                        for cell in table_row_cells.iter() {
                            let padded = format!("{:^width$}", cell, width = COL_WIDTH);
                            row_spans.push(Span::raw(padded));
                            row_spans.push(Span::styled("│", border_style));
                        }
                        lines.push(Line::from(row_spans));
                        table_row_cells.clear();
                    }
                }
                TagEnd::TableCell => {
                    table_row_cells.push(current_cell.clone());
                    current_cell.clear();
                }
                TagEnd::Table => {
                    // Bottom border: └──────┴──────┘
                    if table_col_count > 0 {
                        let border_style = Style::default().fg(Color::DarkGray);
                        let mut bottom = String::from("└");
                        for i in 0..table_col_count {
                            bottom.push_str(&"─".repeat(COL_WIDTH));
                            if i < table_col_count - 1 {
                                bottom.push('┴');
                            }
                        }
                        bottom.push('┘');
                        lines.push(Line::from(Span::styled(bottom, border_style)));
                    }
                    in_table = false;
                    table_col_count = 0;
                    lines.push(Line::from(""));
                }
                _ => {}
            },
            Event::Text(text) => {
                if pending_list_marker {
                    pending_list_marker = false;
                    current_spans.push(Span::styled(
                        "  • ",
                        Style::default().fg(Color::Cyan),
                    ));
                }
                let text = text.to_string();
                if in_table {
                    current_cell.push_str(&text);
                } else if in_code_block {
                    let style = Style::default().fg(Color::Gray);
                    for line_text in text.split('\n') {
                        if !current_spans.is_empty() {
                            lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                        }
                        current_spans
                            .push(Span::styled(format!("    {}", line_text), style));
                    }
                } else {
                    let style = *style_stack.last().unwrap_or(&Style::default());
                    current_spans.push(Span::styled(text, style));
                }
            }
            Event::Code(code) => {
                let text = format!("`{}`", code);
                current_spans.push(Span::styled(
                    text,
                    Style::default().fg(Color::Magenta),
                ));
            }
            Event::SoftBreak | Event::HardBreak => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                }
            }
            Event::TaskListMarker(checked) => {
                pending_list_marker = false;
                let marker = if checked { "  ☑ " } else { "  ☐ " };
                let color = if checked { Color::Green } else { Color::Yellow };
                current_spans.push(Span::styled(marker, Style::default().fg(color)));
            }
            _ => {}
        }
    }

    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }

    if lines.is_empty() {
        lines.push(Line::from(""));
    }

    lines
}
