use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "scoutpack")]
#[command(version)]
#[command(about = "Offline repo context for AI agents.")]
pub struct Cli {
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Create scoutpack.toml and .scoutpackignore.
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Scan a repo/folder and build a local ScoutPack index.
    Pack {
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Search the local ScoutPack index.
    Search {
        query: String,

        #[arg(short, long, default_value_t = 5)]
        limit: usize,

        #[arg(long, default_value_t = false)]
        show_snippets: bool,
    },

    /// Build a compact task-specific markdown context packet.
    Context {
        task: String,

        #[arg(short, long)]
        budget: Option<usize>,
    },

    /// Show index stats.
    Stats {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}
