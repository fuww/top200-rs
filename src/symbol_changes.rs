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
