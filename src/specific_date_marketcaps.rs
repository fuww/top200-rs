// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::api;
use crate::config;
use crate::currencies::{convert_currency_with_rate, get_rate_map_from_db_for_date};
use anyhow::Result;
use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime};
use csv::Writer;
use indicatif::{ProgressBar, ProgressStyle};
use sqlx::sqlite::SqlitePool;
use std::sync::Arc;

/// Format a conversion rate for display (6 decimal places, or empty if not available)
fn format_rate(rate: Option<f64>) -> String {
    rate.map(|r| format!("{:.6}", r)).unwrap_or_default()
}

pub async fn fetch_specific_date_marketcaps(pool: &SqlitePool, date_str: &str) -> Result<()> {
    let config = config::load_config()?;
    let tickers = [config.non_us_tickers, config.us_tickers].concat();

    // Parse the date string
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("Invalid date format. Use YYYY-MM-DD: {}", e))?;

    let naive_dt = NaiveDateTime::new(date, NaiveTime::default());
    let datetime_utc = naive_dt.and_utc();
    let timestamp = naive_dt.and_utc().timestamp();

    // Get FMP client for market data
    let api_key = std::env::var("FINANCIALMODELINGPREP_API_KEY")
        .expect("FINANCIALMODELINGPREP_API_KEY must be set");
    let fmp_client = Arc::new(api::FMPClient::new(api_key));

    println!("Fetching market caps for date: {}", date);

    // Get exchange rates FOR THE SPECIFIC DATE (or closest date before it)
    println!("Fetching exchange rates for {} from database...", date);
    let rate_map = get_rate_map_from_db_for_date(pool, Some(timestamp)).await?;

    if rate_map.is_empty() {
        eprintln!(
            "⚠️  WARNING: No exchange rates found for date {} or earlier!",
            date
        );
        eprintln!("    Currency conversions will be inaccurate.");
        eprintln!("    Run 'ExportRates' command to fetch current rates first.");
    } else {
        println!("✅ Exchange rates fetched for {}", date);
    }

    let total_tickers = tickers.len();
    let progress = ProgressBar::new(total_tickers as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    let mut successful_tickers = Vec::new();
    let mut failed_tickers = Vec::new();

    for ticker in &tickers {
        progress.set_message(format!("Processing {}", ticker));

        match fmp_client
            .get_historical_market_cap(ticker, &datetime_utc)
            .await
        {
            Ok(market_cap) => {
                // Convert currencies with rate information
                let eur_result = convert_currency_with_rate(
                    market_cap.market_cap_original,
                    &market_cap.original_currency,
                    "EUR",
                    &rate_map,
                );

                let usd_result = convert_currency_with_rate(
                    market_cap.market_cap_original,
                    &market_cap.original_currency,
                    "USD",
                    &rate_map,
                );

                // Store the Unix timestamp of the historical date
                let timestamp = naive_dt.and_utc().timestamp();

                // Insert into database with conversion rates
                sqlx::query!(
                    r#"
                    INSERT OR REPLACE INTO market_caps (
                        ticker, name, market_cap_original, original_currency,
                        market_cap_eur, market_cap_usd, eur_rate, usd_rate,
                        exchange, price, active, timestamp
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                    ticker,
                    market_cap.name,
                    market_cap.market_cap_original,
                    market_cap.original_currency,
                    eur_result.amount,
                    usd_result.amount,
                    eur_result.rate,
                    usd_result.rate,
                    market_cap.exchange,
                    market_cap.price,
                    true,
                    timestamp,
                )
                .execute(pool)
                .await?;

                successful_tickers.push(ticker.clone());
            }
            Err(e) => {
                eprintln!(
                    "❌ Failed to fetch market cap for {} on {}: {}",
                    ticker, date, e
                );
                failed_tickers.push((ticker.clone(), e.to_string()));
            }
        }
        progress.inc(1);
    }
    progress.finish_with_message("Processing complete");

    // Print summary
    println!(
        "\n✅ Successfully fetched market caps for {} tickers",
        successful_tickers.len()
    );

    if !failed_tickers.is_empty() {
        println!("\n❌ Failed to fetch {} tickers:", failed_tickers.len());
        for (ticker, error) in &failed_tickers {
            println!("  {} - {}", ticker, error);
        }
    }

    // Export to CSV
    export_specific_date_marketcaps(pool, date).await?;

    Ok(())
}

async fn export_specific_date_marketcaps(pool: &SqlitePool, date: NaiveDate) -> Result<()> {
    let naive_dt = NaiveDateTime::new(date, NaiveTime::default());
    let timestamp = naive_dt.and_utc().timestamp();

    // Fetch market caps for the specific date
    let records = sqlx::query!(
        r#"
        SELECT
            m.ticker as "ticker!",
            m.name as "name!",
            CAST(m.market_cap_original AS REAL) as market_cap_original,
            m.original_currency,
            CAST(m.market_cap_eur AS REAL) as market_cap_eur,
            CAST(m.market_cap_usd AS REAL) as market_cap_usd,
            CAST(m.eur_rate AS REAL) as eur_rate,
            CAST(m.usd_rate AS REAL) as usd_rate,
            m.exchange,
            m.active,
            CAST(m.price AS REAL) as price,
            td.description,
            td.homepage_url,
            td.employees,
            td.ceo
        FROM market_caps m
        LEFT JOIN ticker_details td ON m.ticker = td.ticker
        WHERE m.timestamp = ?
        ORDER BY m.market_cap_eur DESC
        "#,
        timestamp
    )
    .fetch_all(pool)
    .await?;

    if records.is_empty() {
        println!("No market cap data found for date: {}", date);
        return Ok(());
    }

    // Create output directory if it doesn't exist
    std::fs::create_dir_all("output")?;

    // Generate filename with date
    let timestamp_str = Local::now().format("%Y%m%d_%H%M%S");
    let date_str = date.format("%Y-%m-%d");
    let filename = format!("output/marketcaps_{}_{}.csv", date_str, timestamp_str);

    let file = std::fs::File::create(&filename)?;
    let mut writer = Writer::from_writer(file);

    // Write headers
    writer.write_record(&[
        "Rank",
        "Ticker",
        "Name",
        "Market Cap (Original)",
        "Original Currency",
        "Market Cap (EUR)",
        "EUR Rate",
        "Market Cap (USD)",
        "USD Rate",
        "Price",
        "Exchange",
        "Active",
        "Description",
        "Homepage URL",
        "Employees",
        "CEO",
        "Date",
    ])?;

    // Write data with rank
    for (index, record) in records.iter().enumerate() {
        writer.write_record(&[
            (index + 1).to_string(),
            record.ticker.clone(),
            record.name.clone(),
            format!("{:.0}", record.market_cap_original.unwrap_or(0.0)),
            record.original_currency.clone().unwrap_or_default(),
            format!("{:.0}", record.market_cap_eur.unwrap_or(0.0)),
            format_rate(record.eur_rate),
            format!("{:.0}", record.market_cap_usd.unwrap_or(0.0)),
            format_rate(record.usd_rate),
            record.price.unwrap_or(0.0).to_string(),
            record.exchange.clone().unwrap_or_default(),
            if record.active.unwrap_or(true) {
                "true".to_string()
            } else {
                "false".to_string()
            },
            record.description.clone().unwrap_or_default(),
            record.homepage_url.clone().unwrap_or_default(),
            record.employees.map(|e| e.to_string()).unwrap_or_default(),
            record.ceo.clone().unwrap_or_default(),
            date_str.to_string(),
        ])?;
    }

    writer.flush()?;
    println!("✅ Market caps for {} exported to {}", date, filename);
    println!("   Total companies: {}", records.len());

    Ok(())
}
