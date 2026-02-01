# Note Summarize Feature Design

## Overview

Add an LLM-powered summarize feature using Anthropic's API. Summaries are cached in the database and displayed in the preview pane. Cached summaries are marked stale when notes are edited.

## API Integration

New module `src/llm.rs`:
- Reads `ANTHROPIC_API_KEY` from environment
- If missing, returns error: "Set ANTHROPIC_API_KEY in your ~/.zshrc"
- HTTP POST to `https://api.anthropic.com/v1/messages` via reqwest (blocking)
- System prompt instructs Claude to return a markdown-formatted summary
- Model: claude-sonnet-4-20250514
- New dependency: reqwest (blocking, json features)

## Database Changes

Two new columns on notes table:
- `summary TEXT` — cached markdown summary
- `summary_stale INTEGER NOT NULL DEFAULT 0` — 1 when note edited after summary generated

New db.rs functions:
- `get_summary(conn, id)` -> `Option<(String, bool)>` (text, stale flag)
- `set_summary(conn, id, summary)` — saves summary, sets stale = 0
- `mark_summary_stale(conn, id)` — sets stale = 1

Migration: ALTER TABLE ADD COLUMN, silently ignore duplicate column errors.

When `:o` saves edited content, call `mark_summary_stale`.

## TUI Integration

`:s` on selected note:
1. No cache: show "Generating summary...", call API, cache, display
2. Cached, not stale: display immediately
3. Cached, stale: display with "⚠ Summary may be outdated (press :s to regenerate)"
4. Second `:s` on stale summary: re-generates

App state additions:
- `showing_summary: bool`
- `summary_content: Option<String>`
- `summary_stale: bool`
- `summary_force_regen: bool`

Clear summary: Esc in normal mode or navigate to different note.

Preview pane renders summary markdown using existing `render_markdown`.
