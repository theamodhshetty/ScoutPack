use crate::{index, search, token_budget};
use anyhow::Result;
use std::{collections::BTreeMap, path::Path};

pub fn build_context_packet(
    root: impl AsRef<Path>,
    task: &str,
    budget: Option<usize>,
) -> Result<String> {
    let root = root.as_ref();
    let config = crate::config::load(root)?;
    let budget = budget.unwrap_or(config.default_budget);
    let conn = index::ensure_index(root)?;
    let results = search::search_repo(root, task, 12, true)?;
    let commands = index::read_commands(&conn)?;
    let frameworks = index::framework_signals(&conn)?;

    let mut packet = String::new();
    packet.push_str("# ScoutPack Context\n\n");
    packet.push_str("Task:\n");
    packet.push_str(task);
    packet.push_str("\n\n");

    if budget < 600 {
        packet.push_str("Warning: budget too small for snippets. Returning file list only.\n\n");
    }

    packet.push_str("Relevant Files:\n");
    let files = relevant_files(&results);
    if files.is_empty() {
        packet.push_str("- Unknown from index\n");
    } else {
        for (path, reason) in &files {
            packet.push_str(&format!("- `{path}`: {reason}\n"));
        }
    }

    packet.push_str("\nCurrent Repo Signals:\n");
    if frameworks.is_empty() {
        packet.push_str("- Framework: Unknown from index\n");
    } else {
        packet.push_str(&format!("- Framework: {}\n", frameworks.join(", ")));
    }
    for (name, command, _) in commands
        .iter()
        .filter(|(name, _, _)| matches!(name.as_str(), "test" | "lint" | "typecheck" | "build"))
    {
        packet.push_str(&format!("- {name} command: `{command}`\n"));
    }

    packet.push_str("\nLikely Edit Areas:\n");
    let edit_areas = likely_edit_areas(task, &results);
    if edit_areas.is_empty() {
        packet.push_str("- Unknown from index\n");
    } else {
        for area in edit_areas {
            packet.push_str(&format!("- {area}\n"));
        }
    }

    let mut with_snippets = packet.clone();
    with_snippets.push_str("\nRelevant Snippets:\n");
    if results.is_empty() || budget < 600 {
        with_snippets.push_str("- Unknown from index\n");
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
            let candidate = format!("{with_snippets}{block}");
            if token_budget::estimate_tokens(&candidate) <= budget + (budget / 10) {
                with_snippets.push_str(&block);
                added += 1;
            }
        }
        if added == 0 {
            with_snippets.push_str("- Budget exhausted before snippets.\n");
        }
    }

    with_snippets.push_str("\nCommands:\n");
    let relevant_commands: Vec<_> = commands
        .iter()
        .filter(|(name, _, _)| matches!(name.as_str(), "test" | "lint" | "typecheck" | "build"))
        .collect();
    if relevant_commands.is_empty() {
        with_snippets.push_str("- Unknown from index\n");
    } else {
        for (_, command, _) in relevant_commands {
            with_snippets.push_str(&format!("- `{command}`\n"));
        }
    }

    with_snippets.push_str("\nRisks:\n");
    let risks = risk_hints(task);
    if risks.is_empty() {
        with_snippets.push_str("- Unknown from index\n");
    } else {
        for risk in risks {
            with_snippets.push_str(&format!("- {risk}\n"));
        }
    }

    if token_budget::fits(&with_snippets, budget + (budget / 10)) {
        Ok(with_snippets)
    } else {
        Ok(packet)
    }
}

fn relevant_files(results: &[search::SearchResult]) -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    for result in results {
        files.entry(result.path.clone()).or_insert_with(|| {
            if let Some(name) = &result.name {
                format!("{} `{name}`", result.kind)
            } else {
                result.reason.clone()
            }
        });
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
