use anyhow::{Context, Result};
use git2::{Delta, DiffFindOptions, DiffOptions, Patch, Repository, StatusOptions, Tree};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_path: Option<String>,
    pub status: String,
    pub additions: usize,
    pub deletions: usize,
}

impl GitChange {
    pub fn summary_line(&self) -> String {
        let path = match &self.previous_path {
            Some(previous) => format!("`{previous}` -> `{}`", self.path),
            None => format!("`{}`", self.path),
        };
        format!(
            "- {path}: {}, +{} -{}\n",
            self.status, self.additions, self.deletions
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentChanges {
    pub base: String,
    pub head: String,
    pub base_commit: String,
    pub head_commit: String,
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
            let base_id = repo.revparse_single(&base)?.peel_to_commit()?.id();
            let head_id = repo.revparse_single("HEAD")?.peel_to_commit()?.id();
            let merge_base = repo.merge_base(base_id, head_id)?;
            diff_range(&repo, &merge_base.to_string(), &head_id.to_string())
        }
    }
}

fn diff_range(repo: &Repository, base: &str, head: &str) -> Result<RecentChanges> {
    let base_commit = repo
        .revparse_single(base)
        .and_then(|obj| obj.peel_to_commit())
        .with_context(|| format!("Could not resolve git ref `{base}`"))?;
    let head_commit = repo
        .revparse_single(head)
        .and_then(|obj| obj.peel_to_commit())
        .with_context(|| format!("Could not resolve git ref `{head}`"))?;
    let base_tree = base_commit.tree()?;
    let head_tree = head_commit.tree()?;
    let mut options = DiffOptions::new();
    let mut diff =
        repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), Some(&mut options))?;
    // Tree diffs start with additions/deletions. Resolve moves before counting
    // files or creating patches, using the same policy regardless of Git config.
    diff.find_similar(Some(
        DiffFindOptions::new().renames(true).rename_threshold(50),
    ))?;
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
            previous_path: (delta.status() == Delta::Renamed)
                .then(|| delta.old_file().path().map(normalize_path))
                .flatten(),
            status: delta_status(delta.status()).to_owned(),
            additions,
            deletions,
        });
    }
    changes.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(RecentChanges {
        base: base.to_owned(),
        head: head.to_owned(),
        base_commit: base_commit.id().to_string(),
        head_commit: head_commit.id().to_string(),
        files_changed: stats.files_changed(),
        insertions: stats.insertions(),
        deletions: stats.deletions(),
        changes,
    })
}

/// Materialize regular, eligible blobs only; never check out symlinks or execute hooks.
pub fn snapshot(root: &Path, commit: &str) -> Result<tempfile::TempDir> {
    let repo = Repository::discover(root)?;
    let workdir = repo
        .workdir()
        .context("Git context requires a working repository")?
        .canonicalize()?;
    anyhow::ensure!(
        root.canonicalize()? == workdir,
        "Git context must run from repository root: {}",
        workdir.display()
    );
    let tree = repo.find_commit(git2::Oid::from_str(commit)?)?.tree()?;
    let snapshot = tempfile::tempdir()?;
    let config = crate::config::load(root)?;
    write_tree(
        &repo,
        &tree,
        Path::new(""),
        snapshot.path(),
        config.max_file_size_kb.saturating_mul(1024),
    )?;
    // Use the user's current restrictive policy as well as committed ignore rules.
    for name in [".scoutpackignore", ".gitignore"] {
        let local = root.join(name);
        if local.is_file() {
            let mut rules = std::fs::read_to_string(snapshot.path().join(name)).unwrap_or_default();
            rules.push('\n');
            rules.push_str(&std::fs::read_to_string(local)?);
            std::fs::write(snapshot.path().join(name), rules)?;
        }
    }
    // Make the walker honor committed .gitignore files without consulting parent repos.
    std::fs::create_dir(snapshot.path().join(".git"))?;
    crate::index::pack_repo(snapshot.path())?;
    Ok(snapshot)
}

fn write_tree(
    repo: &Repository,
    tree: &Tree<'_>,
    relative: &Path,
    destination: &Path,
    max_bytes: u64,
) -> Result<()> {
    for entry in tree {
        let name = entry
            .name()
            .context("Git snapshot contains a non-UTF8 path")?;
        anyhow::ensure!(
            name != "." && name != ".." && !name.contains(['/', '\\']),
            "Unsafe Git path"
        );
        let path = relative.join(name);
        if crate::scanner::is_default_ignored(&path) || crate::scanner::is_sensitive_path(&path) {
            continue;
        }
        if entry.kind() == Some(git2::ObjectType::Tree) {
            write_tree(
                repo,
                &repo.find_tree(entry.id())?,
                &path,
                destination,
                max_bytes,
            )?;
        } else if matches!(entry.filemode(), 0o100644 | 0o100755)
            && (crate::scanner::classify_path(&path).is_some()
                || matches!(name, ".gitignore" | ".scoutpackignore"))
        {
            let blob = repo.find_blob(entry.id())?;
            if blob.size() as u64 > max_bytes {
                continue;
            }
            let target = destination.join(path);
            std::fs::create_dir_all(target.parent().context("Missing snapshot parent")?)?;
            std::fs::write(target, blob.content())?;
        }
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn commit_all(repo: &Repository) -> String {
        let mut index = repo.index().unwrap();
        index
            .add_all(["."], git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.update_all(["."], None).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let signature = git2::Signature::now("ScoutPack Test", "test@example.com").unwrap();
        let parent = repo.head().ok().map(|head| head.peel_to_commit().unwrap());
        let parents: Vec<_> = parent.iter().collect();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "fixture",
            &tree,
            &parents,
        )
        .unwrap()
        .to_string()
    }

    #[test]
    fn rename_summary_counts_a_move_once_and_keeps_real_deletions() {
        let temp = tempfile::tempdir().unwrap();
        let repo = Repository::init(temp.path()).unwrap();
        // Results should not depend on the user's diff.renames setting.
        repo.config()
            .unwrap()
            .set_bool("diff.renames", false)
            .unwrap();
        fs::write(
            temp.path().join("before.ts"),
            "export const login = true;\n",
        )
        .unwrap();
        fs::write(
            temp.path().join("removed.ts"),
            "export const obsolete = 42;\n",
        )
        .unwrap();
        let base = commit_all(&repo);
        fs::rename(temp.path().join("before.ts"), temp.path().join("after.ts")).unwrap();
        fs::remove_file(temp.path().join("removed.ts")).unwrap();
        let head = commit_all(&repo);

        let summary = diff_range(&repo, &base, &head).unwrap();
        assert_eq!(summary.files_changed, 2);
        assert_eq!((summary.insertions, summary.deletions), (0, 1));
        assert_eq!(summary.changes.len(), 2);
        let moved = &summary.changes[0];
        assert_eq!(moved.path, "after.ts");
        assert_eq!(moved.status, "renamed");
        assert_eq!((moved.additions, moved.deletions), (0, 0));
        let json = serde_json::to_value(moved).unwrap();
        assert_eq!(json["previous_path"], "before.ts");
        let deleted = &summary.changes[1];
        assert_eq!(deleted.path, "removed.ts");
        assert_eq!(deleted.status, "deleted");
        assert!(serde_json::to_value(deleted)
            .unwrap()
            .get("previous_path")
            .is_none());
    }

    #[test]
    fn rename_with_edits_keeps_line_counts_and_does_not_pair_unrelated_files() {
        let temp = tempfile::tempdir().unwrap();
        let repo = Repository::init(temp.path()).unwrap();
        let source = (0..30)
            .map(|n| format!("export const login{n} = {n};\n"))
            .collect::<String>();
        fs::write(temp.path().join("old.ts"), &source).unwrap();
        fs::write(temp.path().join("removed.md"), "obsolete documentation\n").unwrap();
        let base = commit_all(&repo);
        fs::remove_file(temp.path().join("old.ts")).unwrap();
        fs::write(
            temp.path().join("new.ts"),
            source.replace("login0 = 0", "login0 = 99"),
        )
        .unwrap();
        fs::remove_file(temp.path().join("removed.md")).unwrap();
        fs::write(
            temp.path().join("added.py"),
            "print('unrelated addition')\n",
        )
        .unwrap();
        let head = commit_all(&repo);
        let summary = diff_range(&repo, &base, &head).unwrap();
        assert_eq!(summary.files_changed, 3);
        assert_eq!((summary.insertions, summary.deletions), (2, 2));
        assert_eq!(
            summary
                .changes
                .iter()
                .map(|c| (c.path.as_str(), c.status.as_str()))
                .collect::<Vec<_>>(),
            [
                ("added.py", "added"),
                ("new.ts", "renamed"),
                ("removed.md", "deleted")
            ]
        );
        let moved = &summary.changes[1];
        assert_eq!((moved.additions, moved.deletions), (1, 1));
        assert_eq!(
            serde_json::to_value(moved).unwrap()["previous_path"],
            "old.ts"
        );
    }
}
