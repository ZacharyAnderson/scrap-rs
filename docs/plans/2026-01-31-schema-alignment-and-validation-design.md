# Schema Alignment & Input Validation

## Problem

The Rust code uses column names (`name`, `contents`, `last_modified`) that don't match the existing Zig database schema (`title`, `note`, `updated_at`). Every query fails with "no such column." Additionally, there is no input validation — invalid note names and tags pass through to the database layer.

## Schema Alignment

Align `db.rs` SQL and DDL with the Zig schema:

| Rust code (current) | Zig DB (actual) |
|---|---|
| `name` | `title` |
| `contents` | `note` |
| `last_modified` | `updated_at` |
| `tags TEXT DEFAULT '[]'` | `tags JSON` |

Update `CREATE TABLE IF NOT EXISTS` and trigger to match the Zig schema exactly. Update `explorer.sh` column names.

Function signatures in `db.rs` keep descriptive param names — only SQL strings change.

## Input Validation

Add `validate_name()` and `validate_tags()` in `utils.rs`, called at the top of each command's `run()`:

**Note names:**
- Must not be empty or whitespace-only
- No spaces
- No path separators (`/`, `\`)
- Max 100 characters

**Tags:**
- Must not be empty or whitespace-only
- No spaces in individual tags
- Max 50 characters per tag

**Content:** Allow empty — intentional empty notes are valid.

**Edit-tag:** Require at least one tag argument.

## Files Changed

- `src/db.rs` — column names in SQL, DDL, trigger
- `src/utils.rs` — add validation functions
- `src/commands/add.rs` — call validation
- `src/commands/open.rs` — call validation
- `src/commands/delete.rs` — call validation
- `src/commands/find.rs` — no change (no name input)
- `src/commands/edit_tag.rs` — call validation, require >= 1 tag
- `scripts/explorer.sh` — column names
