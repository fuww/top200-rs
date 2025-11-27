// SPDX-FileCopyrightText: 2025 Joost van der Laan
// SPDX-License-Identifier: AGPL-3.0-only

use crate::api::FMPClient;
use crate::currencies::insert_forex_rate;
use anyhow::Result;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use sqlx::sqlite::SqlitePool;

/// Update exchange rates in the database
pub async fn update_exchange_rates(fmp_client: &FMPClient, pool: &SqlitePool) -> Result<()> {
    // Fetch exchange rates
    println!("Fetching current exchange rates...");
    let exchange_rates = match fmp_client.get_exchange_rates().await {
        Ok(rates) => {
            println!("✅ Exchange rates fetched");
            rates
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to fetch exchange rates: {}", e));
        }
    };

    // Store rates in database (use UTC timestamp for consistency)
    let timestamp = Utc::now().timestamp();
    for rate in exchange_rates {
        if let (Some(name), Some(price)) = (rate.name, rate.price) {
            insert_forex_rate(pool, &name, price, price, timestamp).await?;
        }
    }

    println!("✅ Exchange rates updated in database");
    Ok(())
}

/// Currency pairs commonly needed for market cap conversions
const COMMON_FOREX_PAIRS: &[&str] = &[
    "EURUSD", "GBPUSD", "JPYUSD", "CHFUSD", "SEKUSD", "DKKUSD", "NOKUSD", "HKDUSD", "CNYUSD",
    "BRLUSD", "CADUSD", "ILSUSD", "ZARUSD", "INRUSD", "KRWUSD", "TRYUSD", "PLNUSD", "TWDUSD",
];

/// Fetch and store historical exchange rates for a date range
pub async fn fetch_historical_exchange_rates(
    fmp_client: &FMPClient,
    pool: &SqlitePool,
    from_date: &str,
    to_date: &str,
) -> Result<()> {
    println!(
        "Fetching historical exchange rates from {} to {}",
        from_date, to_date
    );

    // Get available forex pairs to validate
    println!("Fetching available forex pairs...");
    let available_pairs = match fmp_client.get_available_forex_pairs().await {
        Ok(pairs) => {
            println!("✅ Found {} available forex pairs", pairs.len());
            pairs
        }
        Err(e) => {
            eprintln!(
                "⚠️  Could not fetch available pairs, using common pairs: {}",
                e
            );
            COMMON_FOREX_PAIRS.iter().map(|s| s.to_string()).collect()
        }
    };

    // Filter to common pairs that are available
    let pairs_to_fetch: Vec<&str> = COMMON_FOREX_PAIRS
        .iter()
        .filter(|p| available_pairs.iter().any(|ap| ap == *p))
        .copied()
        .collect();

    if pairs_to_fetch.is_empty() {
        println!("Using all common forex pairs...");
    } else {
        println!("Fetching {} currency pairs...", pairs_to_fetch.len());
    }

    let pairs = if pairs_to_fetch.is_empty() {
        COMMON_FOREX_PAIRS.to_vec()
    } else {
        pairs_to_fetch
    };

    // Set up progress bar
    let progress = ProgressBar::new(pairs.len() as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>3}/{len:3} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    let mut total_rates = 0usize;
    let mut failed_pairs = Vec::new();

    for pair in &pairs {
        progress.set_message(format!("Fetching {}...", pair));

        match fmp_client
            .get_historical_exchange_rates(pair, from_date, to_date)
            .await
        {
            Ok(response) => {
                let symbol_with_slash = format_pair_with_slash(&response.symbol);

                for data in &response.historical {
                    // Parse date and convert to Unix timestamp
                    if let Ok(date) = NaiveDate::parse_from_str(&data.date, "%Y-%m-%d") {
                        let datetime =
                            NaiveDateTime::new(date, NaiveTime::from_hms_opt(0, 0, 0).unwrap());
                        let timestamp = datetime.and_utc().timestamp();

                        // Use close price as the rate (most commonly used)
                        insert_forex_rate(
                            pool,
                            &symbol_with_slash,
                            data.close,
                            data.close,
                            timestamp,
                        )
                        .await?;
                        total_rates += 1;
                    }
                }
            }
            Err(e) => {
                failed_pairs.push((pair.to_string(), e.to_string()));
            }
        }

        progress.inc(1);
    }

    progress.finish_with_message("Done");

    // Print summary
    println!("\n📊 Historical Exchange Rates Summary:");
    println!("   Date range: {} to {}", from_date, to_date);
    println!("   Pairs processed: {}", pairs.len() - failed_pairs.len());
    println!("   Total rates stored: {}", total_rates);

    if !failed_pairs.is_empty() {
        println!("\n⚠️  Failed to fetch {} pairs:", failed_pairs.len());
        for (pair, error) in &failed_pairs {
            println!("   {} - {}", pair, error);
        }
    }

    println!("\n✅ Historical exchange rates updated in database");
    Ok(())
}

/// Convert a pair like "EURUSD" to "EUR/USD"
fn format_pair_with_slash(pair: &str) -> String {
    if pair.len() == 6 && !pair.contains('/') {
        format!("{}/{}", &pair[0..3], &pair[3..6])
    } else {
        pair.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_pair_with_slash() {
        assert_eq!(format_pair_with_slash("EURUSD"), "EUR/USD");
        assert_eq!(format_pair_with_slash("GBPUSD"), "GBP/USD");
        assert_eq!(format_pair_with_slash("EUR/USD"), "EUR/USD");
        assert_eq!(format_pair_with_slash("JPYUSD"), "JPY/USD");
    }

    #[test]
    fn test_format_pair_with_slash_all_common_pairs() {
        // Test all common forex pairs can be formatted correctly
        for pair in COMMON_FOREX_PAIRS {
            let formatted = format_pair_with_slash(pair);
            assert!(
                formatted.contains('/'),
                "Pair {} should contain slash",
                pair
            );
            let parts: Vec<&str> = formatted.split('/').collect();
            assert_eq!(
                parts.len(),
                2,
                "Pair {} should have exactly two parts",
                pair
            );
            assert_eq!(
                parts[0].len(),
                3,
                "First currency in {} should be 3 chars",
                pair
            );
            assert_eq!(
                parts[1].len(),
                3,
                "Second currency in {} should be 3 chars",
                pair
            );
        }
    }

    #[test]
    fn test_format_pair_with_slash_edge_cases() {
        // Already has slash - should return as-is
        assert_eq!(format_pair_with_slash("EUR/USD"), "EUR/USD");
        assert_eq!(format_pair_with_slash("GBP/JPY"), "GBP/JPY");

        // Short strings - should return as-is
        assert_eq!(format_pair_with_slash("EUR"), "EUR");
        assert_eq!(format_pair_with_slash(""), "");
        assert_eq!(format_pair_with_slash("AB"), "AB");

        // Long strings without slash - should return as-is
        assert_eq!(format_pair_with_slash("EURUSDJPY"), "EURUSDJPY");

        // Exactly 6 chars with slash already - should return as-is
        assert_eq!(format_pair_with_slash("EU/USD"), "EU/USD");
    }

    #[test]
    fn test_common_forex_pairs_not_empty() {
        assert!(!COMMON_FOREX_PAIRS.is_empty());
        assert!(
            COMMON_FOREX_PAIRS.len() >= 10,
            "Should have at least 10 common pairs"
        );
    }

    #[test]
    fn test_common_forex_pairs_all_end_with_usd() {
        // All pairs should end with USD (base currency)
        for pair in COMMON_FOREX_PAIRS {
            assert!(pair.ends_with("USD"), "Pair {} should end with USD", pair);
        }
    }

    #[test]
    fn test_common_forex_pairs_format() {
        // All pairs should be 6 characters (XXXUSD format)
        for pair in COMMON_FOREX_PAIRS {
            assert_eq!(pair.len(), 6, "Pair {} should be 6 characters", pair);
        }
    }

    #[test]
    fn test_common_forex_pairs_unique() {
        // All pairs should be unique
        let mut seen = std::collections::HashSet::new();
        for pair in COMMON_FOREX_PAIRS {
            assert!(seen.insert(pair), "Pair {} appears more than once", pair);
        }
    }

    #[test]
    fn test_common_forex_pairs_includes_major_currencies() {
        // Major currencies should be included
        let major_currencies = ["EUR", "GBP", "JPY", "CHF", "CAD"];
        for currency in major_currencies {
            let pair = format!("{}USD", currency);
            assert!(
                COMMON_FOREX_PAIRS.contains(&pair.as_str()),
                "Major currency pair {} should be in COMMON_FOREX_PAIRS",
                pair
            );
        }
    }
}
