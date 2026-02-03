# scrap

A CLI note-taking app with a TUI, tag-based organization, and LLM-powered summaries.

## Requirements

- Rust (1.85+)
- A terminal editor (`$EDITOR`, or it will detect nvim/vim/vi/nano/emacs)

Optional:

- `ANTHROPIC_API_KEY` environment variable for the summarize feature

## Install

```sh
git clone <repo-url> && cd scrap-rs
cargo install --path .
```

This installs the `scrap` binary to `~/.cargo/bin/`.

## Usage

### TUI

Run `scrap` with no arguments to launch the interactive TUI.

```sh
scrap
```

**Layout:** Note list and tag panel on the left, markdown preview on the right.

**Keybindings:**

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate notes |
| `/` | Search notes |
| `Tab` | Switch focus between notes and tags |
| `:` | Enter command mode |
| `q` | Quit |

**Command mode (`:`):**

| Key | Action |
|-----|--------|
| `o` | Open selected note in your editor |
| `a` | Add a new note |
| `t` | Add/remove tags on selected note |
| `s` | Summarize selected note (requires API key) |

**Tag panel:** Press `Enter` to toggle tag filters. Select multiple tags to narrow results. `Esc` clears all filters.

### CLI Commands

```sh
scrap add <name> [tags...]      # Create a new note
scrap open <name>               # Edit an existing note
scrap delete <name>             # Delete a note
scrap edit-tag --add <name> [tags...]     # Add tags
scrap edit-tag --delete <name> [tags...]  # Remove tags
scrap find [query]              # Launch TUI with optional search
```

Note names can contain spaces when quoted:

```sh
scrap add "my note name" tag1 tag2
scrap open "my note name"
```

### Piped / Programmatic Commands

These commands read from stdin and write to stdout, making them suitable for scripts and external tools.

```sh
echo "note content" | scrap write <name> [tags...]   # Create or update a note from stdin
scrap read <name>                                     # Print note content to stdout
scrap list [--tag TAG]                                # List note names, one per line
echo "extra content" | scrap append <name>            # Append stdin to an existing note
```

Names with spaces work here too:

```sh
echo "content" | scrap write "my note" tag1
scrap read "my note"
```

## Summarize Feature

Summarize uses the Anthropic API to generate markdown summaries of your notes. Summaries are cached in the database and marked stale when you edit a note.

To enable, add your API key to your shell config:

```sh
echo 'export ANTHROPIC_API_KEY=your_key_here' >> ~/.zshrc
source ~/.zshrc
```

In the TUI, press `:s` on a selected note to generate or view a summary.

## Data

Notes are stored in a SQLite database at `~/.scrap/scrap.db`.
