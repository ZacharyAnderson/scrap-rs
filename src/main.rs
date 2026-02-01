mod commands;
mod db;
mod llm;
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
}

fn main() -> anyhow::Result<()> {
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
    }
}
