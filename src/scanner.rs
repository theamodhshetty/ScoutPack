use crate::config::ScoutpackConfig;
use anyhow::{Context, Result};
use ignore::WalkBuilder;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Component, Path},
    time::UNIX_EPOCH,
};

const DEFAULT_IGNORED_DIRS: &[&str] = &[
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

#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub rel_path: String,
    pub language: String,
    pub kind: String,
    pub size_bytes: u64,
    pub hash: String,
    pub mtime: i64,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct SkippedFile {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Default)]
pub struct ScanResult {
    pub files: Vec<ScannedFile>,
    pub skipped: Vec<SkippedFile>,
}

pub fn scan_repo(root: &Path, config: &ScoutpackConfig) -> Result<ScanResult> {
    if !root.exists() {
        anyhow::bail!("Path does not exist: {}", root.display());
    }

    let root = root
        .canonicalize()
        .with_context(|| format!("Could not resolve {}", root.display()))?;
    let mut builder = WalkBuilder::new(&root);
    builder
        .standard_filters(true)
        .hidden(false)
        .add_custom_ignore_filename(".scoutpackignore");

    let mut result = ScanResult::default();
    let max_bytes = config.max_file_size_kb.saturating_mul(1024);

    for entry in builder.build() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                result.skipped.push(SkippedFile {
                    path: "Unknown from index".to_owned(),
                    reason: err.to_string(),
                });
                continue;
            }
        };
        let path = entry.path();
        if path == root || path.is_dir() {
            continue;
        }

        let rel = match path.strip_prefix(&root) {
            Ok(rel) => rel,
            Err(_) => continue,
        };
        if is_default_ignored(rel) {
            continue;
        }
        let rel_path = normalize_path(rel);

        if is_sensitive_path(rel) {
            result.skipped.push(SkippedFile {
                path: rel_path,
                reason: "sensitive file pattern".to_owned(),
            });
            continue;
        }

        let Some((language, kind)) = classify_path(rel) else {
            result.skipped.push(SkippedFile {
                path: rel_path,
                reason: "unsupported file type".to_owned(),
            });
            continue;
        };
        if !language_enabled(&language, config) {
            result.skipped.push(SkippedFile {
                path: rel_path,
                reason: format!("language disabled: {language}"),
            });
            continue;
        }

        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(err) => {
                result.skipped.push(SkippedFile {
                    path: rel_path,
                    reason: err.to_string(),
                });
                continue;
            }
        };
        if metadata.len() > max_bytes {
            result.skipped.push(SkippedFile {
                path: rel_path,
                reason: format!("file exceeds {} KB", config.max_file_size_kb),
            });
            continue;
        }

        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(err) => {
                result.skipped.push(SkippedFile {
                    path: rel_path,
                    reason: err.to_string(),
                });
                continue;
            }
        };
        if is_binary(&bytes) {
            result.skipped.push(SkippedFile {
                path: rel_path,
                reason: "binary file".to_owned(),
            });
            continue;
        }
        let text = match String::from_utf8(bytes.clone()) {
            Ok(text) => text,
            Err(_) => {
                result.skipped.push(SkippedFile {
                    path: rel_path,
                    reason: "non-utf8 text".to_owned(),
                });
                continue;
            }
        };
        let hash = hash_bytes(&bytes);
        let mtime = metadata
            .modified()
            .ok()
            .and_then(|mtime| mtime.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or_default();

        result.files.push(ScannedFile {
            rel_path,
            language,
            kind,
            size_bytes: metadata.len(),
            hash,
            mtime,
            text,
        });
    }

    Ok(result)
}

pub fn classify_path(path: &Path) -> Option<(String, String)> {
    let file_name = path.file_name()?.to_string_lossy();
    let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
    match ext {
        "ts" | "tsx" => Some(("typescript".to_owned(), ts_kind(path))),
        "md" | "mdx" => Some(("markdown".to_owned(), "doc".to_owned())),
        "json" => Some(("json".to_owned(), json_kind(&file_name))),
        "yaml" | "yml" => Some(("yaml".to_owned(), "config".to_owned())),
        "toml" => Some(("toml".to_owned(), "config".to_owned())),
        _ => None,
    }
}

fn ts_kind(path: &Path) -> String {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if file_name == "route.ts" || file_name == "route.tsx" {
        "route-handler".to_owned()
    } else if file_name == "page.tsx" || file_name == "layout.tsx" {
        "route-component".to_owned()
    } else {
        "source".to_owned()
    }
}

fn json_kind(file_name: &str) -> String {
    if file_name == "package.json" {
        "package-manifest".to_owned()
    } else {
        "config".to_owned()
    }
}

fn language_enabled(language: &str, config: &ScoutpackConfig) -> bool {
    match language {
        "typescript" => config.languages.typescript,
        "markdown" => config.languages.markdown,
        "json" => config.languages.json,
        "yaml" => config.languages.yaml,
        "toml" => config.languages.toml,
        _ => false,
    }
}

fn is_default_ignored(rel: &Path) -> bool {
    rel.components().any(|component| match component {
        Component::Normal(part) => {
            let part = part.to_string_lossy();
            DEFAULT_IGNORED_DIRS
                .iter()
                .any(|ignored| *ignored == part.as_ref())
        }
        _ => false,
    })
}

fn is_sensitive_path(rel: &Path) -> bool {
    let file_name = rel.file_name().and_then(|name| name.to_str()).unwrap_or("");
    if file_name == ".env"
        || file_name.starts_with(".env.")
        || file_name == "id_rsa"
        || file_name == "id_ed25519"
        || file_name == "secrets.yaml"
        || file_name == "secrets.yml"
        || file_name == "secrets.json"
        || file_name == ".npmrc"
        || file_name == ".pypirc"
    {
        return true;
    }
    matches!(
        rel.extension().and_then(|ext| ext.to_str()).unwrap_or(""),
        "pem" | "key" | "p12" | "mobileprovision"
    )
}

fn is_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(8192).any(|byte| *byte == 0)
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn normalize_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn sensitive_env_is_skipped() {
        let temp = tempfile::tempdir().unwrap();
        let mut file = fs::File::create(temp.path().join(".env")).unwrap();
        writeln!(file, "SECRET=value").unwrap();

        let result = scan_repo(temp.path(), &ScoutpackConfig::default()).unwrap();
        assert!(result.files.is_empty());
        assert_eq!(result.skipped[0].reason, "sensitive file pattern");
    }

    #[test]
    fn sensitive_file_patterns_are_skipped() {
        let temp = tempfile::tempdir().unwrap();
        for name in [
            ".env",
            ".env.local",
            ".env.production",
            "deploy.pem",
            "private.key",
            "id_rsa",
            "id_ed25519",
            "secrets.yaml",
            "secrets.json",
            ".npmrc",
            ".pypirc",
        ] {
            fs::write(temp.path().join(name), "SECRET=value").unwrap();
        }
        fs::write(temp.path().join("src.ts"), "export const ok = true;").unwrap();

        let result = scan_repo(temp.path(), &ScoutpackConfig::default()).unwrap();
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].rel_path, "src.ts");
        let skipped_paths: Vec<_> = result
            .skipped
            .iter()
            .map(|file| file.path.as_str())
            .collect();
        for name in [
            ".env",
            ".env.local",
            ".env.production",
            "deploy.pem",
            "private.key",
            "id_rsa",
            "id_ed25519",
            "secrets.yaml",
            "secrets.json",
            ".npmrc",
            ".pypirc",
        ] {
            assert!(skipped_paths.contains(&name), "{name} not skipped");
        }
        assert!(result
            .skipped
            .iter()
            .all(|file| file.reason == "sensitive file pattern"));
    }

    #[test]
    fn binary_non_utf8_and_large_files_are_skipped() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("binary.json"), b"{\0}").unwrap();
        fs::write(temp.path().join("latin1.json"), [0xff, 0xfe, 0xfd]).unwrap();
        fs::write(temp.path().join("large.ts"), "x".repeat(2048)).unwrap();

        let mut config = ScoutpackConfig::default();
        config.max_file_size_kb = 1;
        let result = scan_repo(temp.path(), &config).unwrap();

        assert!(result.files.is_empty());
        assert!(result
            .skipped
            .iter()
            .any(|file| file.path == "binary.json" && file.reason == "binary file"));
        assert!(result
            .skipped
            .iter()
            .any(|file| file.path == "latin1.json" && file.reason == "non-utf8 text"));
        assert!(result
            .skipped
            .iter()
            .any(|file| file.path == "large.ts" && file.reason == "file exceeds 1 KB"));
    }

    #[test]
    fn generated_and_ignored_dirs_are_not_indexed() {
        let temp = tempfile::tempdir().unwrap();
        for dir in [
            "node_modules",
            ".git",
            ".next",
            "dist",
            "build",
            "coverage",
            "target",
            ".venv",
        ] {
            fs::create_dir_all(temp.path().join(dir)).unwrap();
            fs::write(
                temp.path().join(dir).join("generated.ts"),
                "export const generated = true;",
            )
            .unwrap();
        }
        fs::write(temp.path().join("src.ts"), "export const ok = true;").unwrap();

        let result = scan_repo(temp.path(), &ScoutpackConfig::default()).unwrap();
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].rel_path, "src.ts");
        assert!(!result
            .files
            .iter()
            .any(|file| file.rel_path.contains("generated.ts")));
    }
}
