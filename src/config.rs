//! 설정 파일 지원 (~/.config/diffy/config.toml)

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    #[serde(default)]
    pub defaults: Defaults,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Defaults {
    pub highlight: bool,
    pub mouse: bool,
    pub view: ViewMode,
    pub file_tree: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ViewMode {
    Unified,
    SideBySide,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            highlight: false,
            mouse: false,
            view: ViewMode::Unified,
            file_tree: true,
        }
    }
}

/// Load config from ~/.config/diffy/config.toml (or XDG_CONFIG_HOME)
pub fn load() -> Config {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) => match toml::from_str(&contents) {
            Ok(config) => config,
            Err(e) => {
                eprintln!(
                    "[diffy] warning: invalid config file {}: {}",
                    path.display(),
                    e
                );
                Config::default()
            }
        },
        Err(_) => Config::default(), // file doesn't exist, use defaults silently
    }
}

fn config_path() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("diffy").join("config.toml")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
            .join(".config")
            .join("diffy")
            .join("config.toml")
    } else {
        PathBuf::from(".config").join("diffy").join("config.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(!config.defaults.highlight);
        assert!(!config.defaults.mouse);
        assert_eq!(config.defaults.view, ViewMode::Unified);
        assert!(config.defaults.file_tree);
    }

    #[test]
    fn test_parse_valid_toml() {
        let toml_str = r#"
[defaults]
highlight = true
mouse = true
view = "side-by-side"
file_tree = false
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.defaults.highlight);
        assert!(config.defaults.mouse);
        assert_eq!(config.defaults.view, ViewMode::SideBySide);
        assert!(!config.defaults.file_tree);
    }

    #[test]
    fn test_parse_partial_toml() {
        let toml_str = r#"
[defaults]
highlight = true
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.defaults.highlight);
        assert!(!config.defaults.mouse); // default
        assert_eq!(config.defaults.view, ViewMode::Unified); // default
    }

    #[test]
    fn test_parse_empty_toml() {
        let config: Config = toml::from_str("").unwrap();
        assert!(!config.defaults.highlight);
        assert!(!config.defaults.mouse);
    }

    #[test]
    fn test_parse_invalid_toml() {
        let result: Result<Config, _> = toml::from_str("not valid [[[ toml");
        assert!(result.is_err());
    }
}
