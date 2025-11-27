// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::collections::HashSet;
use std::fs;
use toml::Value;

use crate::api::FMPClient;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoredSymbolChange {
    pub id: Option<i64>,
    pub old_symbol: String,
    pub new_symbol: String,
    pub change_date: Option<String>,
    pub company_name: Option<String>,
    pub reason: Option<String>,
    pub applied: i64, // SQLite uses INTEGER for boolean
}

#[derive(Debug, Serialize)]
pub struct SymbolChangeReport {
    pub pending_changes: Vec<StoredSymbolChange>,
    pub applicable_changes: Vec<StoredSymbolChange>,
    pub non_applicable_changes: Vec<StoredSymbolChange>,
    pub conflicts: Vec<String>,
}

/// Fetch symbol changes from FMP API and store in database
pub async fn fetch_and_store_symbol_changes(
    pool: &SqlitePool,
    fmp_client: &FMPClient,
) -> Result<usize> {
    println!("Fetching symbol changes from FMP API...");
    let changes = fmp_client.fetch_symbol_changes().await?;

    let mut stored_count = 0;
    for change in changes {
        let result = sqlx::query!(
            r#"
            INSERT INTO symbol_changes (old_symbol, new_symbol, change_date, company_name, reason)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(old_symbol, new_symbol, change_date) DO NOTHING
            "#,
            change.old_symbol,
            change.new_symbol,
            change.date,
            change.name,
            None::<String>, // Reason will be added later if available
        )
        .execute(pool)
        .await;

        if let Ok(result) = result {
            if result.rows_affected() > 0 {
                stored_count += 1;
            }
        }
    }

    println!("✅ Stored {} new symbol changes", stored_count);
    Ok(stored_count)
}

/// Get all pending (unapplied) symbol changes from database
pub async fn get_pending_changes(pool: &SqlitePool) -> Result<Vec<StoredSymbolChange>> {
    let changes = sqlx::query_as!(
        StoredSymbolChange,
        r#"
        SELECT 
            id as "id?",
            old_symbol,
            new_symbol,
            change_date,
            company_name,
            reason,
            applied as "applied!"
        FROM symbol_changes
        WHERE applied = 0
        ORDER BY change_date DESC, old_symbol
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(changes)
}

/// Check which symbol changes apply to our current configuration
pub async fn check_ticker_updates(
    pool: &SqlitePool,
    config_path: &str,
) -> Result<SymbolChangeReport> {
    let pending_changes = get_pending_changes(pool).await?;

    // Read current config
    let config_content = fs::read_to_string(config_path).context("Failed to read config.toml")?;
    let config: Value = toml::from_str(&config_content).context("Failed to parse config.toml")?;

    // Extract all current tickers
    let mut current_tickers = HashSet::new();

    if let Some(us_tickers) = config.get("us_tickers").and_then(|v| v.as_array()) {
        for ticker in us_tickers {
            if let Some(ticker_str) = ticker.as_str() {
                current_tickers.insert(ticker_str.to_string());
            }
        }
    }

    if let Some(non_us_tickers) = config.get("non_us_tickers").and_then(|v| v.as_array()) {
        for ticker in non_us_tickers {
            if let Some(ticker_str) = ticker.as_str() {
                current_tickers.insert(ticker_str.to_string());
            }
        }
    }

    // Categorize changes
    let mut applicable_changes = Vec::new();
    let mut non_applicable_changes = Vec::new();
    let mut conflicts = Vec::new();

    for change in &pending_changes {
        if current_tickers.contains(&change.old_symbol) {
            if current_tickers.contains(&change.new_symbol) {
                conflicts.push(format!(
                    "Both {} and {} exist in config (change date: {})",
                    change.old_symbol,
                    change.new_symbol,
                    change
                        .change_date
                        .as_ref()
                        .unwrap_or(&"unknown".to_string())
                ));
            } else {
                applicable_changes.push(change.clone());
            }
        } else {
            non_applicable_changes.push(change.clone());
        }
    }

    Ok(SymbolChangeReport {
        pending_changes,
        applicable_changes,
        non_applicable_changes,
        conflicts,
    })
}

/// Validate that a ticker symbol is safe to use in config file replacement
/// Prevents potential config file corruption from malformed symbols
fn is_valid_ticker_symbol(symbol: &str) -> bool {
    // Ticker symbols should only contain alphanumeric chars, dots, and hyphens
    // and should be reasonably short (max 20 chars)
    !symbol.is_empty()
        && symbol.len() <= 20
        && symbol
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

/// Apply ticker updates to the configuration file
pub async fn apply_ticker_updates(
    pool: &SqlitePool,
    config_path: &str,
    changes_to_apply: Vec<StoredSymbolChange>,
    dry_run: bool,
) -> Result<()> {
    if changes_to_apply.is_empty() {
        println!("No changes to apply.");
        return Ok(());
    }

    // Validate all symbols before making any changes
    for change in &changes_to_apply {
        if !is_valid_ticker_symbol(&change.old_symbol) {
            anyhow::bail!(
                "Invalid old symbol '{}': must be alphanumeric with dots/hyphens, max 20 chars",
                change.old_symbol
            );
        }
        if !is_valid_ticker_symbol(&change.new_symbol) {
            anyhow::bail!(
                "Invalid new symbol '{}': must be alphanumeric with dots/hyphens, max 20 chars",
                change.new_symbol
            );
        }
    }

    // Read current config
    let config_content = fs::read_to_string(config_path).context("Failed to read config.toml")?;

    if !dry_run {
        // Create backup
        let backup_path = format!(
            "{}.backup.{}",
            config_path,
            Utc::now().format("%Y%m%d_%H%M%S")
        );
        fs::copy(config_path, &backup_path).context("Failed to create config backup")?;
        println!("✅ Created backup at: {}", backup_path);
    }

    let mut updated_content = config_content.clone();

    for change in &changes_to_apply {
        println!(
            "Applying change: {} -> {}",
            change.old_symbol, change.new_symbol
        );

        // Replace the ticker in the config content
        // Handle both quoted and potential comment scenarios
        let old_pattern = format!("\"{}\"", change.old_symbol);
        let new_replacement = format!(
            "\"{}\" # Changed from {} on {}",
            change.new_symbol,
            change.old_symbol,
            change
                .change_date
                .as_ref()
                .unwrap_or(&Utc::now().format("%Y-%m-%d").to_string())
        );

        if updated_content.contains(&old_pattern) {
            updated_content = updated_content.replace(&old_pattern, &new_replacement);

            if !dry_run {
                // Mark as applied in database
                sqlx::query!(
                    "UPDATE symbol_changes SET applied = 1, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                    change.id
                )
                .execute(pool)
                .await?;
            }
        } else {
            println!(
                "⚠️  Warning: Could not find {} in config",
                change.old_symbol
            );
        }
    }

    if dry_run {
        println!("\n=== DRY RUN - Changes that would be made: ===");
        println!("{}", updated_content);
        println!("=== END DRY RUN ===");
    } else {
        // Write updated config
        fs::write(config_path, updated_content).context("Failed to write updated config")?;
        println!(
            "✅ Updated config.toml with {} changes",
            changes_to_apply.len()
        );
    }

    Ok(())
}

/// Generate a detailed report of symbol changes
pub fn print_symbol_change_report(report: &SymbolChangeReport) {
    println!("\n=== Symbol Change Report ===");
    println!("Total pending changes: {}", report.pending_changes.len());
    println!(
        "Applicable to our config: {}",
        report.applicable_changes.len()
    );
    println!("Not applicable: {}", report.non_applicable_changes.len());
    println!("Conflicts: {}", report.conflicts.len());

    if !report.applicable_changes.is_empty() {
        println!("\n📝 Applicable Changes:");
        for change in &report.applicable_changes {
            println!(
                "  {} -> {} ({})",
                change.old_symbol,
                change.new_symbol,
                change
                    .company_name
                    .as_ref()
                    .unwrap_or(&"Unknown".to_string())
            );
        }
    }

    if !report.conflicts.is_empty() {
        println!("\n⚠️  Conflicts:");
        for conflict in &report.conflicts {
            println!("  {}", conflict);
        }
    }

    if !report.non_applicable_changes.is_empty() && report.non_applicable_changes.len() <= 10 {
        println!("\n📋 Non-applicable changes (not in our config):");
        for change in &report.non_applicable_changes {
            println!(
                "  {} -> {} ({})",
                change.old_symbol,
                change.new_symbol,
                change
                    .company_name
                    .as_ref()
                    .unwrap_or(&"Unknown".to_string())
            );
        }
    } else if !report.non_applicable_changes.is_empty() {
        println!(
            "\n📋 {} non-applicable changes (not in our config)",
            report.non_applicable_changes.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for is_valid_ticker_symbol function
    #[test]
    fn test_valid_us_ticker() {
        assert!(is_valid_ticker_symbol("AAPL"));
        assert!(is_valid_ticker_symbol("MSFT"));
        assert!(is_valid_ticker_symbol("NKE"));
    }

    #[test]
    fn test_valid_ticker_with_dot() {
        assert!(is_valid_ticker_symbol("BRK.A"));
        assert!(is_valid_ticker_symbol("BRK.B"));
        assert!(is_valid_ticker_symbol("MC.PA"));
    }

    #[test]
    fn test_valid_ticker_with_hyphen() {
        assert!(is_valid_ticker_symbol("HM-B.ST"));
        assert!(is_valid_ticker_symbol("HM-A"));
    }

    #[test]
    fn test_valid_ticker_with_numbers() {
        assert!(is_valid_ticker_symbol("9983.T"));
        assert!(is_valid_ticker_symbol("7974.T"));
        assert!(is_valid_ticker_symbol("3382.T"));
    }

    #[test]
    fn test_invalid_empty_ticker() {
        assert!(!is_valid_ticker_symbol(""));
    }

    #[test]
    fn test_invalid_ticker_too_long() {
        assert!(!is_valid_ticker_symbol("ABCDEFGHIJKLMNOPQRSTUVWXYZ")); // > 20 chars
    }

    #[test]
    fn test_invalid_ticker_with_space() {
        assert!(!is_valid_ticker_symbol("AA PL"));
    }

    #[test]
    fn test_invalid_ticker_with_special_chars() {
        assert!(!is_valid_ticker_symbol("AAPL!"));
        assert!(!is_valid_ticker_symbol("AAPL@"));
        assert!(!is_valid_ticker_symbol("AAPL#"));
        assert!(!is_valid_ticker_symbol("AAPL$"));
        assert!(!is_valid_ticker_symbol("AAPL%"));
    }

    #[test]
    fn test_invalid_ticker_with_unicode() {
        assert!(!is_valid_ticker_symbol("AAPL日本"));
        assert!(!is_valid_ticker_symbol("日本語"));
    }

    #[test]
    fn test_valid_ticker_max_length() {
        // Exactly 20 characters should be valid
        assert!(is_valid_ticker_symbol("ABCDEFGHIJ1234567890"));
    }

    #[test]
    fn test_valid_ticker_boundary() {
        // 21 characters should be invalid
        assert!(!is_valid_ticker_symbol("ABCDEFGHIJ12345678901"));
    }

    // Tests for StoredSymbolChange struct
    #[test]
    fn test_stored_symbol_change_creation() {
        let change = StoredSymbolChange {
            id: Some(1),
            old_symbol: "FB".to_string(),
            new_symbol: "META".to_string(),
            change_date: Some("2021-10-28".to_string()),
            company_name: Some("Meta Platforms Inc.".to_string()),
            reason: Some("Company rebranding".to_string()),
            applied: 0,
        };

        assert_eq!(change.old_symbol, "FB");
        assert_eq!(change.new_symbol, "META");
        assert_eq!(change.applied, 0);
    }

    #[test]
    fn test_stored_symbol_change_clone() {
        let change = StoredSymbolChange {
            id: Some(1),
            old_symbol: "OLD".to_string(),
            new_symbol: "NEW".to_string(),
            change_date: None,
            company_name: None,
            reason: None,
            applied: 0,
        };

        let cloned = change.clone();
        assert_eq!(change.old_symbol, cloned.old_symbol);
        assert_eq!(change.new_symbol, cloned.new_symbol);
    }

    // Tests for SymbolChangeReport struct
    #[test]
    fn test_symbol_change_report_empty() {
        let report = SymbolChangeReport {
            pending_changes: vec![],
            applicable_changes: vec![],
            non_applicable_changes: vec![],
            conflicts: vec![],
        };

        assert!(report.pending_changes.is_empty());
        assert!(report.applicable_changes.is_empty());
        assert!(report.non_applicable_changes.is_empty());
        assert!(report.conflicts.is_empty());
    }

    #[test]
    fn test_symbol_change_report_with_changes() {
        let change = StoredSymbolChange {
            id: Some(1),
            old_symbol: "OLD".to_string(),
            new_symbol: "NEW".to_string(),
            change_date: None,
            company_name: None,
            reason: None,
            applied: 0,
        };

        let report = SymbolChangeReport {
            pending_changes: vec![change.clone()],
            applicable_changes: vec![change.clone()],
            non_applicable_changes: vec![],
            conflicts: vec![],
        };

        assert_eq!(report.pending_changes.len(), 1);
        assert_eq!(report.applicable_changes.len(), 1);
    }

    #[test]
    fn test_symbol_change_report_with_conflicts() {
        let report = SymbolChangeReport {
            pending_changes: vec![],
            applicable_changes: vec![],
            non_applicable_changes: vec![],
            conflicts: vec![
                "Both OLD and NEW exist in config".to_string(),
                "Another conflict".to_string(),
            ],
        };

        assert_eq!(report.conflicts.len(), 2);
    }

    // Tests for config replacement pattern
    #[test]
    fn test_old_pattern_generation() {
        let old_symbol = "AAPL";
        let old_pattern = format!("\"{}\"", old_symbol);
        assert_eq!(old_pattern, "\"AAPL\"");
    }

    #[test]
    fn test_new_replacement_generation() {
        let new_symbol = "NEW";
        let old_symbol = "OLD";
        let change_date = "2025-01-15";

        let new_replacement = format!(
            "\"{}\" # Changed from {} on {}",
            new_symbol, old_symbol, change_date
        );

        assert_eq!(new_replacement, "\"NEW\" # Changed from OLD on 2025-01-15");
        assert!(new_replacement.contains("Changed from"));
    }

    // Tests for HashSet operations (used in check_ticker_updates)
    #[test]
    fn test_ticker_hashset_operations() {
        let mut tickers = HashSet::new();
        tickers.insert("AAPL".to_string());
        tickers.insert("MSFT".to_string());
        tickers.insert("NKE".to_string());

        assert!(tickers.contains(&"AAPL".to_string()));
        assert!(!tickers.contains(&"GOOGL".to_string()));
        assert_eq!(tickers.len(), 3);
    }

    #[test]
    fn test_conflict_detection_logic() {
        let mut current_tickers = HashSet::new();
        current_tickers.insert("OLD".to_string());
        current_tickers.insert("NEW".to_string()); // Both exist - conflict!

        let old_symbol = "OLD".to_string();
        let new_symbol = "NEW".to_string();

        // Conflict: both old and new symbols exist
        let is_conflict =
            current_tickers.contains(&old_symbol) && current_tickers.contains(&new_symbol);
        assert!(is_conflict);
    }

    #[test]
    fn test_applicable_change_logic() {
        let mut current_tickers = HashSet::new();
        current_tickers.insert("OLD".to_string());
        // NEW does not exist

        let old_symbol = "OLD".to_string();
        let new_symbol = "NEW".to_string();

        // Applicable: old exists, new doesn't
        let is_applicable =
            current_tickers.contains(&old_symbol) && !current_tickers.contains(&new_symbol);
        assert!(is_applicable);
    }

    #[test]
    fn test_non_applicable_change_logic() {
        let mut current_tickers = HashSet::new();
        current_tickers.insert("AAPL".to_string());
        // Neither OLD nor NEW exist in our config

        let old_symbol = "OLD".to_string();

        // Non-applicable: old symbol not in our config
        let is_non_applicable = !current_tickers.contains(&old_symbol);
        assert!(is_non_applicable);
    }

    // Test serialization of StoredSymbolChange
    #[test]
    fn test_stored_symbol_change_serialization() {
        let change = StoredSymbolChange {
            id: Some(1),
            old_symbol: "FB".to_string(),
            new_symbol: "META".to_string(),
            change_date: Some("2021-10-28".to_string()),
            company_name: Some("Meta Platforms".to_string()),
            reason: None,
            applied: 0,
        };

        let json = serde_json::to_string(&change).expect("Should serialize");
        assert!(json.contains("FB"));
        assert!(json.contains("META"));
    }

    #[test]
    fn test_stored_symbol_change_deserialization() {
        let json = r#"{
            "id": 1,
            "old_symbol": "FB",
            "new_symbol": "META",
            "change_date": "2021-10-28",
            "company_name": "Meta Platforms",
            "reason": null,
            "applied": 0
        }"#;

        let change: StoredSymbolChange = serde_json::from_str(json).expect("Should deserialize");

        assert_eq!(change.old_symbol, "FB");
        assert_eq!(change.new_symbol, "META");
        assert_eq!(change.applied, 0);
    }
}

// Required for serialization tests
#[cfg(test)]
use serde_json;
