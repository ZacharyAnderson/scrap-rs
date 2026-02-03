mod commands;
mod db;
mod llm;
mod tui;
mod utils;
mod version_check;

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
    /// Find and browse notes
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
    /// Write a note from stdin (create or update)
    Write {
        /// Name of the note
        name: String,
        /// Tags for the note
        tags: Vec<String>,
    },
    /// Read a note to stdout
    Read {
        /// Name of the note
        name: String,
    },
    /// List note names to stdout
    List {
        /// Filter by tag
        #[arg(long)]
        tag: Option<String>,
    },
    /// Append stdin content to an existing note
    Append {
        /// Name of the note
        name: String,
    },
}

fn main() -> anyhow::Result<()> {
    // Check for updates (non-blocking, cached)
    version_check::check_for_updates();

    let cli = Cli::parse();

    match cli.command {
        None => tui::run(),
        Some(Commands::Add { name, tags }) => commands::add::run(&name, &tags),
        Some(Commands::Delete { name }) => commands::delete::run(&name),
        Some(Commands::Find { query: _ }) => tui::run(),
        Some(Commands::Open { name }) => commands::open::run(&name),
        Some(Commands::EditTag {
            add,
            delete,
            name,
            tags,
        }) => commands::edit_tag::run(&name, &tags, add, delete),
        Some(Commands::Write { name, tags }) => commands::write::run(&name, &tags),
        Some(Commands::Read { name }) => commands::read::run(&name),
        Some(Commands::List { tag }) => commands::list::run(tag.as_deref()),
        Some(Commands::Append { name }) => commands::append::run(&name),
    }
}
