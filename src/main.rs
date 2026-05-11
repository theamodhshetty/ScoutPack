mod chunk;
mod cli;
mod config;
mod context;
mod git;
mod index;
mod output;
mod scanner;
mod search;
mod token_budget;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    match cli.command {
        Commands::Init { path } => {
            config::init_project(&path)?;
            println!("ScoutPack initialized at {}", path.display());
        }
        Commands::Pack { path } => {
            let summary = index::pack_repo(&path)?;
            println!(
                "Indexed {} files, {} chunks, skipped {} files.",
                summary.files_indexed, summary.chunks_indexed, summary.files_skipped
            );
            println!("Index: {}", summary.index_path.display());
        }
        Commands::Search {
            query,
            limit,
            show_snippets,
        } => {
            let results = search::search_current_dir(&query, limit, show_snippets)?;
            output::print_search_results(&results);
        }
        Commands::Context { task, budget } => {
            let packet = context::build_context_packet(".", &task, budget)?;
            print!("{packet}");
        }
        Commands::Stats { path } => {
            let stats = index::read_stats(&path)?;
            output::print_stats(&stats);
        }
    }

    Ok(())
}

fn init_tracing(verbose: u8) {
    let level = match verbose {
        0 => "warn",
        1 => "info",
        _ => "debug",
    };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(level)
        .without_time()
        .try_init();
}
