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
            us_tickers: vec![
                "NKE".to_string(),
                "TJX".to_string(),
                "VFC".to_string(),
            ],
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
