use std::{path::Path, process::Command};

pub fn current_branch(root: &Path) -> Option<String> {
    run_git(root, &["rev-parse", "--abbrev-ref", "HEAD"])
}

pub fn recent_changed_files(root: &Path) -> Vec<String> {
    run_git(root, &["status", "--short"])
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.get(3..).map(str::trim))
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub fn recent_commit_subjects(root: &Path) -> Vec<String> {
    run_git(root, &["log", "--oneline", "-n", "20"])
        .unwrap_or_default()
        .lines()
        .map(ToOwned::to_owned)
        .collect()
}

fn run_git(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}
