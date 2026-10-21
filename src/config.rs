//! Configuration management

use std::{fs, io::ErrorKind, path::Path};

use anyhow::{Context, Result};

use serde::Deserialize;

/// Application configuration
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub refresh: RefreshConfig,
    pub graph: GraphConfig,
    pub ui: UiConfig,
}

/// Commit graph display configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct GraphConfig {
    /// Show remote branches and commits only reachable from remote branches
    pub show_remote_branches: bool,
    pub show_tags: bool,
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            show_remote_branches: true,
            show_tags: true,
        }
    }
}

/// Where the file diff is drawn
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum FileView {
    /// Take over the whole screen
    #[default]
    Fullscreen,
    /// Take over the graph pane, keeping commit detail and file list visible
    Replace,
}

/// UI layout configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// UI language: auto or an ISO language code; Git content keeps its original language.
    pub language: String,
    pub file_view: FileView,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            language: "auto".into(),
            file_view: FileView::default(),
        }
    }
}

/// Auto-refresh configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RefreshConfig {
    /// Enable auto-refresh for local state (commits, branches, working tree)
    pub auto_refresh: bool,
    /// Interval in seconds for local refresh (minimum: 1, default: 10)
    #[serde(deserialize_with = "deserialize_refresh_interval")]
    pub refresh_interval: u64,
    /// Enable auto-fetch from remote
    pub auto_fetch: bool,
    /// Interval in seconds for remote fetch (minimum: 10, default: 60)
    #[serde(deserialize_with = "deserialize_fetch_interval")]
    pub fetch_interval: u64,
}

impl Default for RefreshConfig {
    fn default() -> Self {
        Self {
            auto_refresh: true,
            refresh_interval: 10,
            auto_fetch: true,
            fetch_interval: 60,
        }
    }
}

fn deserialize_refresh_interval<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = u64::deserialize(deserializer)?;
    Ok(value.max(1))
}

fn deserialize_fetch_interval<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = u64::deserialize(deserializer)?;
    Ok(value.max(10))
}

impl Config {
    /// Load config, using defaults only when the file is absent.
    pub fn load() -> Result<Self> {
        match dirs::config_dir() {
            Some(dir) => Self::load_path(&dir.join("lazykeifu/config.toml")),
            None => Ok(Self::default()),
        }
    }

    fn load_path(path: &Path) -> Result<Self> {
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("Failed to read config {}", path.display()));
            }
        };
        toml::from_str(&content).with_context(|| format!("Invalid config {}", path.display()))
    }
}

#[cfg(test)]
mod language_tests {
    use super::*;

    #[test]
    fn language_defaults_to_auto_and_accepts_explicit_choice() {
        assert_eq!(Config::default().ui.language, "auto");
        let config: Config = toml::from_str("[ui]\nlanguage = 'zh-TW'\n").unwrap();
        assert_eq!(config.ui.language, "zh-TW");
        assert!(matches!(config.ui.file_view, FileView::Fullscreen));
    }
}

#[cfg(test)]
mod loading_tests {
    use super::*;

    #[test]
    fn absent_config_uses_defaults_but_valid_config_preserves_preferences() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        assert!(Config::load_path(&path).unwrap().refresh.auto_fetch);
        fs::write(&path, "[refresh]\nauto_fetch=false\n").unwrap();
        assert!(!Config::load_path(&path).unwrap().refresh.auto_fetch);
    }

    #[test]
    fn invalid_config_reports_path_and_cause_instead_of_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        for content in [
            "[refresh",
            "[refresh]\nauto_fetch=false\nfetch_interval='broken'",
            "\0",
        ] {
            fs::write(&path, content).unwrap();
            let error = Config::load_path(&path).unwrap_err();
            assert!(error.to_string().contains(&path.display().to_string()));
            assert!(error.chain().count() > 1);
        }
    }

    #[test]
    fn unreadable_config_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let error = Config::load_path(dir.path()).unwrap_err();
        assert!(error.to_string().contains("Failed to read config"));
        assert!(error.chain().count() > 1);
    }
}
