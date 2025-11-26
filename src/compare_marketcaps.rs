// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime};
use csv::{Reader, Writer};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use sqlx::sqlite::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write as IoWrite;
use std::path::Path;

use crate::currencies::{convert_currency, get_rate_map_from_db_for_date};

#[derive(Debug, Deserialize)]
struct MarketCapRecord {
    #[serde(rename = "Rank")]
    rank: Option<usize>,
    #[serde(rename = "Ticker")]
    ticker: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Market Cap (Original)")]
    market_cap_original: Option<f64>,
    #[serde(rename = "Original Currency")]
    original_currency: Option<String>,
    #[serde(rename = "Market Cap (EUR)")]
    market_cap_eur: Option<f64>,
    #[serde(rename = "Market Cap (USD)")]
    market_cap_usd: Option<f64>,
}

#[derive(Debug)]
struct MarketCapComparison {
    ticker: String,
    name: String,
    market_cap_from: Option<f64>,
    market_cap_to: Option<f64>,
    absolute_change: Option<f64>,
    percentage_change: Option<f64>,
    rank_from: Option<usize>,
    rank_to: Option<usize>,
    rank_change: Option<i32>,
    market_share_from: Option<f64>,
    market_share_to: Option<f64>,
}

/// Find the most recent CSV file for a given date
fn find_csv_for_date(date: &str) -> Result<String> {
    let output_dir = Path::new("output");
    let pattern = format!("marketcaps_{}_", date);

    let mut matching_files = Vec::new();
    for entry in std::fs::read_dir(output_dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        if file_name_str.starts_with(&pattern) && file_name_str.ends_with(".csv") {
            matching_files.push(file_name_str.to_string());
        }
    }

    if matching_files.is_empty() {
        anyhow::bail!(
            "No CSV file found for date {}. Please run 'fetch-specific-date-market-caps {}' first.",
            date,
            date
        );
    }

    // Sort to get the most recent file (by filename timestamp)
    matching_files.sort();
    let selected_file = matching_files.last().unwrap();

    Ok(format!("output/{}", selected_file))
}

/// Read market cap data from CSV file
fn read_market_cap_csv(file_path: &str) -> Result<Vec<MarketCapRecord>> {
    let file =
        File::open(file_path).with_context(|| format!("Failed to open CSV file: {}", file_path))?;

    let mut reader = Reader::from_reader(file);
    let mut records = Vec::new();

    for result in reader.deserialize() {
        let record: MarketCapRecord = result?;
        records.push(record);
    }

    Ok(records)
}

/// Calculate market share for each company
fn calculate_market_shares(records: &[MarketCapRecord]) -> HashMap<String, f64> {
    let total_market_cap: f64 = records.iter().filter_map(|r| r.market_cap_usd).sum();

    let mut shares = HashMap::new();

    if total_market_cap > 0.0 {
        for record in records {
            if let Some(market_cap) = record.market_cap_usd {
                let share = (market_cap / total_market_cap) * 100.0;
                shares.insert(record.ticker.clone(), share);
            }
        }
    }

    shares
}

/// Compare market caps between two dates
pub async fn compare_market_caps(pool: &SqlitePool, from_date: &str, to_date: &str) -> Result<()> {
    println!("Comparing market caps from {} to {}", from_date, to_date);

    // Find CSV files for both dates
    let from_file = find_csv_for_date(from_date)?;
    let to_file = find_csv_for_date(to_date)?;

    println!("Using files:");
    println!("  From: {}", from_file);
    println!("  To:   {}", to_file);

    // Get exchange rates for the "to" date to normalize all values
    let to_date_parsed = NaiveDate::parse_from_str(to_date, "%Y-%m-%d")?;
    let to_date_timestamp = NaiveDateTime::new(to_date_parsed, NaiveTime::default())
        .and_utc()
        .timestamp();

    println!(
        "\n🔄 Normalizing currencies using {} exchange rates...",
        to_date
    );
    let mut normalization_rates =
        get_rate_map_from_db_for_date(pool, Some(to_date_timestamp)).await?;

    // If no rates found for the specific date, fall back to latest available rates
    if normalization_rates.is_empty() {
        eprintln!(
            "⚠️  No exchange rates found for {} - falling back to latest rates",
            to_date
        );
        normalization_rates = get_rate_map_from_db_for_date(pool, None).await?;
    }

    if normalization_rates.is_empty() {
        eprintln!("⚠️  WARNING: No exchange rates found at all!");
        eprintln!("    Comparisons will include both market cap AND currency changes.");
        eprintln!("    Run 'export-rates' to fetch exchange rates.");
    } else {
        println!("✅ Using exchange rates for currency normalization");
        println!("   This eliminates FX noise and shows pure market cap changes");
    }

    // Read data from both files
    let progress = ProgressBar::new(4);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    progress.set_message("Reading from date CSV...");
    let from_records = read_market_cap_csv(&from_file)?;
    progress.inc(1);

    progress.set_message("Reading to date CSV...");
    let to_records = read_market_cap_csv(&to_file)?;
    progress.inc(1);

    // Create lookup maps
    let mut from_map: HashMap<String, MarketCapRecord> = HashMap::new();
    let mut to_map: HashMap<String, MarketCapRecord> = HashMap::new();

    for record in from_records.iter() {
        from_map.insert(
            record.ticker.clone(),
            MarketCapRecord {
                rank: record.rank,
                ticker: record.ticker.clone(),
                name: record.name.clone(),
                market_cap_original: record.market_cap_original,
                original_currency: record.original_currency.clone(),
                market_cap_eur: record.market_cap_eur,
                market_cap_usd: record.market_cap_usd,
            },
        );
    }

    for record in to_records.iter() {
        to_map.insert(
            record.ticker.clone(),
            MarketCapRecord {
                rank: record.rank,
                ticker: record.ticker.clone(),
                name: record.name.clone(),
                market_cap_original: record.market_cap_original,
                original_currency: record.original_currency.clone(),
                market_cap_eur: record.market_cap_eur,
                market_cap_usd: record.market_cap_usd,
            },
        );
    }

    // Calculate market shares
    progress.set_message("Calculating market shares...");
    let from_shares = calculate_market_shares(&from_records);
    let to_shares = calculate_market_shares(&to_records);
    progress.inc(1);

    // Build comparison data
    progress.set_message("Analyzing changes...");
    let mut comparisons = Vec::new();
    let mut all_tickers = std::collections::HashSet::new();

    for ticker in from_map.keys() {
        all_tickers.insert(ticker.clone());
    }
    for ticker in to_map.keys() {
        all_tickers.insert(ticker.clone());
    }

    for ticker in all_tickers {
        let from_record = from_map.get(&ticker);
        let to_record = to_map.get(&ticker);

        let name = from_record
            .map(|r| r.name.clone())
            .or_else(|| to_record.map(|r| r.name.clone()))
            .unwrap_or_else(|| ticker.clone());

        // NORMALIZE: Convert both dates using the SAME exchange rate (to_date's rate)
        // This eliminates FX noise and shows pure market cap changes
        let market_cap_from = from_record.and_then(|r| {
            r.market_cap_original.map(|orig| {
                let currency = r.original_currency.as_deref().unwrap_or("USD");
                if normalization_rates.is_empty() {
                    // Fallback: use unconverted USD value if no rates available
                    r.market_cap_usd.unwrap_or(orig)
                } else {
                    // Normalize: convert original value using to_date's exchange rate
                    convert_currency(orig, currency, "USD", &normalization_rates)
                }
            })
        });

        let market_cap_to = to_record.and_then(|r| {
            r.market_cap_original.map(|orig| {
                let currency = r.original_currency.as_deref().unwrap_or("USD");
                if normalization_rates.is_empty() {
                    // Fallback: use unconverted USD value if no rates available
                    r.market_cap_usd.unwrap_or(orig)
                } else {
                    // Normalize: convert original value using to_date's exchange rate
                    convert_currency(orig, currency, "USD", &normalization_rates)
                }
            })
        });

        let (absolute_change, percentage_change) = match (market_cap_from, market_cap_to) {
            (Some(from_val), Some(to_val)) => {
                let abs_change = to_val - from_val;
                let pct_change = if from_val != 0.0 {
                    (abs_change / from_val) * 100.0
                } else {
                    0.0
                };
                (Some(abs_change), Some(pct_change))
            }
            _ => (None, None),
        };

        let rank_from = from_record.and_then(|r| r.rank);
        let rank_to = to_record.and_then(|r| r.rank);

        let rank_change = match (rank_from, rank_to) {
            (Some(from_rank), Some(to_rank)) => Some(from_rank as i32 - to_rank as i32),
            _ => None,
        };

        comparisons.push(MarketCapComparison {
            ticker: ticker.clone(),
            name,
            market_cap_from,
            market_cap_to,
            absolute_change,
            percentage_change,
            rank_from,
            rank_to,
            rank_change,
            market_share_from: from_shares.get(&ticker).copied(),
            market_share_to: to_shares.get(&ticker).copied(),
        });
    }

    // Sort by percentage change (descending)
    comparisons.sort_by(|a, b| {
        let a_pct = a.percentage_change.unwrap_or(f64::NEG_INFINITY);
        let b_pct = b.percentage_change.unwrap_or(f64::NEG_INFINITY);
        b_pct.partial_cmp(&a_pct).unwrap()
    });

    // Collect unique currencies used in the data
    let mut currencies_used: HashSet<String> = HashSet::new();
    for record in from_records.iter().chain(to_records.iter()) {
        if let Some(currency) = &record.original_currency {
            if currency != "USD" {
                currencies_used.insert(currency.clone());
            }
        }
    }

    progress.inc(1);
    progress.finish_with_message("Analysis complete");

    // Export main comparison CSV
    export_comparison_csv(&comparisons, from_date, to_date)?;

    // Export summary report with exchange rates information
    export_summary_report(
        &comparisons,
        from_date,
        to_date,
        &normalization_rates,
        &currencies_used,
    )?;

    Ok(())
}

/// Export comparison data to CSV
fn export_comparison_csv(
    comparisons: &[MarketCapComparison],
    from_date: &str,
    to_date: &str,
) -> Result<()> {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!(
        "output/comparison_{}_to_{}_{}.csv",
        from_date, to_date, timestamp
    );

    let file = File::create(&filename)?;
    let mut writer = Writer::from_writer(file);

    // Write headers
    writer.write_record(&[
        "Ticker",
        "Name",
        "Market Cap From (USD)",
        "Market Cap To (USD)",
        "Absolute Change (USD)",
        "Percentage Change (%)",
        "Rank From",
        "Rank To",
        "Rank Change",
        "Market Share From (%)",
        "Market Share To (%)",
    ])?;

    // Write data
    for comp in comparisons {
        writer.write_record(&[
            comp.ticker.clone(),
            comp.name.clone(),
            comp.market_cap_from
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "NA".to_string()),
            comp.market_cap_to
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "NA".to_string()),
            comp.absolute_change
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "NA".to_string()),
            comp.percentage_change
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "NA".to_string()),
            comp.rank_from
                .map(|v| v.to_string())
                .unwrap_or_else(|| "NA".to_string()),
            comp.rank_to
                .map(|v| v.to_string())
                .unwrap_or_else(|| "NA".to_string()),
            comp.rank_change
                .map(|v| {
                    if v > 0 {
                        format!("+{}", v)
                    } else {
                        v.to_string()
                    }
                })
                .unwrap_or_else(|| "NA".to_string()),
            comp.market_share_from
                .map(|v| format!("{:.4}", v))
                .unwrap_or_else(|| "NA".to_string()),
            comp.market_share_to
                .map(|v| format!("{:.4}", v))
                .unwrap_or_else(|| "NA".to_string()),
        ])?;
    }

    writer.flush()?;
    println!("✅ Comparison data exported to {}", filename);

    Ok(())
}

/// Export summary report in Markdown format
fn export_summary_report(
    comparisons: &[MarketCapComparison],
    from_date: &str,
    to_date: &str,
    rate_map: &HashMap<String, f64>,
    currencies_used: &HashSet<String>,
) -> Result<()> {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!(
        "output/comparison_{}_to_{}_summary_{}.md",
        from_date, to_date, timestamp
    );

    let mut file = File::create(&filename)?;

    writeln!(
        file,
        "# Market Cap Comparison: {} to {}",
        from_date, to_date
    )?;
    writeln!(file)?;

    // Calculate overview statistics
    let total_from: f64 = comparisons.iter().filter_map(|c| c.market_cap_from).sum();
    let total_to: f64 = comparisons.iter().filter_map(|c| c.market_cap_to).sum();
    let total_change = total_to - total_from;
    let total_pct_change = if total_from > 0.0 {
        (total_change / total_from) * 100.0
    } else {
        0.0
    };

    writeln!(file, "## Overview Statistics")?;
    writeln!(
        file,
        "- Total Market Cap on {}: ${:.2}B",
        from_date,
        total_from / 1_000_000_000.0
    )?;
    writeln!(
        file,
        "- Total Market Cap on {}: ${:.2}B",
        to_date,
        total_to / 1_000_000_000.0
    )?;
    writeln!(
        file,
        "- Total Change: ${:.2}B ({:.2}%)",
        total_change / 1_000_000_000.0,
        total_pct_change
    )?;
    writeln!(file)?;

    // Filter out comparisons with valid percentage changes
    let mut valid_comparisons: Vec<_> = comparisons
        .iter()
        .filter(|c| c.percentage_change.is_some())
        .collect();

    // Top 10 gainers
    writeln!(file, "## Top 10 Gainers (by percentage)")?;
    valid_comparisons.sort_by(|a, b| {
        b.percentage_change
            .unwrap()
            .partial_cmp(&a.percentage_change.unwrap())
            .unwrap()
    });

    for (i, comp) in valid_comparisons.iter().take(10).enumerate() {
        let pct = comp.percentage_change.unwrap();
        let abs_change = comp.absolute_change.unwrap_or(0.0);

        writeln!(
            file,
            "{}. **{}** ([{}](https://finance.yahoo.com/quote/{}/)): +{:.2}% (${:.2}M increase)",
            i + 1,
            comp.name,
            comp.ticker,
            comp.ticker,
            pct,
            abs_change / 1_000_000.0
        )?;
    }
    writeln!(file)?;

    // Top 10 losers
    writeln!(file, "## Top 10 Losers (by percentage)")?;
    valid_comparisons.sort_by(|a, b| {
        a.percentage_change
            .unwrap()
            .partial_cmp(&b.percentage_change.unwrap())
            .unwrap()
    });

    for (i, comp) in valid_comparisons.iter().take(10).enumerate() {
        writeln!(
            file,
            "{}. **{}** ([{}](https://finance.yahoo.com/quote/{}/)): {:.2}% (${:.2}M decrease)",
            i + 1,
            comp.name,
            comp.ticker,
            comp.ticker,
            comp.percentage_change.unwrap(),
            comp.absolute_change.unwrap_or(0.0) / 1_000_000.0
        )?;
    }
    writeln!(file)?;

    // Top 10 by absolute gain
    writeln!(file, "## Top 10 by Absolute Gain")?;
    valid_comparisons.sort_by(|a, b| {
        b.absolute_change
            .unwrap_or(0.0)
            .partial_cmp(&a.absolute_change.unwrap_or(0.0))
            .unwrap()
    });

    for (i, comp) in valid_comparisons.iter().take(10).enumerate() {
        writeln!(
            file,
            "{}. **{}** ([{}](https://finance.yahoo.com/quote/{}/)): ${:.2}B gain ({:.2}%)",
            i + 1,
            comp.name,
            comp.ticker,
            comp.ticker,
            comp.absolute_change.unwrap_or(0.0) / 1_000_000_000.0,
            comp.percentage_change.unwrap_or(0.0)
        )?;
    }
    writeln!(file)?;

    // Top 10 by absolute loss
    writeln!(file, "## Top 10 by Absolute Loss")?;
    valid_comparisons.sort_by(|a, b| {
        a.absolute_change
            .unwrap_or(0.0)
            .partial_cmp(&b.absolute_change.unwrap_or(0.0))
            .unwrap()
    });

    for (i, comp) in valid_comparisons.iter().take(10).enumerate() {
        if comp.absolute_change.unwrap_or(0.0) < 0.0 {
            writeln!(
                file,
                "{}. **{}** ([{}](https://finance.yahoo.com/quote/{}/)): ${:.2}B loss ({:.2}%)",
                i + 1,
                comp.name,
                comp.ticker,
                comp.ticker,
                comp.absolute_change.unwrap_or(0.0).abs() / 1_000_000_000.0,
                comp.percentage_change.unwrap_or(0.0)
            )?;
        }
    }
    writeln!(file)?;

    // Biggest rank improvements
    writeln!(file, "## Biggest Rank Improvements")?;
    let mut rank_comparisons: Vec<_> = comparisons
        .iter()
        .filter(|c| c.rank_change.is_some())
        .collect();
    rank_comparisons.sort_by(|a, b| b.rank_change.unwrap().cmp(&a.rank_change.unwrap()));

    for (i, comp) in rank_comparisons.iter().take(10).enumerate() {
        if comp.rank_change.unwrap() > 0 {
            writeln!(
                file,
                "{}. **{}** ([{}](https://finance.yahoo.com/quote/{}/)): +{} positions (#{} → #{})",
                i + 1,
                comp.name,
                comp.ticker,
                comp.ticker,
                comp.rank_change.unwrap(),
                comp.rank_from.unwrap_or(0),
                comp.rank_to.unwrap_or(0)
            )?;
        }
    }
    writeln!(file)?;

    // Biggest rank declines
    writeln!(file, "## Biggest Rank Declines")?;
    rank_comparisons.sort_by(|a, b| a.rank_change.unwrap().cmp(&b.rank_change.unwrap()));

    for (i, comp) in rank_comparisons.iter().take(10).enumerate() {
        if comp.rank_change.unwrap() < 0 {
            writeln!(
                file,
                "{}. **{}** ([{}](https://finance.yahoo.com/quote/{}/)): {} positions (#{} → #{})",
                i + 1,
                comp.name,
                comp.ticker,
                comp.ticker,
                comp.rank_change.unwrap(),
                comp.rank_from.unwrap_or(0),
                comp.rank_to.unwrap_or(0)
            )?;
        }
    }
    writeln!(file)?;

    // Market concentration analysis
    writeln!(file, "## Market Concentration Analysis")?;

    let companies_with_increase = comparisons
        .iter()
        .filter(|c| c.percentage_change.map(|v| v > 0.0).unwrap_or(false))
        .count();

    let companies_with_decrease = comparisons
        .iter()
        .filter(|c| c.percentage_change.map(|v| v < 0.0).unwrap_or(false))
        .count();

    let new_companies = comparisons
        .iter()
        .filter(|c| c.market_cap_from.is_none() && c.market_cap_to.is_some())
        .count();

    let delisted_companies = comparisons
        .iter()
        .filter(|c| c.market_cap_from.is_some() && c.market_cap_to.is_none())
        .count();

    writeln!(
        file,
        "- Companies with increased market cap: {}",
        companies_with_increase
    )?;
    writeln!(
        file,
        "- Companies with decreased market cap: {}",
        companies_with_decrease
    )?;
    writeln!(file, "- New companies in list: {}", new_companies)?;
    writeln!(
        file,
        "- Companies no longer in list: {}",
        delisted_companies
    )?;
    writeln!(file)?;

    // Exchange rates section
    writeln!(file, "## Exchange Rates Used for Normalization")?;
    writeln!(file)?;
    writeln!(
        file,
        "All values in this report are normalized to USD using exchange rates from **{}**.",
        to_date
    )?;
    writeln!(
        file,
        "This eliminates currency fluctuations and shows pure market cap changes."
    )?;
    writeln!(file)?;

    if currencies_used.is_empty() {
        writeln!(
            file,
            "_All companies are USD-denominated, no currency conversion needed._"
        )?;
    } else {
        writeln!(file, "| Currency | Rate to USD |")?;
        writeln!(file, "|----------|-------------|")?;

        // Sort currencies for consistent output
        let mut sorted_currencies: Vec<_> = currencies_used.iter().collect();
        sorted_currencies.sort();

        for currency in sorted_currencies {
            let rate_key = format!("{}/USD", currency);
            if let Some(&rate) = rate_map.get(&rate_key) {
                writeln!(file, "| {} | {:.6} |", currency, rate)?;
            } else {
                // Try reverse lookup
                let reverse_key = format!("USD/{}", currency);
                if let Some(&rate) = rate_map.get(&reverse_key) {
                    writeln!(file, "| {} | {:.6} |", currency, 1.0 / rate)?;
                } else {
                    writeln!(file, "| {} | _not available_ |", currency)?;
                }
            }
        }
    }

    writeln!(file)?;
    writeln!(file, "---")?;
    writeln!(
        file,
        "*Generated on {}*",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    )?;

    println!("✅ Summary report exported to {}", filename);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yahoo_finance_link_format() {
        // Test that ticker is properly formatted in markdown link
        let ticker = "AAPL";
        let expected = format!("[{}](https://finance.yahoo.com/quote/{}/)", ticker, ticker);
        assert_eq!(expected, "[AAPL](https://finance.yahoo.com/quote/AAPL/)");
    }

    #[test]
    fn test_yahoo_finance_link_with_special_characters() {
        // Test tickers with special characters (e.g., BRK.B, BF.A)
        let ticker = "BRK.B";
        let link = format!("[{}](https://finance.yahoo.com/quote/{}/)", ticker, ticker);
        assert_eq!(link, "[BRK.B](https://finance.yahoo.com/quote/BRK.B/)");
    }

    #[test]
    fn test_market_cap_comparison_calculation() {
        let from_val = 1000000000.0;
        let to_val = 1100000000.0;
        let abs_change = to_val - from_val;
        let pct_change = if from_val != 0.0 {
            (abs_change / from_val) * 100.0
        } else {
            0.0
        };

        assert_eq!(abs_change, 100000000.0);
        assert_eq!(pct_change, 10.0);
    }

    #[test]
    fn test_market_cap_large_realistic_values() {
        // Apple-like market cap: ~$3 trillion
        let from_val = 3_000_000_000_000.0;
        let to_val = 3_300_000_000_000.0; // 10% increase
        let abs_change = to_val - from_val;
        let pct_change = if from_val != 0.0 {
            (abs_change / from_val) * 100.0
        } else {
            0.0
        };

        assert_eq!(abs_change, 300_000_000_000.0); // $300B increase
        assert_eq!(pct_change, 10.0); // 10% increase
    }

    #[test]
    fn test_market_cap_small_percentage() {
        // Test a small 2% change
        let from_val = 1_000_000_000_000.0; // $1T
        let to_val = 1_020_000_000_000.0; // $1.02T
        let abs_change = to_val - from_val;
        let pct_change = if from_val != 0.0 {
            (abs_change / from_val) * 100.0
        } else {
            0.0
        };

        assert_eq!(abs_change, 20_000_000_000.0); // $20B increase
        assert_eq!(pct_change, 2.0); // 2% increase
    }

    #[test]
    fn test_market_cap_comparison_negative_change() {
        let from_val = 1000000000.0;
        let to_val = 900000000.0;
        let abs_change = to_val - from_val;
        let pct_change = if from_val != 0.0 {
            (abs_change / from_val) * 100.0
        } else {
            0.0
        };

        assert_eq!(abs_change, -100000000.0);
        assert_eq!(pct_change, -10.0);
    }

    #[test]
    fn test_rank_change_calculation() {
        // Rank improved from 10 to 5 (positive rank change)
        let from_rank = 10;
        let to_rank = 5;
        let rank_change = from_rank as i32 - to_rank as i32;

        assert_eq!(rank_change, 5); // Positive means improvement
    }

    #[test]
    fn test_rank_change_decline() {
        // Rank declined from 5 to 10 (negative rank change)
        let from_rank = 5;
        let to_rank = 10;
        let rank_change = from_rank as i32 - to_rank as i32;

        assert_eq!(rank_change, -5); // Negative means decline
    }

    #[test]
    fn test_market_share_calculation() {
        let records = vec![
            MarketCapRecord {
                rank: Some(1),
                ticker: "AAPL".to_string(),
                name: "Apple".to_string(),
                market_cap_original: Some(2000000000000.0),
                original_currency: Some("USD".to_string()),
                market_cap_eur: Some(1800000000000.0),
                market_cap_usd: Some(2000000000000.0),
            },
            MarketCapRecord {
                rank: Some(2),
                ticker: "MSFT".to_string(),
                name: "Microsoft".to_string(),
                market_cap_original: Some(1000000000000.0),
                original_currency: Some("USD".to_string()),
                market_cap_eur: Some(900000000000.0),
                market_cap_usd: Some(1000000000000.0),
            },
        ];

        let shares = calculate_market_shares(&records);

        // Total market cap: 3T
        // AAPL share: 2T / 3T = 66.67%
        // MSFT share: 1T / 3T = 33.33%
        assert!((shares.get("AAPL").unwrap() - 66.666666).abs() < 0.01);
        assert!((shares.get("MSFT").unwrap() - 33.333333).abs() < 0.01);
    }
}
