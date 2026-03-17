//! Config storage at `~/.config/agentgif/config.json`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub api_key: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub username: String,
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("agentgif")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn load_config() -> Config {
    match fs::read_to_string(config_path()) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

pub fn save_config(cfg: &Config) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(config_dir())?;
    let data = serde_json::to_string_pretty(cfg)?;
    fs::write(config_path(), format!("{data}\n"))?;
    Ok(())
}

pub fn get_api_key() -> String {
    load_config().api_key
}

pub fn save_credentials(api_key: &str, username: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg = load_config();
    cfg.api_key = api_key.to_string();
    cfg.username = username.to_string();
    save_config(&cfg)
}

pub fn clear_credentials() -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg = load_config();
    cfg.api_key = String::new();
    cfg.username = String::new();
    save_config(&cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let cfg = Config::default();
        assert!(cfg.api_key.is_empty());
        assert!(cfg.username.is_empty());
    }

    #[test]
    fn test_config_roundtrip() {
        let cfg = Config {
            api_key: "test-key".into(),
            username: "testuser".into(),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let loaded: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.api_key, "test-key");
        assert_eq!(loaded.username, "testuser");
    }

    #[test]
    fn test_config_omits_empty() {
        let cfg = Config::default();
        let json = serde_json::to_string(&cfg).unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_config_dir_not_empty() {
        let dir = config_dir();
        assert!(!dir.to_string_lossy().is_empty());
    }
}
