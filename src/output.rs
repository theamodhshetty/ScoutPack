use crate::{index::IndexStats, search::SearchResult};
use anyhow::Result;
use serde_json::json;

pub fn print_search_results(results: &[SearchResult]) {
    if results.is_empty() {
        println!("No results from index.");
        return;
    }

    for (idx, result) in results.iter().enumerate() {
        let name = result
            .name
            .as_ref()
            .map(|name| format!(" {name}"))
            .unwrap_or_default();
        println!(
            "{}. {}:{}-{} [{}]{}",
            idx + 1,
            result.path,
            result.start_line,
            result.end_line,
            result.kind,
            name
        );
        println!("   reason: {}", result.reason);
        if let Some(text) = &result.text {
            println!("\n{text}\n");
        }
    }
}

pub fn print_search_results_json(query: &str, results: &[SearchResult]) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "query": query,
            "results": results,
        }))?
    );
    Ok(())
}

pub fn print_context_json(task: &str, budget: Option<usize>, packet: &str) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "task": task,
            "budget": budget,
            "estimated_tokens": crate::token_budget::estimate_tokens(packet),
            "packet": packet,
        }))?
    );
    Ok(())
}

pub fn print_stats(stats: &IndexStats) {
    println!("Index: {}", stats.index_path.display());
    println!("Files: {}", stats.file_count);
    println!("Chunks: {}", stats.chunk_count);
    println!("Symbols: {}", stats.symbol_count);
    println!("Commands: {}", stats.command_count);
    println!("Skipped: {}", stats.skipped_count);
    if let Some(manifest) = &stats.manifest {
        println!("Version: {}", manifest.scoutpack_version);
        println!("Root: {}", manifest.indexed_root_path);
        println!(
            "Git branch: {}",
            manifest
                .current_git_branch
                .as_deref()
                .unwrap_or("Unknown from index")
        );
        println!(
            "Recent changed files: {}",
            manifest.recent_changed_files.len()
        );
        println!("Recent commits: {}", manifest.recent_commit_subjects.len());
    }
}

pub fn print_stats_json(stats: &IndexStats) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "index_path": stats.index_path,
            "file_count": stats.file_count,
            "chunk_count": stats.chunk_count,
            "symbol_count": stats.symbol_count,
            "command_count": stats.command_count,
            "skipped_count": stats.skipped_count,
            "manifest": stats.manifest,
        }))?
    );
    Ok(())
}
