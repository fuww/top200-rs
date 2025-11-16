// SPDX-FileCopyrightText: 2025 Joost van der Laan
// SPDX-License-Identifier: AGPL-3.0-only

use crate::api;
use crate::config;
use crate::currencies::get_rate_map_from_db;
use anyhow::Result;
use chrono::Local;
use csv::Writer;
use sqlx::sqlite::SqlitePool;
use std::path::PathBuf;

pub async fn export_details_eu_csv(pool: &SqlitePool) -> Result<()> {
    let config = config::load_config()?;
    let tickers = config.non_us_tickers;

    // Create output directory if it doesn't exist
    let output_dir = PathBuf::from("output");
    std::fs::create_dir_all(&output_dir)?;

    // Create CSV file with timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let csv_path = output_dir.join(format!("eu_marketcaps_{}.csv", timestamp));
    let mut writer = Writer::from_path(&csv_path)?;

    // Write header
    writer.write_record([
        "Ticker",
        "Company Name",
        "Market Cap",
        "Currency",
        "Exchange",
        "Price",
        "Active",
        "Description",
        "Homepage URL",
        "Employees",
        "Revenue",
        "Revenue (USD)",
        "Working Capital Ratio",
        "Quick Ratio",
        "EPS",
        "P/E Ratio",
        "D/E Ratio",
        "ROE",
    ])?;

    let rate_map = get_rate_map_from_db(pool).await?;

    let mut tasks = Vec::new();

    for ticker in tickers {
        let ticker = ticker.clone();
        let rate_map = rate_map.clone();
        tasks.push(tokio::spawn(async move {
            let details = api::get_details_eu(&ticker, &rate_map).await;
            (ticker, details)
        }));
    }

    for task in tasks {
        let (ticker, details) = task.await?;
        match details {
            Ok(details) => {
                writer.write_record([
                    &details.ticker,
                    &details.name.unwrap_or_default(),
                    &details
                        .market_cap
                        .map(|m| m.to_string())
                        .unwrap_or_default(),
                    &details.currency_symbol.unwrap_or_default(),
                    &details
                        .extra
                        .get("exchange")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    &details
                        .extra
                        .get("price")
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    &details.active.map(|a| a.to_string()).unwrap_or_default(),
                    &details.description.unwrap_or_default(),
                    &details.homepage_url.unwrap_or_default(),
                    &details.employees.unwrap_or_default(),
                    &details.revenue.map(|r| r.to_string()).unwrap_or_default(),
                    &details
                        .revenue_usd
                        .map(|r| r.to_string())
                        .unwrap_or_default(),
                    &details
                        .working_capital_ratio
                        .map(|r| r.to_string())
                        .unwrap_or_default(),
                    &details
                        .quick_ratio
                        .map(|r| r.to_string())
                        .unwrap_or_default(),
                    &details.eps.map(|r| r.to_string()).unwrap_or_default(),
                    &details.pe_ratio.map(|r| r.to_string()).unwrap_or_default(),
                    &details
                        .debt_equity_ratio
                        .map(|r| r.to_string())
                        .unwrap_or_default(),
                    &details.roe.map(|r| r.to_string()).unwrap_or_default(),
                ])?;
                println!("✅ Data written to CSV");
            }
            Err(e) => {
                eprintln!("Error fetching details for {}: {}", ticker, e);
                // Write empty row for failed ticker
                let error_msg = format!("Error: {}", e);
                writer.write_record([
                    &ticker, "", "", "", "", "", "", &error_msg, "", "", "", "", "", "", "", "", "",
                ])?;
            }
        }
    }

    writer.flush()?;
    println!("\n✅ CSV file created at: {}", csv_path.display());

    Ok(())
}

pub async fn list_details_eu(pool: &SqlitePool) -> Result<()> {
    let config = config::load_config()?;
    let tickers = config.non_us_tickers;
    let rate_map = get_rate_map_from_db(pool).await?;

    for (i, ticker) in tickers.iter().enumerate() {
        println!(
            "\nFetching the marketcap for {} ({}/{}) ⌛️",
            ticker,
            i + 1,
            tickers.len()
        );
        match api::get_details_eu(ticker, &rate_map).await {
            Ok(details) => {
                println!("Company: {}", details.name.unwrap_or_default());
                if let Some(market_cap) = details.market_cap {
                    println!(
                        "Market Cap: {} {}",
                        details.currency_symbol.unwrap_or_default(),
                        market_cap
                    );
                }
                println!("Active: {}", details.active.unwrap_or_default());
                println!("---");
            }
            Err(e) => eprintln!("Error fetching details for {}: {}", ticker, e),
        }
    }

    Ok(())
}
