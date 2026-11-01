mod cmd;
mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand};

/// CLI tool to view Rust documentation globally as Markdown.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// View documentation for a Rust path
    Doc {
        /// The Rust path to view documentation for, e.g., std::vec::Vec
        path: String,
        /// Show only the primary definition and member signatures, stripping heavy text
        #[arg(short = 's', long)]
        signatures_only: bool,
    },
    /// View the source code for a documented item
    Source {
        /// The Rust path to view source code for, e.g., std::vec::Vec
        path: String,
    },
    /// Search for a term in the documentation HTML files
    Search {
        /// The term to search for
        query: String,
        /// The crate to search within, e.g. std or a local crate name (searches all local docs if not provided)
        #[arg(short, long)]
        crate_name: Option<String>,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Doc {
            path,
            signatures_only,
        } => cmd::doc::cmd_doc(&path, signatures_only)?,
        Commands::Source { path } => cmd::source::cmd_source(&path)?,
        Commands::Search { query, crate_name } => cmd::search::cmd_search(&query, crate_name)?,
    }

    Ok(())
}
