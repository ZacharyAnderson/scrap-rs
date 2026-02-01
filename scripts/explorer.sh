#!/usr/bin/env bash

DB_FILE="$HOME/.scrap/scrap.db"

if [[ ! -f "$DB_FILE" ]]; then
    echo "No scrap database found."
    exit 1
fi

# Resolve scrap binary
if [[ -n "$SCRAP_BIN" && -f "$SCRAP_BIN" ]]; then
    :
else
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    if [[ -f "$SCRIPT_DIR/../target/release/scrap" ]]; then
        SCRAP_BIN="$SCRIPT_DIR/../target/release/scrap"
    elif [[ -f "$SCRIPT_DIR/../target/debug/scrap" ]]; then
        SCRAP_BIN="$SCRIPT_DIR/../target/debug/scrap"
    elif command -v scrap &>/dev/null; then
        SCRAP_BIN="scrap"
    else
        echo "scrap binary not found. Build the project or add scrap to your PATH."
        exit 1
    fi
fi

# Preview command: bat with markdown, fall back to cat
if command -v bat &>/dev/null; then
    PREVIEW_CMD="sqlite3 \"$DB_FILE\" \"SELECT note FROM notes WHERE title = {1} LIMIT 1;\" | bat --style=plain --language=markdown --color=always -"
else
    PREVIEW_CMD="sqlite3 \"$DB_FILE\" \"SELECT note FROM notes WHERE title = {1} LIMIT 1;\""
fi

SEARCH_QUERY="$1"
FZF_ARGS=""
if [[ -n "$SEARCH_QUERY" ]]; then
    FZF_ARGS="--query=$SEARCH_QUERY"
fi

SELECTED_NOTE=$(sqlite3 -separator $'\t' "$DB_FILE" "SELECT title, tags FROM notes ORDER BY updated_at DESC;" | \
    fzf --delimiter=$'\t' \
        --with-nth=1,2 \
        --prompt="Select a note: " \
        --preview="$PREVIEW_CMD" \
        --preview-window="right:60%:wrap" \
        --header="Title | Tags" \
        --height=100% \
        $FZF_ARGS
)

if [[ -n "$SELECTED_NOTE" ]]; then
    TITLE=$(echo "$SELECTED_NOTE" | cut -f1)
    "$SCRAP_BIN" open "$TITLE"
fi
