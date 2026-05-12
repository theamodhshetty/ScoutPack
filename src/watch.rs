use crate::index;
use anyhow::{Context, Result};
use notify::{
    event::{CreateKind, ModifyKind, RemoveKind},
    Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::{
    path::{Component, Path, PathBuf},
    sync::mpsc,
    time::{Duration, Instant},
};

const WATCH_IGNORED_DIRS: &[&str] = &[
    ".git",
    ".scoutpack",
    "node_modules",
    ".next",
    "dist",
    "build",
    "coverage",
    ".turbo",
    ".cache",
    "target",
    ".venv",
    "vendor",
];

pub fn watch_repo(path: PathBuf, debounce: Duration) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("Path does not exist: {}", path.display());
    }
    let root = path
        .canonicalize()
        .with_context(|| format!("Could not resolve path {}", path.display()))?;
    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(
        move |event| {
            let _ = tx.send(event);
        },
        Config::default(),
    )?;
    watcher.watch(&root, RecursiveMode::Recursive)?;

    let initial = index::pack_repo(&root)?;
    eprintln!(
        "[scoutpack] watching {} (indexed {} files, reused {} unchanged, skipped {})",
        root.display(),
        initial.files_indexed,
        initial.files_reused,
        initial.files_skipped
    );

    let mut pending = false;
    let mut last_event = Instant::now();
    loop {
        let timeout = if pending {
            debounce.saturating_sub(last_event.elapsed())
        } else {
            Duration::from_secs(3600)
        };

        match rx.recv_timeout(timeout) {
            Ok(Ok(event)) => {
                if should_reindex(&root, &event) {
                    pending = true;
                    last_event = Instant::now();
                }
            }
            Ok(Err(error)) => {
                eprintln!("[scoutpack] watch error: {error}");
            }
            Err(mpsc::RecvTimeoutError::Timeout) if pending => {
                let start = Instant::now();
                match index::pack_repo(&root) {
                    Ok(summary) => {
                        eprintln!(
                            "[scoutpack] reindexed {} files in {}ms (reused {}, removed {}, skipped {})",
                            summary.files_indexed,
                            start.elapsed().as_millis(),
                            summary.files_reused,
                            summary.files_removed,
                            summary.files_skipped
                        );
                    }
                    Err(error) => eprintln!("[scoutpack] reindex failed: {error}"),
                }
                pending = false;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                anyhow::bail!("File watcher disconnected");
            }
        }
    }
}

fn should_reindex(root: &Path, event: &Event) -> bool {
    if !is_relevant_kind(&event.kind) {
        return false;
    }
    event.paths.iter().any(|path| is_watchable_path(root, path))
}

fn is_relevant_kind(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(CreateKind::File | CreateKind::Any)
            | EventKind::Modify(
                ModifyKind::Data(_)
                    | ModifyKind::Name(_)
                    | ModifyKind::Metadata(_)
                    | ModifyKind::Any
            )
            | EventKind::Remove(RemoveKind::File | RemoveKind::Any)
            | EventKind::Any
    )
}

fn is_watchable_path(root: &Path, path: &Path) -> bool {
    let rel = path.strip_prefix(root).unwrap_or(path);
    if rel.components().any(|component| match component {
        Component::Normal(part) => {
            let part = part.to_string_lossy();
            WATCH_IGNORED_DIRS
                .iter()
                .any(|ignored| *ignored == part.as_ref())
        }
        _ => false,
    }) {
        return false;
    }
    true
}
