// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub non_us_tickers: Vec<String>,
    pub us_tickers: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        // Try to read from config.toml first
        if let Ok(config) = load_config() {
            return config;
        }

        // Fallback to hardcoded defaults
        Self {
            non_us_tickers: vec![
                "C.PA".to_string(),
                "LVMH.PA".to_string(),
                "ITX.MC".to_string(),
            ],
            us_tickers: vec!["NKE".to_string(), "TJX".to_string(), "VFC".to_string()],
        }
    }
}

fn get_config_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("config.toml");
    path
}

pub fn load_config() -> anyhow::Result<Config> {
    let config_path = get_config_path();
    let config_str = fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&config_str)?;
    Ok(config)
}

#[allow(dead_code)]
pub fn save_config(config: &Config) -> anyhow::Result<()> {
    let config_path = get_config_path();
    let config_str = toml::to_string_pretty(config)?;
    fs::write(config_path, config_str)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        // Temporarily move config.toml if it exists
        let config_path = get_config_path();
        let backup_path = config_path.with_extension("toml.bak");
        if config_path.exists() {
            fs::rename(&config_path, &backup_path).unwrap();
        }

        // Run the test
        let config = Config::default();
        assert!(!config.us_tickers.is_empty());
        assert!(!config.non_us_tickers.is_empty());
        assert!(config.us_tickers.contains(&"NKE".to_string()));
        assert!(config.non_us_tickers.contains(&"C.PA".to_string()));

        // Restore config.toml if we moved it
        if backup_path.exists() {
            fs::rename(&backup_path, &config_path).unwrap();
        }
    }

    #[test]
    fn test_save_and_load_config() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;
        let temp_path = temp_dir.path().join("config.toml");

        // Set CARGO_MANIFEST_DIR to temp directory for testing
        env::set_var("CARGO_MANIFEST_DIR", temp_dir.path());

        let config = Config {
            us_tickers: vec!["TEST1".to_string()],
            non_us_tickers: vec!["TEST2.PA".to_string()],
        };

        // Save config
        save_config(&config)?;

        // Load and verify
        let loaded_config = load_config()?;
        assert_eq!(loaded_config.us_tickers, vec!["TEST1"]);
        assert_eq!(loaded_config.non_us_tickers, vec!["TEST2.PA"]);

        Ok(())
    }

    #[test]
    fn test_invalid_config_file() {
        let temp_dir = tempdir().unwrap();
        env::set_var("CARGO_MANIFEST_DIR", temp_dir.path());

        // Create invalid TOML file
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "invalid toml content").unwrap();

        // Should fall back to default config
        let config = Config::default();
        assert!(!config.us_tickers.is_empty());
        assert!(!config.non_us_tickers.is_empty());
    }
}
