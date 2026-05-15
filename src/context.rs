use crate::{git, index, search, token_budget};
use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ContextOptions {
    pub git_mode: Option<git::GitContextMode>,
    pub semantic: bool,
    pub semantic_alpha: f64,
    pub expand_calls: usize,
}

impl Default for ContextOptions {
    fn default() -> Self {
        Self {
            git_mode: None,
            semantic: false,
            semantic_alpha: 0.45,
            expand_calls: 0,
        }
    }
}

pub fn build_context_packet(
    root: impl AsRef<Path>,
    task: &str,
    budget: Option<usize>,
) -> Result<String> {
    build_context_packet_with_options(root, task, budget, ContextOptions::default())
}

pub fn build_context_packet_with_options(
    root: impl AsRef<Path>,
    task: &str,
    budget: Option<usize>,
    options: ContextOptions,
) -> Result<String> {
    let root = root.as_ref();
    let config = crate::config::load(root)?;
    let budget = budget.unwrap_or(config.default_budget);
    let conn = index::ensure_index(root)?;
    let mut results = search::search_repo_with_options(
        root,
        task,
        12,
        true,
        search::SearchOptions {
            semantic: options.semantic,
            semantic_alpha: options.semantic_alpha,
        },
    )?;
    let neighbors = search::enrich_with_import_neighbors(root, &results, 4, true)?;
    merge_results(&mut results, neighbors);
    let recent_changes = apply_git_context(root, options.git_mode.as_ref(), &mut results)?;
    if options.expand_calls > 0 {
        let call_neighbors =
            search::expand_call_graph(root, &results, options.expand_calls, 8, true)?;
        merge_results(&mut results, call_neighbors);
    }
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

    if let Some(recent_changes) = &recent_changes {
        prefix.push_str("\nRecent Changes:\n");
        prefix.push_str(&format!(
            "- Range: `{}`..`{}`\n",
            recent_changes.base, recent_changes.head
        ));
        prefix.push_str(&format!(
            "- Summary: {} files changed, +{} -{}\n",
            recent_changes.files_changed, recent_changes.insertions, recent_changes.deletions
        ));
        if recent_changes.changes.is_empty() {
            prefix.push_str("- No changed files in range\n");
        } else {
            for change in recent_changes.changes.iter().take(20) {
                prefix.push_str(&format!(
                    "- `{}`: {}, +{} -{}\n",
                    change.path, change.status, change.additions, change.deletions
                ));
            }
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

    let suffix = required_tail(&commands, task, &results, budget);
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

fn apply_git_context(
    root: &Path,
    mode: Option<&git::GitContextMode>,
    results: &mut Vec<search::SearchResult>,
) -> Result<Option<git::RecentChanges>> {
    let Some(mode) = mode else {
        return Ok(None);
    };
    let recent = git::recent_changes(root, mode)?;
    let changed_paths: Vec<String> = recent
        .changes
        .iter()
        .filter(|change| change.status != "deleted")
        .map(|change| change.path.clone())
        .collect();
    let changed: HashSet<_> = changed_paths.iter().cloned().collect();
    let guaranteed = search::chunks_for_paths(root, &changed_paths, true)?;

    match mode {
        git::GitContextMode::Since(_) => {
            for result in results.iter_mut() {
                if changed.contains(&result.path) {
                    result.score *= 1.5;
                    result.reason = format!("changed in git range; {}", result.reason);
                }
            }
            merge_results(results, guaranteed);
        }
        git::GitContextMode::Diff { .. } | git::GitContextMode::Branch => {
            results.retain(|result| changed.contains(&result.path));
            merge_results(results, guaranteed);
        }
    }
    Ok(Some(recent))
}

fn required_tail(
    commands: &[(String, String, String)],
    task: &str,
    results: &[search::SearchResult],
    budget: usize,
) -> String {
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
    let risks = risk_hints(task, results);
    if risks.is_empty() {
        tail.push_str("- Unknown from index\n");
    } else {
        for risk in risks {
            tail.push_str(&format!("- {risk}\n"));
        }
    }

    tail.push_str("\nToken Budget Summary:\n");
    tail.push_str(&format!("- Target budget: {budget} tokens\n"));
    tail.push_str(
        "- ScoutPack keeps required files, commands, and risks before optional snippets.\n",
    );
    tail
}

fn merge_results(results: &mut Vec<search::SearchResult>, extra: Vec<search::SearchResult>) {
    for result in extra {
        if let Some(existing) = results.iter_mut().find(|existing| {
            existing.path == result.path
                && existing.start_line == result.start_line
                && existing.end_line == result.end_line
        }) {
            if result.reason.starts_with("call graph:")
                && !existing.reason.contains(result.reason.as_str())
            {
                existing.reason = format!("{}; {}", existing.reason, result.reason);
            }
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
                let base = format!("{} `{name}`", result.kind);
                if result.reason.contains("call graph:") {
                    format!("{base}; {}", result.reason)
                } else {
                    base
                }
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

fn risk_hints(task: &str, results: &[search::SearchResult]) -> Vec<String> {
    let task = task.to_ascii_lowercase();
    let mut risks = Vec::new();
    let redirect_sources = evidence_sources(results, &["redirect", "login"]);
    let session_sources = evidence_sources(results, &["session", "auth", "getsession"]);

    if (task.contains("redirect") || task.contains("login") || task.contains("auth"))
        && !redirect_sources.is_empty()
    {
        risks.push(format!(
            "redirect loop if post-login destination points back to login or auth guard (source: {})",
            redirect_sources.join(", ")
        ));
    }
    if (task.contains("login") || task.contains("auth")) && !session_sources.is_empty() {
        risks.push(format!(
            "SSR/client mismatch if session state is checked only client-side (source: {})",
            session_sources.join(", ")
        ));
    }
    if task.contains("env") || task.contains("secret") {
        risks.push(
            "secret files are intentionally not indexed; verify env names manually".to_owned(),
        );
    }
    if task.contains("test") {
        risks.push(
            "test command exists in index, but ScoutPack does not execute project scripts"
                .to_owned(),
        );
    }
    risks
}

fn evidence_sources(results: &[search::SearchResult], terms: &[&str]) -> Vec<String> {
    let mut sources = Vec::new();
    for result in results {
        let text = result
            .text
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let path = result.path.to_ascii_lowercase();
        if !terms
            .iter()
            .any(|term| text.contains(term) || path.contains(term))
        {
            continue;
        }
        let source = format!(
            "`{}:{}-{}`",
            result.path, result.start_line, result.end_line
        );
        if !sources.contains(&source) {
            sources.push(source);
        }
        if sources.len() == 3 {
            break;
        }
    }
    sources
}
