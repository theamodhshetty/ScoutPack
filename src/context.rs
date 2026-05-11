use crate::{index, search, token_budget};
use anyhow::Result;
use std::path::Path;

pub fn build_context_packet(
    root: impl AsRef<Path>,
    task: &str,
    budget: Option<usize>,
) -> Result<String> {
    let root = root.as_ref();
    let config = crate::config::load(root)?;
    let budget = budget.unwrap_or(config.default_budget);
    let conn = index::ensure_index(root)?;
    let mut results = search::search_repo(root, task, 12, true)?;
    let neighbors = search::enrich_with_import_neighbors(root, &results, 4, true)?;
    merge_results(&mut results, neighbors);
    let commands = index::read_commands(&conn)?;
    let frameworks = index::framework_signals(&conn)?;

    let mut prefix = String::new();
    prefix.push_str("# ScoutPack Context\n\n");
    prefix.push_str("Task:\n");
    prefix.push_str(task);
    prefix.push_str("\n\n");

    if budget < 600 {
        prefix.push_str("Warning: budget too small for snippets. Returning file list only.\n\n");
    }

    prefix.push_str("Relevant Files:\n");
    let files = relevant_files(&results);
    if files.is_empty() {
        prefix.push_str("- Unknown from index\n");
    } else {
        for (path, reason) in files {
            prefix.push_str(&format!("- `{path}`: {reason}\n"));
        }
    }

    prefix.push_str("\nCurrent Repo Signals:\n");
    if frameworks.is_empty() {
        prefix.push_str("- Framework: Unknown from index\n");
    } else {
        prefix.push_str(&format!("- Framework: {}\n", frameworks.join(", ")));
    }
    for (name, command, _) in commands
        .iter()
        .filter(|(name, _, _)| matches!(name.as_str(), "test" | "lint" | "typecheck" | "build"))
    {
        prefix.push_str(&format!("- {name} command: `{command}`\n"));
    }

    prefix.push_str("\nLikely Edit Areas:\n");
    let edit_areas = likely_edit_areas(task, &results);
    if edit_areas.is_empty() {
        prefix.push_str("- Unknown from index\n");
    } else {
        for area in edit_areas {
            prefix.push_str(&format!("- {area}\n"));
        }
    }

    let suffix = required_tail(&commands, task);
    let mut snippet_section = String::from("\nRelevant Snippets:\n");
    if results.is_empty() || budget < 600 {
        snippet_section.push_str("- Unknown from index\n");
    } else {
        let mut added = 0usize;
        for result in &results {
            let Some(text) = result.text.as_deref() else {
                continue;
            };
            let block = format!(
                "\n```file:{}:{}-{}\n{}\n```\n",
                result.path, result.start_line, result.end_line, text
            );
            let candidate = format!("{prefix}{snippet_section}{block}{suffix}");
            if token_budget::estimate_tokens(&candidate) <= budget + (budget / 10) {
                snippet_section.push_str(&block);
                added += 1;
            }
        }
        if added == 0 {
            snippet_section.push_str("- Budget exhausted before snippets.\n");
        }
    }

    let with_snippets = format!("{prefix}{snippet_section}{suffix}");
    if token_budget::fits(&with_snippets, budget + (budget / 10)) {
        Ok(with_snippets)
    } else {
        Ok(format!(
            "{prefix}\nRelevant Snippets:\n- Budget exhausted before snippets.\n{suffix}"
        ))
    }
}

fn required_tail(commands: &[(String, String, String)], task: &str) -> String {
    let mut tail = String::new();
    tail.push_str("\nCommands:\n");
    let relevant_commands: Vec<_> = commands
        .iter()
        .filter(|(name, _, _)| matches!(name.as_str(), "test" | "lint" | "typecheck" | "build"))
        .collect();
    if relevant_commands.is_empty() {
        tail.push_str("- Unknown from index\n");
    } else {
        for (_, command, _) in relevant_commands {
            tail.push_str(&format!("- `{command}`\n"));
        }
    }

    tail.push_str("\nRisks:\n");
    let risks = risk_hints(task);
    if risks.is_empty() {
        tail.push_str("- Unknown from index\n");
    } else {
        for risk in risks {
            tail.push_str(&format!("- {risk}\n"));
        }
    }
    tail
}

fn merge_results(results: &mut Vec<search::SearchResult>, extra: Vec<search::SearchResult>) {
    for result in extra {
        if results.iter().any(|existing| {
            existing.path == result.path
                && existing.start_line == result.start_line
                && existing.end_line == result.end_line
        }) {
            continue;
        }
        results.push(result);
    }
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

fn relevant_files(results: &[search::SearchResult]) -> Vec<(String, String)> {
    let mut files = Vec::new();
    for result in results {
        if files
            .iter()
            .any(|(path, _): &(String, String)| path == &result.path)
        {
            continue;
        }
        let reason = {
            if let Some(name) = &result.name {
                format!("{} `{name}`", result.kind)
            } else {
                result.reason.clone()
            }
        };
        files.push((result.path.clone(), reason));
    }
    files
}

fn likely_edit_areas(task: &str, results: &[search::SearchResult]) -> Vec<String> {
    let task = task.to_ascii_lowercase();
    let mut areas = Vec::new();
    if task.contains("redirect") {
        areas.push("redirect and destination parameter handling".to_owned());
    }
    if task.contains("login") || task.contains("auth") {
        areas.push("auth/session boundary files".to_owned());
    }
    if task.contains("dark") || task.contains("theme") {
        areas.push("theme state, provider, and persisted preference files".to_owned());
    }
    if task.contains("api") {
        areas.push("route handlers and client fetch call sites".to_owned());
    }
    for result in results.iter().take(3) {
        areas.push(format!(
            "`{}:{}-{}`",
            result.path, result.start_line, result.end_line
        ));
    }
    areas.sort();
    areas.dedup();
    areas
}

fn risk_hints(task: &str) -> Vec<&'static str> {
    let task = task.to_ascii_lowercase();
    let mut risks = Vec::new();
    if task.contains("redirect") || task.contains("login") || task.contains("auth") {
        risks.push("redirect loop if post-login destination points back to login or auth guard");
        risks.push("SSR/client mismatch if session state is checked only client-side");
    }
    if task.contains("env") || task.contains("secret") {
        risks.push("secret files are intentionally not indexed; verify env names manually");
    }
    if task.contains("test") {
        risks.push("test command exists in index, but ScoutPack does not execute project scripts");
    }
    risks
}
