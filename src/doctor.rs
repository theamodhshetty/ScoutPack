use crate::{config, git, index};
use anyhow::{Context, Result};
use serde::Serialize;
use std::{path::Path, time::SystemTime};

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub status: String,
    pub root: String,
    pub index_path: String,
    pub index_exists: bool,
    pub index_readable: bool,
    pub schema_version: Option<i64>,
    pub expected_schema_version: i64,
    pub fts_ready: bool,
    pub stale: bool,
    pub stale_reasons: Vec<String>,
    pub config_exists: bool,
    pub config_valid: bool,
    pub ignore_exists: bool,
    pub indexed_at: Option<i64>,
    pub age_seconds: Option<i64>,
    pub file_count: Option<i64>,
    pub chunk_count: Option<i64>,
    pub symbol_count: Option<i64>,
    pub skipped_count: Option<i64>,
    pub languages: Vec<index::LanguageCount>,
    pub frameworks: Vec<String>,
    pub git_branch: Option<String>,
    pub git_head: Option<String>,
    pub working_changes: Vec<String>,
    pub freshness: Option<index::IndexFreshness>,
    pub recommendations: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh: Option<index::PackSummary>,
}

pub fn diagnose(root: &Path, fix: bool) -> Result<DoctorReport> {
    if !root.exists() {
        anyhow::bail!("Path does not exist: {}", root.display());
    }
    let root = root
        .canonicalize()
        .with_context(|| format!("Could not resolve {}", root.display()))?;
    let refresh = fix.then(|| index::pack_repo(&root)).transpose()?;
    let index_path = index::index_path(&root);
    let index_exists = index_path.exists();
    let config_exists = root.join("scoutpack.toml").exists();
    let ignore_exists = root.join(".scoutpackignore").exists();
    let config_valid = config::load(&root).is_ok();
    let git_branch = git::current_branch(&root);
    let git_head = git::head_commit(&root);
    let working_changes = git::recent_changed_files(&root);

    let mut report = DoctorReport {
        status: "missing".to_owned(),
        root: root.display().to_string(),
        index_path: index_path.display().to_string(),
        index_exists,
        index_readable: false,
        schema_version: None,
        expected_schema_version: index::SCHEMA_VERSION,
        fts_ready: false,
        stale: true,
        stale_reasons: Vec::new(),
        config_exists,
        config_valid,
        ignore_exists,
        indexed_at: None,
        age_seconds: None,
        file_count: None,
        chunk_count: None,
        symbol_count: None,
        skipped_count: None,
        languages: Vec::new(),
        frameworks: Vec::new(),
        git_branch,
        git_head,
        working_changes,
        freshness: None,
        recommendations: Vec::new(),
        refresh,
    };

    if !config_valid {
        report
            .stale_reasons
            .push("scoutpack.toml is invalid".to_owned());
    }
    if !index_exists {
        report
            .stale_reasons
            .push("local index does not exist".to_owned());
        report.recommendations.push(
            "Run `scoutpack doctor --fix` or any search/context command to build it.".to_owned(),
        );
        return Ok(report);
    }

    report.schema_version = match index::database_schema_version(&root) {
        Ok(version) => {
            report.index_readable = true;
            version
        }
        Err(error) => {
            report.status = "broken".to_owned();
            report
                .stale_reasons
                .push(format!("SQLite index is unreadable: {error}"));
            report
                .recommendations
                .push("Check for another ScoutPack process or remove the unreadable local `.scoutpack/pack.sqlite`, then run `scoutpack doctor --fix`.".to_owned());
            return Ok(report);
        }
    };

    if report.schema_version != Some(index::SCHEMA_VERSION) {
        report.stale_reasons.push(format!(
            "schema version is {:?}; expected {}",
            report.schema_version,
            index::SCHEMA_VERSION
        ));
    } else {
        let conn = index::ensure_index(&root)?;
        report.fts_ready = index::fts_available(&conn)?;
        if !report.fts_ready {
            report
                .stale_reasons
                .push("SQLite FTS index is missing".to_owned());
        }
        let stats = index::read_stats(&root)?;
        report.file_count = Some(stats.file_count);
        report.chunk_count = Some(stats.chunk_count);
        report.symbol_count = Some(stats.symbol_count);
        report.skipped_count = Some(stats.skipped_count);
        report.languages = index::language_breakdown(&conn)?;
        report.frameworks = index::framework_signals(&conn)?;
        if let Some(manifest) = stats.manifest {
            report.indexed_at = Some(manifest.indexed_at);
            report.age_seconds = unix_now().map(|now| now.saturating_sub(manifest.indexed_at));
        } else {
            report
                .stale_reasons
                .push("index manifest is missing or invalid".to_owned());
        }

        if config_valid {
            match index::inspect_freshness(&root) {
                Ok(freshness) => {
                    if freshness.changed_files > 0 {
                        report.stale_reasons.push(format!(
                            "{} added or changed files need indexing",
                            freshness.changed_files
                        ));
                    }
                    if freshness.removed_files > 0 {
                        report.stale_reasons.push(format!(
                            "{} removed files remain in the index",
                            freshness.removed_files
                        ));
                    }
                    if freshness.config_changed {
                        report
                            .stale_reasons
                            .push("index configuration changed".to_owned());
                    }
                    report.freshness = Some(freshness);
                }
                Err(error) => report
                    .stale_reasons
                    .push(format!("freshness check failed: {error}")),
            }
        }
    }

    report.stale = !report.stale_reasons.is_empty();
    report.status = if report.stale {
        "stale".to_owned()
    } else {
        "healthy".to_owned()
    };
    if report.stale {
        report
            .recommendations
            .push("Run `scoutpack doctor --fix` to refresh the local index.".to_owned());
    }
    if !config_exists || !ignore_exists {
        report.recommendations.push(
            "Optional: run `scoutpack init` to create explicit config and ignore files.".to_owned(),
        );
    }
    Ok(report)
}

pub fn print_report(report: &DoctorReport) {
    println!("ScoutPack doctor: {}", report.status);
    println!("Root: {}", report.root);
    println!("Index: {}", report.index_path);
    println!(
        "Schema: {} / {}",
        report
            .schema_version
            .map(|version| version.to_string())
            .unwrap_or_else(|| "missing".to_owned()),
        report.expected_schema_version
    );
    println!(
        "FTS: {}",
        if report.fts_ready {
            "ready"
        } else {
            "not ready"
        }
    );
    if let Some(files) = report.file_count {
        println!(
            "Files: {files} | Chunks: {} | Symbols: {} | Skipped: {}",
            report.chunk_count.unwrap_or_default(),
            report.symbol_count.unwrap_or_default(),
            report.skipped_count.unwrap_or_default()
        );
    }
    if !report.languages.is_empty() {
        let languages = report
            .languages
            .iter()
            .map(|item| format!("{} {}", item.language, item.files))
            .collect::<Vec<_>>()
            .join(", ");
        println!("Languages: {languages}");
    }
    if !report.frameworks.is_empty() {
        println!("Frameworks: {}", report.frameworks.join(", "));
    }
    for reason in &report.stale_reasons {
        println!("Issue: {reason}");
    }
    for recommendation in &report.recommendations {
        println!("Next: {recommendation}");
    }
}

fn unix_now() -> Option<i64> {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs() as i64)
}
