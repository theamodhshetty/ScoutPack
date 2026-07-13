use anyhow::{Context, Result};
use git2::{Delta, DiffOptions, Patch, Repository, StatusOptions, Tree};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum GitContextMode {
    Since(String),
    Diff { base: String, head: String },
    Branch,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitChange {
    pub path: String,
    pub status: String,
    pub additions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentChanges {
    pub base: String,
    pub head: String,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub changes: Vec<GitChange>,
}

pub fn current_branch(root: &Path) -> Option<String> {
    let repo = Repository::discover(root).ok()?;
    let head = repo.head().ok()?;
    if head.is_branch() {
        head.shorthand().map(ToOwned::to_owned)
    } else {
        head.target().map(|oid| oid.to_string())
    }
}

pub fn head_commit(root: &Path) -> Option<String> {
    let repo = Repository::discover(root).ok()?;
    let head = repo.head().ok()?;
    head.target().map(|oid| oid.to_string())
}

pub fn recent_changed_files(root: &Path) -> Vec<String> {
    let Ok(repo) = Repository::discover(root) else {
        return Vec::new();
    };
    let workdir = repo.workdir().map(Path::to_path_buf);
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true);
    let Ok(statuses) = repo.statuses(Some(&mut options)) else {
        return Vec::new();
    };
    statuses
        .iter()
        .filter_map(|entry| entry.path().map(ToOwned::to_owned))
        .filter(|path| path != ".scoutpack" && !path.starts_with(".scoutpack/"))
        .map(|path| relativize_to_root(&repo, workdir.as_deref(), root, &path))
        .collect()
}

pub fn recent_commit_subjects(root: &Path) -> Vec<String> {
    let Ok(repo) = Repository::discover(root) else {
        return Vec::new();
    };
    let Ok(mut revwalk) = repo.revwalk() else {
        return Vec::new();
    };
    if revwalk.push_head().is_err() {
        return Vec::new();
    }
    revwalk
        .take(20)
        .filter_map(Result::ok)
        .filter_map(|oid| repo.find_commit(oid).ok())
        .map(|commit| {
            let short = commit.id().to_string().chars().take(7).collect::<String>();
            let summary = commit.summary().unwrap_or("Unknown commit");
            format!("{short} {summary}")
        })
        .collect()
}

pub fn recent_changes(root: &Path, mode: &GitContextMode) -> Result<RecentChanges> {
    let repo = Repository::discover(root)
        .with_context(|| format!("Not a git repository: {}", root.display()))?;
    match mode {
        GitContextMode::Since(base) => diff_range(&repo, base, "HEAD"),
        GitContextMode::Diff { base, head } => diff_range(&repo, base, head),
        GitContextMode::Branch => {
            let base = default_main_ref(&repo)?;
            diff_range(&repo, &base, "HEAD")
        }
    }
}

fn diff_range(repo: &Repository, base: &str, head: &str) -> Result<RecentChanges> {
    let base_tree =
        rev_to_tree(repo, base).with_context(|| format!("Could not resolve git ref `{base}`"))?;
    let head_tree =
        rev_to_tree(repo, head).with_context(|| format!("Could not resolve git ref `{head}`"))?;
    let mut options = DiffOptions::new();
    let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), Some(&mut options))?;
    let stats = diff.stats()?;
    let mut changes = Vec::new();

    for idx in 0..diff.deltas().len() {
        let Some(delta) = diff.get_delta(idx) else {
            continue;
        };
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .map(normalize_path)
            .unwrap_or_else(|| "Unknown from git".to_owned());
        let (additions, deletions) = Patch::from_diff(&diff, idx)?
            .map(|patch| {
                let (_, additions, deletions) = patch.line_stats().unwrap_or((0, 0, 0));
                (additions, deletions)
            })
            .unwrap_or((0, 0));
        changes.push(GitChange {
            path,
            status: delta_status(delta.status()).to_owned(),
            additions,
            deletions,
        });
    }
    changes.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(RecentChanges {
        base: base.to_owned(),
        head: head.to_owned(),
        files_changed: stats.files_changed(),
        insertions: stats.insertions(),
        deletions: stats.deletions(),
        changes,
    })
}

fn rev_to_tree<'repo>(repo: &'repo Repository, rev: &str) -> Result<Tree<'repo>> {
    let object = repo.revparse_single(rev)?;
    Ok(object.peel_to_commit()?.tree()?)
}

fn default_main_ref(repo: &Repository) -> Result<String> {
    for candidate in [
        "refs/heads/main",
        "refs/remotes/origin/main",
        "refs/heads/master",
        "refs/remotes/origin/master",
    ] {
        if repo.find_reference(candidate).is_ok() {
            return Ok(candidate.to_owned());
        }
    }
    anyhow::bail!("Could not find main branch. Use `--diff <base>..<head>` instead.")
}

fn delta_status(delta: Delta) -> &'static str {
    match delta {
        Delta::Added => "added",
        Delta::Deleted => "deleted",
        Delta::Modified => "modified",
        Delta::Renamed => "renamed",
        Delta::Copied => "copied",
        Delta::Typechange => "typechange",
        Delta::Untracked => "untracked",
        Delta::Ignored => "ignored",
        Delta::Conflicted => "conflicted",
        _ => "unknown",
    }
}

fn relativize_to_root(
    _repo: &Repository,
    workdir: Option<&Path>,
    root: &Path,
    path: &str,
) -> String {
    let Some(workdir) = workdir else {
        return path.to_owned();
    };
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let absolute = workdir.join(path);
    absolute
        .strip_prefix(root)
        .map(normalize_path)
        .unwrap_or_else(|_| path.to_owned())
}

fn normalize_path(path: impl AsRef<Path>) -> String {
    let path: PathBuf = path.as_ref().components().collect();
    path.to_string_lossy().replace('\\', "/")
}
