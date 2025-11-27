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
    match fs::read_to_string(&config_path) {
        Ok(config_str) => {
            match toml::from_str(&config_str) {
                Ok(config) => Ok(config),
                Err(e) => {
                    eprintln!("Failed to parse config.toml: {}", e); // Log error
                    Err(e.into())
                }
            }
        }
        Err(e) => {
            eprintln!(
                "Failed to read config.toml from path {:?}: {}",
                config_path, e
            ); // Log error
            Err(e.into())
        }
    }
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
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_default_has_tickers() {
        // Default config should have some tickers as fallback
        let default_config = Config {
            non_us_tickers: vec![
                "C.PA".to_string(),
                "LVMH.PA".to_string(),
                "ITX.MC".to_string(),
            ],
            us_tickers: vec!["NKE".to_string(), "TJX".to_string(), "VFC".to_string()],
        };

        assert!(!default_config.non_us_tickers.is_empty());
        assert!(!default_config.us_tickers.is_empty());
        assert!(default_config.non_us_tickers.contains(&"C.PA".to_string()));
        assert!(default_config.us_tickers.contains(&"NKE".to_string()));
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = Config {
            non_us_tickers: vec!["MC.PA".to_string(), "9983.T".to_string()],
            us_tickers: vec!["NKE".to_string(), "LULU".to_string()],
        };

        // Serialize to TOML
        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize config");

        // Deserialize back
        let parsed_config: Config =
            toml::from_str(&toml_str).expect("Failed to deserialize config");

        assert_eq!(config.non_us_tickers, parsed_config.non_us_tickers);
        assert_eq!(config.us_tickers, parsed_config.us_tickers);
    }

    #[test]
    fn test_config_deserialization_from_toml_string() {
        let toml_content = r#"
non_us_tickers = ["MC.PA", "ITX.MC"]
us_tickers = ["NKE", "GPS"]
"#;

        let config: Config = toml::from_str(toml_content).expect("Failed to parse TOML");

        assert_eq!(config.non_us_tickers.len(), 2);
        assert_eq!(config.us_tickers.len(), 2);
        assert_eq!(config.non_us_tickers[0], "MC.PA");
        assert_eq!(config.us_tickers[0], "NKE");
    }

    #[test]
    fn test_config_empty_arrays() {
        let toml_content = r#"
non_us_tickers = []
us_tickers = []
"#;

        let config: Config = toml::from_str(toml_content).expect("Failed to parse TOML");

        assert!(config.non_us_tickers.is_empty());
        assert!(config.us_tickers.is_empty());
    }

    #[test]
    fn test_config_with_special_ticker_symbols() {
        let config = Config {
            non_us_tickers: vec![
                "HM-B.ST".to_string(), // Hyphen and dot
                "9983.T".to_string(),  // Starts with number
                "BRK.A".to_string(),   // Berkshire style
                "LVMH.PA".to_string(), // Two-letter exchange
            ],
            us_tickers: vec!["BRK.B".to_string()],
        };

        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize");
        let parsed: Config = toml::from_str(&toml_str).expect("Failed to deserialize");

        assert_eq!(config.non_us_tickers, parsed.non_us_tickers);
        assert!(parsed.non_us_tickers.contains(&"HM-B.ST".to_string()));
    }

    #[test]
    fn test_invalid_toml_syntax() {
        let invalid_toml = r#"
non_us_tickers = ["MC.PA"
us_tickers = ["NKE"]
"#;

        let result: Result<Config, _> = toml::from_str(invalid_toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_field_in_toml() {
        // Missing us_tickers field should fail
        let toml_content = r#"
non_us_tickers = ["MC.PA"]
"#;

        let result: Result<Config, _> = toml::from_str(toml_content);
        assert!(result.is_err());
    }

    #[test]
    fn test_save_and_load_config_to_temp_file() {
        let config = Config {
            non_us_tickers: vec!["TEST.PA".to_string()],
            us_tickers: vec!["TEST".to_string()],
        };

        // Create a temp file
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize");
        temp_file
            .write_all(toml_str.as_bytes())
            .expect("Failed to write");

        // Read it back
        let content = fs::read_to_string(temp_file.path()).expect("Failed to read");
        let loaded: Config = toml::from_str(&content).expect("Failed to parse");

        assert_eq!(config.non_us_tickers, loaded.non_us_tickers);
        assert_eq!(config.us_tickers, loaded.us_tickers);
    }
}
