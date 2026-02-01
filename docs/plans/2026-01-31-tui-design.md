# Scrap TUI Design

## Overview

Replace the fzf-based find command with a full TUI built on ratatui. The TUI becomes the main interface when running `scrap` or `scrap find`. CLI subcommands remain for scripting.

## Layout

```
┌─────────────────────┬──────────────────────────────────┐
│  Notes              │  Preview                         │
│                     │                                  │
│  > my-note    [dev] │  ## Hello                        │
│    todo-list  [work]│                                  │
│    ideas      [misc]│  Some markdown content here      │
│                     │                                  │
├─────────────────────┴──────────────────────────────────┤
│  / search  │  : command mode                           │
│  :o open  :a add  :t edit tags  :s summarize  Esc back │
└────────────────────────────────────────────────────────┘
```

- Left pane (~30%): scrollable note list with title and tags
- Right pane (~70%): markdown preview rendered with pulldown-cmark
- Bottom bar: mode indicator and available keybindings

## Modes & Keybindings

### Normal mode (default)
- `j`/`k` or arrows: move selection
- `/`: enter search mode
- `:`: enter command mode
- `q`: quit

### Search mode (after `/`)
- Type to fuzzy-filter by title and tags
- `Enter`: accept filter, return to normal mode
- `Esc`: clear search, return to normal mode

### Command mode (after `:`)
- `o`: open selected note (suspends TUI, launches $EDITOR, restores TUI)
- `a`: add note (modal for name/tags, then $EDITOR for content)
- `t`: edit tags on selected note (modal popup)
- `s`: summarize (placeholder for future LLM integration)
- `Esc`: cancel to normal mode

### Modal popups (add note, edit tags)
- Centered overlay with text input fields
- `Tab`: move between fields
- `Enter`: confirm
- `Esc`: cancel

## Architecture

### New dependencies
- ratatui + crossterm
- pulldown-cmark

### New files
- `src/tui/mod.rs` — app state, main event loop, terminal setup/teardown
- `src/tui/ui.rs` — layout rendering (panes, bottom bar, modals)
- `src/tui/events.rs` — keyboard input handling, mode transitions
- `src/tui/markdown.rs` — pulldown-cmark to ratatui styled text conversion

### Unchanged
- `src/db.rs` — TUI calls same DB functions
- `src/utils.rs` — validation and editor detection reused
- `src/commands/` — CLI subcommands stay for non-interactive use

### Key behaviors
- Query all notes on launch, refresh after mutations
- Preview updates on selection change
- Open note: teardown terminal, spawn editor, restore terminal on exit
- Modals call db.rs functions directly
- `scrap` (no args) and `scrap find` both launch TUI
