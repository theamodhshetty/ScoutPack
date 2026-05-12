mod chunk;
mod cli;
mod config;
mod context;
mod git;
mod index;
mod mcp;
mod output;
mod scanner;
mod search;
mod semantic;
mod token_budget;
mod watch;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use cli::{Cli, Commands, ContextFormat};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    match cli.command {
        Commands::Init { path } => {
            config::init_project(&path)?;
            println!("ScoutPack initialized at {}", path.display());
        }
        Commands::Pack {
            path,
            embed,
            allow_model_download,
        } => {
            let summary = index::pack_repo_with_options(
                &path,
                index::PackOptions {
                    embed,
                    allow_model_download,
                },
            )?;
            println!(
                "Indexed {} files, reused {} unchanged, removed {}, added {} chunks, added {} embeddings, skipped {} files.",
                summary.files_indexed,
                summary.files_reused,
                summary.files_removed,
                summary.chunks_indexed,
                summary.embeddings_indexed,
                summary.files_skipped
            );
            println!("Index: {}", summary.index_path.display());
        }
        Commands::Watch { path, debounce_ms } => {
            watch::watch_repo(path, std::time::Duration::from_millis(debounce_ms))?;
        }
        Commands::Search {
            query,
            limit,
            show_snippets,
            json,
            semantic,
            semantic_alpha,
        } => {
            let results = search::search_current_dir_with_options(
                &query,
                limit,
                show_snippets,
                search::SearchOptions {
                    semantic,
                    semantic_alpha,
                },
            )?;
            if json {
                output::print_search_results_json(&query, &results)?;
            } else {
                output::print_search_results(&results);
            }
        }
        Commands::Context {
            task,
            budget,
            format,
            json,
            since,
            diff,
            branch,
            semantic,
            semantic_alpha,
        } => {
            let git_mode = context_git_mode(since, diff, branch)?;
            let packet = context::build_context_packet_with_options(
                ".",
                &task,
                budget,
                context::ContextOptions {
                    git_mode,
                    semantic,
                    semantic_alpha,
                },
            )?;
            let format = if json { ContextFormat::Json } else { format };
            match format {
                ContextFormat::Markdown => print!("{packet}"),
                ContextFormat::Json => output::print_context_json(&task, budget, &packet)?,
                ContextFormat::Xml => output::print_context_xml(&task, budget, &packet)?,
            }
        }
        Commands::Stats { path, json } => {
            let stats = index::read_stats(&path)?;
            if json {
                output::print_stats_json(&stats)?;
            } else {
                output::print_stats(&stats);
            }
        }
        Commands::Mcp { path } => {
            mcp::serve(path).await?;
        }
        Commands::Completions { shell } => {
            let mut command = Cli::command();
            let name = command.get_name().to_owned();
            clap_complete::generate(shell, &mut command, name, &mut std::io::stdout());
        }
    }

    Ok(())
}

fn context_git_mode(
    since: Option<String>,
    diff: Option<String>,
    branch: bool,
) -> Result<Option<git::GitContextMode>> {
    let selected = since.is_some() as u8 + diff.is_some() as u8 + branch as u8;
    if selected > 1 {
        anyhow::bail!("Use only one of `--since`, `--diff`, or `--branch`.");
    }
    if let Some(since) = since {
        return Ok(Some(git::GitContextMode::Since(since)));
    }
    if let Some(diff) = diff {
        let Some((base, head)) = diff.split_once("..") else {
            anyhow::bail!("`--diff` must use `<base>..<head>`, for example `main..HEAD`.");
        };
        if base.is_empty() || head.is_empty() {
            anyhow::bail!("`--diff` must use `<base>..<head>`, for example `main..HEAD`.");
        }
        return Ok(Some(git::GitContextMode::Diff {
            base: base.to_owned(),
            head: head.to_owned(),
        }));
    }
    if branch {
        return Ok(Some(git::GitContextMode::Branch));
    }
    Ok(None)
}

fn init_tracing(verbose: u8) {
    let level = match verbose {
        0 => "warn",
        1 => "info",
        _ => "debug",
    };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(level)
        .with_writer(std::io::stderr)
        .without_time()
        .try_init();
}
