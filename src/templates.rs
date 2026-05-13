use crate::{context, git, token_budget};
use anyhow::{Context, Result};
use serde::Serialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct TemplateOptions {
    pub budget: Option<usize>,
    pub git_mode: Option<git::GitContextMode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateInfo {
    pub name: String,
    pub source: String,
}

struct BuiltinTemplate {
    name: &'static str,
    body: &'static str,
}

const BUILTIN_TEMPLATES: &[BuiltinTemplate] = &[
    BuiltinTemplate {
        name: "bugfix",
        body: include_str!("../templates/bugfix.md"),
    },
    BuiltinTemplate {
        name: "refactor",
        body: include_str!("../templates/refactor.md"),
    },
    BuiltinTemplate {
        name: "review",
        body: include_str!("../templates/review.md"),
    },
    BuiltinTemplate {
        name: "docs",
        body: include_str!("../templates/docs.md"),
    },
    BuiltinTemplate {
        name: "test",
        body: include_str!("../templates/test.md"),
    },
];

pub fn render_prompt(
    root: impl AsRef<Path>,
    name: &str,
    task: &str,
    options: TemplateOptions,
) -> Result<String> {
    let root = root.as_ref();
    let template = load_template(root, name)?;
    let packet = context::build_context_packet_with_options(
        root,
        task,
        options.budget,
        context::ContextOptions {
            git_mode: options.git_mode.clone(),
            semantic: false,
            semantic_alpha: 0.45,
        },
    )?;
    let recent_changes = render_recent_changes(root, options.git_mode.as_ref())?;

    Ok(template
        .replace("{{task}}", task)
        .replace("{{context}}", &packet)
        .replace("{{recent_changes}}", &recent_changes))
}

pub fn list_templates(root: impl AsRef<Path>) -> Vec<TemplateInfo> {
    let root = root.as_ref();
    let mut templates = BUILTIN_TEMPLATES
        .iter()
        .map(|template| TemplateInfo {
            name: template.name.to_owned(),
            source: "built-in".to_owned(),
        })
        .collect::<Vec<_>>();

    for dir in custom_template_dirs(root) {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            if templates.iter().any(|template| template.name == stem) {
                continue;
            }
            templates.push(TemplateInfo {
                name: stem.to_owned(),
                source: path.display().to_string(),
            });
        }
    }

    templates.sort_by(|a, b| a.name.cmp(&b.name));
    templates
}

fn load_template(root: &Path, name: &str) -> Result<String> {
    validate_template_name(name)?;

    for dir in custom_template_dirs(root) {
        let path = dir.join(format!("{name}.md"));
        if path.exists() {
            return fs::read_to_string(&path)
                .with_context(|| format!("Could not read template {}", path.display()));
        }
    }

    BUILTIN_TEMPLATES
        .iter()
        .find(|template| template.name == name)
        .map(|template| template.body.to_owned())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown template `{name}`. Available templates: {}",
                list_templates(root)
                    .into_iter()
                    .map(|template| template.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

fn validate_template_name(name: &str) -> Result<()> {
    if name.is_empty()
        || !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        anyhow::bail!("Template name must contain only letters, numbers, `-`, and `_`.");
    }
    Ok(())
}

fn custom_template_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![
        root.join("scoutpack/templates"),
        root.join(".scoutpack/templates"),
    ];
    if let Some(home) = env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".config/scoutpack/templates"));
    }
    dirs
}

fn render_recent_changes(root: &Path, mode: Option<&git::GitContextMode>) -> Result<String> {
    let Some(mode) = mode else {
        return Ok("Recent changes:\n- No git range requested.\n".to_owned());
    };
    let changes = git::recent_changes(root, mode)?;
    let mut out = String::new();
    out.push_str("Recent changes:\n");
    out.push_str(&format!(
        "- Range: `{}`..`{}`\n",
        changes.base, changes.head
    ));
    out.push_str(&format!(
        "- Summary: {} files changed, +{} -{}\n",
        changes.files_changed, changes.insertions, changes.deletions
    ));
    for change in changes.changes.iter().take(20) {
        out.push_str(&format!(
            "- `{}`: {}, +{} -{}\n",
            change.path, change.status, change.additions, change.deletions
        ));
    }
    Ok(out)
}

pub fn render_summary(
    task: &str,
    prompt: &str,
    name: &str,
    budget: Option<usize>,
) -> serde_json::Value {
    serde_json::json!({
        "template": name,
        "task": task,
        "budget": budget,
        "estimated_tokens": token_budget::estimate_tokens(prompt),
        "prompt": prompt,
    })
}

#[cfg(test)]
mod tests {
    use super::{list_templates, validate_template_name};
    use std::path::Path;

    #[test]
    fn built_in_templates_are_listed() {
        let names = list_templates(Path::new("."))
            .into_iter()
            .map(|template| template.name)
            .collect::<Vec<_>>();
        assert!(names.contains(&"bugfix".to_owned()));
        assert!(names.contains(&"review".to_owned()));
        assert!(names.contains(&"test".to_owned()));
    }

    #[test]
    fn template_names_are_restricted() {
        assert!(validate_template_name("bugfix").is_ok());
        assert!(validate_template_name("../secret").is_err());
        assert!(validate_template_name("bad/name").is_err());
    }
}
