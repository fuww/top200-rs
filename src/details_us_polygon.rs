// SPDX-FileCopyrightText: 2025 Joost van der Laan
// SPDX-License-Identifier: AGPL-3.0-only

use crate::api::PolygonClient;
use crate::config;
use anyhow::Result;
use chrono::{Local, NaiveDate};
use csv::Writer;
use sqlx::sqlite::SqlitePool;
use std::{env, path::PathBuf, sync::Arc};

pub async fn export_details_us_csv(_pool: &SqlitePool) -> Result<()> {
    let config = config::load_config()?;
    let tickers = config.us_tickers;
    let api_key = env::var("POLYGON_API_KEY").expect("POLYGON_API_KEY must be set");
    let client = Arc::new(PolygonClient::new(api_key));
    let date = NaiveDate::from_ymd_opt(2023, 11, 1).unwrap();

    // Create output directory if it doesn't exist
    let output_dir = PathBuf::from("output");
    std::fs::create_dir_all(&output_dir)?;

    // Create CSV file with timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let csv_path = output_dir.join(format!("us_marketcaps_{}.csv", timestamp));
    let mut writer = Writer::from_path(&csv_path)?;

    // Write header
    writer.write_record([
        "Ticker",
        "Company Name",
        "Market Cap",
        "Currency",
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

    for (i, ticker) in tickers.iter().enumerate() {
        println!(
            "\nFetching the marketcap for {} ({}/{}) ",
            ticker,
            i + 1,
            tickers.len()
        );
        match client.get_details(ticker, date).await {
            Ok(details) => {
                writer.write_record([
                    &details.ticker,
                    &details.name.unwrap_or_default(),
                    &details
                        .market_cap
                        .map(|m| m.to_string())
                        .unwrap_or_default(),
                    &details.currency_symbol.unwrap_or_default(),
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
                println!(" Data written to CSV");
            }
            Err(e) => {
                eprintln!("Error fetching details for {}: {}", ticker, e);
                // Write empty row for failed ticker
                let error_msg = format!("Error: {}", e);
                writer.write_record([
                    ticker, "", "", "", "", &error_msg, "", "", "", "", "", "", "", "", "", "",
                ])?;
            }
        }
    }

    writer.flush()?;
    println!("\n CSV file created at: {}", csv_path.display());

    Ok(())
}

pub async fn list_details_us(_pool: &SqlitePool) -> Result<()> {
    let config = config::load_config()?;
    let tickers = config.us_tickers;
    let api_key = env::var("POLYGON_API_KEY").expect("POLYGON_API_KEY must be set");
    let client = Arc::new(PolygonClient::new(api_key));
    let date = NaiveDate::from_ymd_opt(2023, 11, 1).unwrap();

    for (i, ticker) in tickers.iter().enumerate() {
        println!(
            "\nFetching the marketcap for {} ({}/{}) ",
            ticker,
            i + 1,
            tickers.len()
        );
        match client.get_details(ticker, date).await {
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
