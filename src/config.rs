use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

pub const DEFAULT_CONFIG_FILE: &str = "scoutpack.toml";
pub const DEFAULT_IGNORE_FILE: &str = ".scoutpackignore";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoutpackConfig {
    pub max_file_size_kb: u64,
    pub default_budget: usize,
    pub languages: LanguageConfig,
    pub ranking: RankingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConfig {
    #[serde(default = "default_true")]
    pub typescript: bool,
    #[serde(default = "default_true")]
    pub python: bool,
    #[serde(default = "default_true")]
    pub rust: bool,
    #[serde(default = "default_true")]
    pub go: bool,
    #[serde(default = "default_true")]
    pub solidity: bool,
    #[serde(default = "default_true")]
    pub markdown: bool,
    #[serde(default = "default_true")]
    pub json: bool,
    #[serde(default = "default_true")]
    pub yaml: bool,
    #[serde(default = "default_true")]
    pub toml: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingConfig {
    pub recent_file_boost: f64,
    pub path_match_boost: f64,
    pub symbol_match_boost: f64,
}

impl Default for ScoutpackConfig {
    fn default() -> Self {
        Self {
            max_file_size_kb: 512,
            default_budget: 3000,
            languages: LanguageConfig {
                typescript: true,
                python: true,
                rust: true,
                go: true,
                solidity: true,
                markdown: true,
                json: true,
                yaml: true,
                toml: true,
            },
            ranking: RankingConfig {
                recent_file_boost: 0.15,
                path_match_boost: 0.25,
                symbol_match_boost: 0.35,
            },
        }
    }
}

pub fn init_project(root: &Path) -> Result<()> {
    fs::create_dir_all(root).with_context(|| format!("Could not create {}", root.display()))?;

    let config_path = root.join(DEFAULT_CONFIG_FILE);
    if !config_path.exists() {
        fs::write(&config_path, default_config_text())
            .with_context(|| format!("Could not write {}", config_path.display()))?;
    }

    let ignore_path = root.join(DEFAULT_IGNORE_FILE);
    if !ignore_path.exists() {
        fs::write(&ignore_path, default_ignore_text())
            .with_context(|| format!("Could not write {}", ignore_path.display()))?;
    }

    Ok(())
}

pub fn load(root: &Path) -> Result<ScoutpackConfig> {
    let config_path = root.join(DEFAULT_CONFIG_FILE);
    if !config_path.exists() {
        return Ok(ScoutpackConfig::default());
    }

    let text = fs::read_to_string(&config_path)
        .with_context(|| format!("Could not read {}", config_path.display()))?;
    toml::from_str(&text).with_context(|| format!("Invalid {}", config_path.display()))
}

pub fn config_hash(config: &ScoutpackConfig) -> Result<String> {
    let text = toml::to_string(config)?;
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    Ok(hex::encode(hasher.finalize()))
}

pub fn default_config_text() -> String {
    toml::to_string_pretty(&ScoutpackConfig::default()).expect("default config serializes")
}

pub fn default_ignore_text() -> &'static str {
    ".git\n.scoutpack\nnode_modules\n.next\ndist\nbuild\ncoverage\n.turbo\n.cache\ntarget\n.venv\nvendor\n"
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid_toml() {
        let parsed: ScoutpackConfig = toml::from_str(&default_config_text()).unwrap();
        assert_eq!(parsed.max_file_size_kb, 512);
        assert!(parsed.languages.typescript);
        assert!(parsed.languages.python);
        assert!(parsed.languages.rust);
        assert!(parsed.languages.go);
        assert!(parsed.languages.solidity);
    }

    #[test]
    fn old_language_config_defaults_new_languages_on() {
        let parsed: ScoutpackConfig = toml::from_str(
            r#"
max_file_size_kb = 512
default_budget = 3000

[languages]
typescript = true
markdown = true
json = true
yaml = true
toml = true

[ranking]
recent_file_boost = 0.15
path_match_boost = 0.25
symbol_match_boost = 0.35
"#,
        )
        .unwrap();
        assert!(parsed.languages.python);
        assert!(parsed.languages.rust);
        assert!(parsed.languages.go);
        assert!(parsed.languages.solidity);
    }
}
